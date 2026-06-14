//! Java bridge subprocess lifecycle management.
//!
//! Spawns a Java process and communicates with it via newline-delimited
//! JSON over stdin/stdout.

use crate::database::error::{DbError, DbResult};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};

use super::protocol::{JdbcRequest, JdbcResponse};

/// Manages the Java bridge subprocess.
pub struct JdbcBridgeLauncher {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    jar_path: PathBuf,
}

impl JdbcBridgeLauncher {
    /// Create a new launcher that will use the given JAR path.
    pub fn new(jar_path: PathBuf) -> Self {
        Self {
            process: None,
            stdin: None,
            jar_path,
        }
    }

    /// Get the Java executable path, preferring the bundled JRE.
    pub fn detect_java() -> Option<PathBuf> {
        // Bundled JRE takes priority
        let bundled = super::download::jre_java_path();
        if bundled.exists() {
            return Some(bundled);
        }
        None
    }

    /// Start the Java bridge process.
    pub fn start(&mut self) -> DbResult<()> {
        let java = Self::detect_java().ok_or_else(|| {
            DbError::Connection(
                "Bundled JRE not found. Call download_jre() first to install it."
                    .to_string(),
            )
        })?;

        if !self.jar_path.exists() {
            return Err(DbError::Connection(format!(
                "JDBC bridge JAR not found at {}. Run download_bridge_plugin() first.",
                self.jar_path.display()
            )));
        }

        let mut child = Command::new(&java)
            .args(["-jar", self.jar_path.to_str().unwrap_or("")])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| DbError::Connection(format!("Failed to start JDBC bridge: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| DbError::Connection("Failed to capture bridge stdin".to_string()))?;

        self.process = Some(child);
        self.stdin = Some(stdin);

        // Wait briefly and check if the process is still alive
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Err(DbError::Connection(format!(
                        "JDBC bridge exited immediately with code: {}",
                        status
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
        let process = self.process.as_mut().ok_or_else(|| {
            DbError::Connection("JDBC bridge not started".to_string())
        })?;

        let stdout = process.stdout.as_mut().ok_or_else(|| {
            DbError::Connection("JDBC bridge stdout not available".to_string())
        })?;

        let stdin = self.stdin.as_mut().ok_or_else(|| {
            DbError::Connection("JDBC bridge stdin not available".to_string())
        })?;

        // Serialize and write request line
        let json = serde_json::to_string(req)
            .map_err(|e| DbError::Connection(format!("Failed to serialize request: {}", e)))?;

        writeln!(stdin, "{}", json)
            .map_err(|e| DbError::Connection(format!("Failed to write to bridge stdin: {}", e)))?;
        stdin
            .flush()
            .map_err(|e| DbError::Connection(format!("Failed to flush bridge stdin: {}", e)))?;

        // Read response line
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| DbError::Connection(format!("Failed to read bridge response: {}", e)))?;

        if line.trim().is_empty() {
            return Err(DbError::Connection(
                "Empty response from JDBC bridge".to_string(),
            ));
        }

        let resp: JdbcResponse = serde_json::from_str(line.trim())
            .map_err(|e| DbError::Connection(format!("Failed to parse bridge response: {}", e)))?;

        if let Some(ref err) = resp.error {
            return Err(DbError::Connection(format!("JDBC bridge error: {}", err)));
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
    }
}

impl Drop for JdbcBridgeLauncher {
    fn drop(&mut self) {
        self.shutdown();
    }
}
