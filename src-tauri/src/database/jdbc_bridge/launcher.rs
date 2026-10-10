//! Java bridge subprocess lifecycle management.
//!
//! Spawns a Java process and communicates with it via newline-delimited
//! JSON over stdin/stdout.

use crate::database::error::{DbError, DbResult};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use super::protocol::{JdbcRequest, JdbcResponse};

/// Manages the Java bridge subprocess.
pub struct JdbcBridgeLauncher {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    jar_path: PathBuf,
    /// Buffered stderr lines from the Java bridge, drained by a background
    /// reader thread to prevent the OS pipe buffer from filling up.
    stderr_buffer: Option<Arc<Mutex<Vec<String>>>>,
}

impl JdbcBridgeLauncher {
    /// Create a new launcher that will use the given JAR path.
    pub fn new(jar_path: PathBuf) -> Self {
        Self {
            process: None,
            stdin: None,
            jar_path,
            stderr_buffer: None,
        }
    }

    /// Get the Java executable path, preferring the managed JRE, then system.
    pub fn detect_java() -> Option<PathBuf> {
        super::jre::JreDetector::detect()
    }

    /// JVM options granting native access to classpath code.
    ///
    /// Drivers like DuckDB load a native library through `System::load`, which
    /// Java 24+ flags as a restricted method and future JDKs will deny without
    /// `--enable-native-access` (issue #176). The option exists since JDK 17;
    /// older JVMs reject it at startup, so it is only passed when the detected
    /// Java is new enough (or when the version can't be determined — every
    /// supported launch path already requires Java 25+).
    fn native_access_args(java_version: Option<u32>) -> Vec<String> {
        match java_version {
            Some(version) if version < 17 => Vec::new(),
            _ => vec!["--enable-native-access=ALL-UNNAMED".to_string()],
        }
    }

    fn read_stderr_buffer(buf: &Arc<Mutex<Vec<String>>>) -> String {
        buf.lock().unwrap_or_else(|e| e.into_inner()).join("\n")
    }

    fn drain_stderr(&self) -> String {
        self.stderr_buffer
            .as_ref()
            .map(|buf| Self::read_stderr_buffer(buf))
            .unwrap_or_default()
    }

    pub fn stderr_snapshot(&self) -> String {
        self.drain_stderr()
    }

