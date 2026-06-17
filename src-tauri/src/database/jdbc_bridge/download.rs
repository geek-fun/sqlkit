//! JDBC bridge download management.
//!
//! Downloads the bridge fat JAR from GitHub Releases, version-pinned
//! by the app version. JDBC driver JARs are resolved by the Java
//! bridge process, not by Rust.

use crate::database::error::{DbError, DbResult};
use std::path::{Path, PathBuf};

/// Bridge JAR filename.
const BRIDGE_JAR: &str = "jdbc-bridge.jar";

/// Current app version, used for bridge JAR version pinning.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Subdirectory under user home for bridge data.
const BRIDGE_DIR: &str = ".sqlkit/jdbc-bridge";

/// Get the bridge data directory (~/.sqlkit/jdbc-bridge).
fn bridge_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(BRIDGE_DIR)
}

/// Get the path to the bridge JAR (version-pinned).
pub fn bridge_jar_path() -> PathBuf {
    bridge_dir().join(APP_VERSION).join(BRIDGE_JAR)
}

/// Check if the bridge JAR is already installed.
pub fn is_bridge_installed() -> bool {
    bridge_jar_path().exists()
}

/// Download a file from URL to a temporary path, then atomically rename to final.
pub async fn download_to_path(url: &str, dest: &Path) -> DbResult<()> {
    let tmp_path = dest.with_extension("tmp");
    let response = reqwest::get(url)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to download from {}: {}", url, e)))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| DbError::Connection(format!("Failed to read download: {}", e)))?;
    // Ensure parent dir exists
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to create dir: {}", e)))?;
    }
    tokio::fs::write(&tmp_path, &bytes)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to write temp file: {}", e)))?;
    tokio::fs::rename(&tmp_path, dest)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to finalize download: {}", e)))?;
    Ok(())
}

/// Download the bridge fat JAR from GitHub Releases (version-pinned).
pub async fn download_bridge_plugin() -> DbResult<()> {
    let jar_path = bridge_jar_path();

    // Ensure parent dir exists
    if let Some(parent) = jar_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to create bridge dir: {}", e)))?;
    }

    let url = format!(
        "https://github.com/geek-fun/sqlkit/releases/download/v{}/jdbc-bridge.jar",
        APP_VERSION
    );
    download_to_path(&url, &jar_path).await?;

    // Clean up old bridge versions after successful download
    cleanup_old_bridge_versions().await?;

    Ok(())
}

/// Clean up bridge JAR directories for versions other than the current one.
async fn cleanup_old_bridge_versions() -> DbResult<()> {
    let dir = bridge_dir();
    if !dir.exists() {
        return Ok(());
    }
    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to list bridge dir: {}", e)))?;
    let mut remove_tasks = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| DbError::Connection(format!("Failed to read bridge entry: {}", e)))?
    {
        let path = entry.path();
        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name != APP_VERSION && dir_name != "drivers" {
                    remove_tasks.push(tokio::fs::remove_dir_all(path));
                }
            }
        }
    }
    for task in remove_tasks {
        task.await
            .map_err(|e| DbError::Connection(format!("Failed to remove old bridge version: {}", e)))?;
    }
    Ok(())
}

/// List all installed bridge JAR versions on disk.
pub fn list_bridge_versions() -> Vec<String> {
    let dir = bridge_dir();
    if !dir.exists() {
        return Vec::new();
    }
    let mut versions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if name != "drivers" {
                        let jar = entry.path().join(BRIDGE_JAR);
                        if jar.exists() {
                            versions.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    versions.sort();
    versions
}
