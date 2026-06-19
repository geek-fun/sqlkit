//! Java bridge subprocess lifecycle management.
//!
//! Spawns a Java process and communicates with it via newline-delimited
//! JSON over stdin/stdout.

use crate::database::error::{DbError, DbResult};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
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

    fn read_stderr_buffer(buf: &Arc<Mutex<Vec<String>>>) -> String {
        buf.lock()
            .unwrap_or_else(|e| e.into_inner())
            .join("\n")
    }

    fn drain_stderr(&self) -> String {
        self.stderr_buffer
            .as_ref()
            .map(|buf| Self::read_stderr_buffer(buf))
            .unwrap_or_default()
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

    pub fn start(&mut self) -> DbResult<()> {
        let java = Self::detect_java().ok_or_else(|| {
            DbError::Connection(
                "Java not found. Install a JRE or call download_jre() to use the bundled JRE."
                    .to_string(),
            )
        })?;

        let mut child = Command::new(&java)
            .args(["-jar", self.jar_path.to_str().unwrap_or("")])
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

        let mut child = Command::new(&java)
            .args(["-cp", &classpath, "sqlkit.bridge.BridgeMain"])
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
            let stderr = self.stderr_buffer.as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            if stderr.is_empty() {
                DbError::Connection(format!("Failed to write to bridge stdin: {}", e))
            } else {
                DbError::Connection(format!(
                    "Bridge write error: {}. stderr: {}",
                    e, stderr
                ))
            }
        })?;
        stdin.flush().map_err(|e| {
            let stderr = self.stderr_buffer.as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            if stderr.is_empty() {
                DbError::Connection(format!("Failed to flush bridge stdin: {}", e))
            } else {
                DbError::Connection(format!(
                    "Bridge write error: {}. stderr: {}",
                    e, stderr
                ))
            }
        })?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| {
            let stderr = self.stderr_buffer.as_ref()
                .map(Self::read_stderr_buffer)
                .unwrap_or_default();
            if stderr.is_empty() {
                DbError::Connection(format!("Failed to read bridge response: {}", e))
            } else {
                DbError::Connection(format!(
                    "Bridge read error: {}. stderr: {}",
                    e, stderr
                ))
            }
        })?;

        if line.trim().is_empty() {
            let stderr = self.stderr_buffer.as_ref()
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
    pub fn shutdown(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stdin = None;
        self.stderr_buffer = None;
    }
}

impl Drop for JdbcBridgeLauncher {
    fn drop(&mut self) {
        self.shutdown();
    }
}