    fn spawn_stderr_reader(stderr: std::process::ChildStderr) -> Arc<Mutex<Vec<String>>> {
        let buf: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let buf_clone = buf.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok() && !line.is_empty() {
                let trimmed = line.trim().to_string();
                if !trimmed.is_empty() {
                    if let Ok(mut b) = buf_clone.lock() {
                        b.push(trimmed);
                        if b.len() > 200 {
                            b.remove(0);
                        }
                    }
                }
                line.clear();
            }
        });
        buf
    }

    pub fn start(&mut self, jvm_args: &[String]) -> DbResult<()> {
        let java = Self::detect_java().ok_or_else(|| {
            DbError::Connection(
                "Java not found. Install a JRE or call download_jre() to use the bundled JRE."
                    .to_string(),
            )
        })?;

        let java_version = super::jre::system_java_version(&java);
        let mut cmd = Command::new(&java);
        for arg in Self::native_access_args(java_version) {
            cmd.arg(arg);
        }
        for arg in jvm_args {
            cmd.arg(arg);
        }
        cmd.arg("-jar").arg(self.jar_path.to_str().unwrap_or(""));
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| DbError::Connection(format!("Failed to start JDBC bridge: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| DbError::Connection("Failed to capture bridge stdin".to_string()))?;

        let stderr = child.stderr.take().expect("stderr was piped");
        self.stderr_buffer = Some(Self::spawn_stderr_reader(stderr));
        self.process = Some(child);
        self.stdin = Some(stdin);

        // Wait briefly and check if the process is still alive
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Err(DbError::Connection(format!(
                        "JDBC bridge exited immediately with code: {}. stderr: {}",
                        status,
                        self.drain_stderr()
                    )));
                }
                Ok(None) => { /* still running, good */ }
                Err(e) => {
                    return Err(DbError::Connection(format!(
                        "Error checking bridge process: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    /// Start the Java bridge process with additional driver JARs on the classpath.
    pub fn start_with_drivers(&mut self, driver_jars: Vec<PathBuf>) -> DbResult<()> {
        let java = Self::detect_java().ok_or_else(|| {
            DbError::Connection(
                "Java not found. Install a JRE or call download_managed_jre() to use the bundled JRE."
                    .to_string(),
            )
        })?;

        // Build classpath: bridge JAR + driver JARs
        let mut classpath = self.jar_path.to_string_lossy().to_string();
        for jar in &driver_jars {
            if jar.exists() {
                classpath.push_str(&format!(":{}", jar.display()));
            }
        }

        let java_version = super::jre::system_java_version(&java);
        let mut cmd = Command::new(&java);
        for arg in Self::native_access_args(java_version) {
            cmd.arg(arg);
        }
        cmd.args(["-cp", &classpath, "sqlkit.bridge.BridgeMain"]);
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| DbError::Connection(format!("Failed to start JDBC bridge: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| DbError::Connection("Failed to capture bridge stdin".to_string()))?;

        let stderr = child.stderr.take().expect("stderr was piped");
        self.stderr_buffer = Some(Self::spawn_stderr_reader(stderr));
        self.process = Some(child);
        self.stdin = Some(stdin);

        // Wait briefly and check if the process is still alive
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Err(DbError::Connection(format!(
                        "JDBC bridge exited immediately with code: {}. stderr: {}",
                        status,
                        self.drain_stderr()
                    )));
                }
                Ok(None) => { /* still running, good */ }
                Err(e) => {
                    return Err(DbError::Connection(format!(
                        "Error checking bridge process: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    /// Send a request and receive a response.
    pub fn send_request(&mut self, req: &JdbcRequest) -> DbResult<JdbcResponse> {
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| DbError::Connection("JDBC bridge not started".to_string()))?;

        // Check if the process is still alive before trying to communicate
        if let Ok(Some(status)) = process.try_wait() {
            let stderr = self
                .stderr_buffer
                .as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            return if stderr.is_empty() {
                Err(DbError::Connection(format!(
                    "JDBC bridge exited before request (code: {}). No stderr output.",
                    status
                )))
            } else {
                Err(DbError::Connection(format!(
                    "JDBC bridge exited before request (code: {}). stderr: {}",
                    status, stderr
                )))
            };
        }

        let stdout = process
            .stdout
            .as_mut()
            .ok_or_else(|| DbError::Connection("JDBC bridge stdout not available".to_string()))?;

        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| DbError::Connection("JDBC bridge stdin not available".to_string()))?;

        let json = serde_json::to_string(req)
            .map_err(|e| DbError::Connection(format!("Failed to serialize request: {}", e)))?;

        writeln!(stdin, "{}", json).map_err(|e| {
            let stderr = self
                .stderr_buffer
                .as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            if stderr.is_empty() {
                DbError::Connection(format!("Failed to write to bridge stdin: {}", e))
            } else {
                DbError::Connection(format!("Bridge write error: {}. stderr: {}", e, stderr))
            }
        })?;
        stdin.flush().map_err(|e| {
            let stderr = self
                .stderr_buffer
                .as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            if stderr.is_empty() {
                DbError::Connection(format!("Failed to flush bridge stdin: {}", e))
            } else {
                DbError::Connection(format!("Bridge write error: {}. stderr: {}", e, stderr))
            }
        })?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut read_attempts = 0;

        // Skip any non-JSON lines (e.g. JVM prints version info to stdout)
        // Retry once if first read is empty (JVM may be slow to start)
        loop {
            line.clear();
            reader.read_line(&mut line).map_err(|e| {
                let stderr = self
                    .stderr_buffer
                    .as_ref()
                    .map(Self::read_stderr_buffer)
                    .unwrap_or_default();
                if stderr.is_empty() {
                    DbError::Connection(format!("Failed to read bridge response: {}", e))
                } else {
                    DbError::Connection(format!("Bridge read error: {}. stderr: {}", e, stderr))
                }
            })?;

            let trimmed = line.trim();
            if trimmed.is_empty() {
                if read_attempts == 0 {
                    // JVM may be slow to start — wait and retry once
                    read_attempts += 1;
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    continue;
                }
                let stderr = self
                    .stderr_buffer
                    .as_ref()
                    .map(Self::read_stderr_buffer)
                    .unwrap_or_default();
                return if stderr.is_empty() {
                    Err(DbError::Connection(
                        "Empty response from JDBC bridge".to_string(),
                    ))
                } else {
                    Err(DbError::Connection(format!(
                        "Bridge read error. stderr: {}",
                        stderr
                    )))
                };
            }
            // Skip lines that don't start with '{' (non-JSON noise from JVM)
            if trimmed.starts_with('{') {
                break;
            }
        }

        let resp: JdbcResponse = serde_json::from_str(line.trim())
            .map_err(|e| DbError::Connection(format!("Failed to parse bridge response: {}", e)))?;

        if let Some(ref err) = resp.error {
            let error_type = resp.error_type.as_deref().unwrap_or("unknown");
            return Err(match error_type {
                "version_incompatible" => DbError::DriverVersionIncompatible(err.clone()),
                "authentication_failed" => DbError::Authentication(err.clone()),
                "network_error" | "timeout" => DbError::Connection(err.clone()),
                _ => DbError::Connection(format!("JDBC bridge error: {}", err)),
            });
        }

        Ok(resp)
    }

    /// Send a request and receive a response, emitting progress events during the read phase.
    ///
    /// Same as [`send_request`] but intercepts intermediate JSON lines containing
    /// `"phase":"progress"` emitted by the Java bridge during long operations
    /// (e.g. `resolve_driver`).  Such lines are parsed and the `downloaded`/`total`
    /// fields are forwarded to `progress_cb`.  The read loop continues until the
    /// actual JSON-RPC response arrives.
    pub fn send_request_with_progress(
        &mut self,
        req: &JdbcRequest,
        mut progress_cb: impl FnMut(u64, u64),
    ) -> DbResult<JdbcResponse> {
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| DbError::Connection("JDBC bridge not started".to_string()))?;

        // Check if the process is still alive before trying to communicate
        if let Ok(Some(status)) = process.try_wait() {
            let stderr = self
                .stderr_buffer
                .as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            return if stderr.is_empty() {
                Err(DbError::Connection(format!(
                    "JDBC bridge exited before request (code: {}). No stderr output.",
                    status
                )))
            } else {
                Err(DbError::Connection(format!(
                    "JDBC bridge exited before request (code: {}). stderr: {}",
                    status, stderr
                )))
            };
        }

        let stdout = process
            .stdout
            .as_mut()
            .ok_or_else(|| DbError::Connection("JDBC bridge stdout not available".to_string()))?;

        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| DbError::Connection("JDBC bridge stdin not available".to_string()))?;

        let json = serde_json::to_string(req)
            .map_err(|e| DbError::Connection(format!("Failed to serialize request: {}", e)))?;

        writeln!(stdin, "{}", json).map_err(|e| {
            let stderr = self
                .stderr_buffer
                .as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            if stderr.is_empty() {
                DbError::Connection(format!("Failed to write to bridge stdin: {}", e))
            } else {
                DbError::Connection(format!("Bridge write error: {}. stderr: {}", e, stderr))
            }
        })?;
        stdin.flush().map_err(|e| {
            let stderr = self
                .stderr_buffer
                .as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            if stderr.is_empty() {
                DbError::Connection(format!("Failed to flush bridge stdin: {}", e))
            } else {
                DbError::Connection(format!("Bridge write error: {}. stderr: {}", e, stderr))
            }
        })?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut read_attempts = 0;

        // Skip any non-JSON lines (e.g. JVM prints version info to stdout).
        // Intercept intermediate progress events from the bridge.
        // Retry once if first read is empty (JVM may be slow to start).
        loop {
            line.clear();
            reader.read_line(&mut line).map_err(|e| {
                let stderr = self
                    .stderr_buffer
                    .as_ref()
                    .map(Self::read_stderr_buffer)
                    .unwrap_or_default();
                if stderr.is_empty() {
                    DbError::Connection(format!("Failed to read bridge response: {}", e))
                } else {
                    DbError::Connection(format!("Bridge read error: {}. stderr: {}", e, stderr))
                }
            })?;

            let trimmed = line.trim();
            if trimmed.is_empty() {
                if read_attempts == 0 {
                    // JVM may be slow to start — wait and retry once
                    read_attempts += 1;
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    continue;
                }
                let stderr = self
                    .stderr_buffer
                    .as_ref()
                    .map(Self::read_stderr_buffer)
                    .unwrap_or_default();
                return if stderr.is_empty() {
                    Err(DbError::Connection(
                        "Empty response from JDBC bridge".to_string(),
                    ))
                } else {
                    Err(DbError::Connection(format!(
                        "Bridge read error. stderr: {}",
                        stderr
                    )))
                };
            }
            // Skip lines that don't start with '{' (non-JSON noise from JVM)
            if !trimmed.starts_with('{') {
                continue;
            }
            // Intercept progress events: {"phase":"progress","downloaded":N,"total":M}
            if trimmed.contains("\"phase\":\"progress\"") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    let downloaded = val.get("downloaded").and_then(|v| v.as_u64()).unwrap_or(0);
                    let total = val.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                    progress_cb(downloaded, total);
                }
                continue;
            }
            // This is the actual JSON-RPC response
            break;
        }

        let resp: JdbcResponse = serde_json::from_str(line.trim())
            .map_err(|e| DbError::Connection(format!("Failed to parse bridge response: {}", e)))?;

        if let Some(ref err) = resp.error {
            let error_type = resp.error_type.as_deref().unwrap_or("unknown");
            return Err(match error_type {
                "version_incompatible" => DbError::DriverVersionIncompatible(err.clone()),
                "authentication_failed" => DbError::Authentication(err.clone()),
                "network_error" | "timeout" => DbError::Connection(err.clone()),
                _ => DbError::Connection(format!("JDBC bridge error: {}", err)),
            });
        }

        Ok(resp)
    }

    /// Check if the bridge process is still alive.
    pub fn is_alive(&mut self) -> bool {
        match self.process.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => false,
                _ => true,
            },
            None => false,
        }
    }

    /// Shutdown the bridge process gracefully.
    ///
    /// Only belongs to the shared registry owner: connections must release their
    /// own pool through the `disconnect` RPC (`JdbcMethod::Disconnect`) instead,
    /// because other connections may be served by the same process.
    pub fn shutdown(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stdin = None;
        self.stderr_buffer = None;
    }
}

/// Bridge processes shared across connections, keyed by JAR + JVM options.
type LauncherRegistry = Mutex<HashMap<String, Arc<tokio::sync::Mutex<JdbcBridgeLauncher>>>>;

fn launcher_registry() -> &'static LauncherRegistry {
    static REGISTRY: OnceLock<LauncherRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Key of a reusable bridge process configuration.
///
/// JVM options are part of the key because they cannot change once the process
/// is running (Oracle wallet/keystore arguments, for example).
fn launcher_key(bridge_jar: &Path, jvm_args: &[String]) -> String {
    format!("{}::{}", bridge_jar.display(), jvm_args.join(" "))
}

/// Get the bridge JVM shared by every connection with the same JAR and options,
/// starting one when none is alive.
///
/// Starting a JVM is cheap, but the first JDBC connection inside it is not:
/// DuckDB's driver maps a ~107 MB native library (measured ~1.4 s), and the
/// driver resolution also reaches Maven Central. Sharing the process pays both
/// costs once per app session instead of once per connection or "Test".
pub fn shared_launcher(
    bridge_jar: &Path,
    jvm_args: &[String],
) -> DbResult<Arc<tokio::sync::Mutex<JdbcBridgeLauncher>>> {
    let key = launcher_key(bridge_jar, jvm_args);
    let mut registry = launcher_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(existing) = registry.get(&key) {
        let alive = match existing.try_lock() {
            Ok(mut guard) => guard.is_alive(),
            // Busy means another request is in flight, so the process is alive.
            Err(_) => true,
        };
        if alive {
            return Ok(existing.clone());
        }
    }

    let mut launcher = JdbcBridgeLauncher::new(bridge_jar.to_path_buf());
    launcher.start(jvm_args)?;
    let shared = Arc::new(tokio::sync::Mutex::new(launcher));
    registry.insert(key, shared.clone());
    Ok(shared)
}

#[cfg(test)]
mod launcher_registry_tests {
    use super::*;

    #[test]
    fn launcher_key_separates_jars_and_jvm_options() {
        let jar = PathBuf::from("/tmp/jdbc-bridge-0.8.8.jar");
        assert_eq!(launcher_key(&jar, &[]), launcher_key(&jar, &[]),);
        assert_ne!(
            launcher_key(&jar, &[]),
            launcher_key(&jar, &["-Dfoo=bar".to_string()]),
        );
        assert_ne!(
            launcher_key(&jar, &[]),
            launcher_key(Path::new("/tmp/other.jar"), &[]),
        );
    }

    #[test]
    fn native_access_args_on_for_modern_java() {
        for version in [17u32, 21, 25] {
            assert_eq!(
                JdbcBridgeLauncher::native_access_args(Some(version)),
                vec!["--enable-native-access=ALL-UNNAMED".to_string()],
                "Java {} should get native access",
                version,
            );
        }
    }

    #[test]
    fn native_access_args_off_for_legacy_java() {
        for version in [8u32, 11] {
            assert!(
                JdbcBridgeLauncher::native_access_args(Some(version)).is_empty(),
                "Java {} would reject the option at startup",
                version,
            );
        }
    }

    #[test]
    fn native_access_args_on_when_version_unknown() {
        // All supported launch paths require Java 25+, so an undetectable
        // version still passes the flag — failing closed keeps DuckDB's
        // native library load working on future JDKs (issue #176).
        assert_eq!(
            JdbcBridgeLauncher::native_access_args(None),
            vec!["--enable-native-access=ALL-UNNAMED".to_string()],
        );
    }
}

impl Drop for JdbcBridgeLauncher {
    fn drop(&mut self) {
        self.shutdown();
    }
}
