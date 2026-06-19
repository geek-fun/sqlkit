//! JDBC bridge download management.
//!
//! Downloads the bridge fat JAR from GitHub Releases, version-pinned
//! by the app version. JARs are stored flat in `~/.sqlkit/jdbc-bridge/`
//! with versioned filenames (`jdbc-bridge-{version}.jar`).
//! JDBC driver JARs are resolved by the Java bridge process, not by Rust.

use crate::database::error::{DbError, DbResult};
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_VERSION: &str = env!("APP_VERSION");

/// Subdirectory under user home for bridge data.
const BRIDGE_DIR: &str = ".sqlkit/jdbc-bridge";

/// Bridge JAR filename pattern prefix.
const BRIDGE_JAR_PREFIX: &str = "jdbc-bridge-";

/// Bridge JAR filename pattern suffix.
const BRIDGE_JAR_SUFFIX: &str = ".jar";

/// Get the bridge data directory (~/.sqlkit/jdbc-bridge).
fn bridge_dir() -> PathBuf {
    super::jre::home_dir().join(BRIDGE_DIR)
}

/// Get the path to the current version's bridge JAR (`~/.sqlkit/jdbc-bridge/jdbc-bridge-{ver}.jar`).
pub fn bridge_jar_path() -> PathBuf {
    bridge_dir().join(format!("{}{}{}", BRIDGE_JAR_PREFIX, APP_VERSION, BRIDGE_JAR_SUFFIX))
}

/// Check if the current version's bridge JAR is installed.
pub fn is_bridge_installed() -> bool {
    bridge_jar_path().exists()
}

/// Download a file from URL to a temporary path, then atomically rename to final.
pub async fn download_to_path(url: &str, dest: &Path) -> DbResult<()> {
    let tmp_path = dest.with_extension("tmp");
    let response = reqwest::get(url)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to download from {}: {}", url, e)))?;
    if !response.status().is_success() {
        return Err(DbError::Connection(format!(
            "Download failed: HTTP {} from {}",
            response.status(),
            url
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| DbError::Connection(format!("Failed to read download: {}", e)))?;
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
///
/// Validates the downloaded JAR by checking:
/// 1. HTTP 200 response status
/// 2. File size ≥ 1 MB (a fat JAR with dependencies)
/// 3. `java -jar jdbc-bridge.jar --version` exits 0 (if a JRE is available)
///
/// Retries once on validation failure.
pub async fn download_bridge_plugin() -> DbResult<()> {
    let jar_path = bridge_jar_path();

    if let Some(parent) = jar_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to create bridge dir: {}", e)))?;
    }

    let url = format!(
        "https://github.com/geek-fun/sqlkit/releases/download/v{}/jdbc-bridge-{}.jar",
        APP_VERSION, APP_VERSION
    );

    let mut last_err = None::<String>;
    for _attempt in 0..2 {
        if let Err(e) = download_to_path(&url, &jar_path).await {
            last_err = Some(e.to_string());
            continue;
        }

        let meta = std::fs::metadata(&jar_path).map_err(|e| {
            DbError::Connection(format!("Failed to check JAR file size: {}", e))
        })?;
        if meta.len() < 1_000_000 {
            let _ = std::fs::remove_file(&jar_path);
            last_err = Some(format!(
                "JAR file too small ({} bytes, expected ≥ 1 MB)",
                meta.len()
            ));
            continue;
        }

        if let Some(java) = super::jre::JreDetector::detect() {
            let output = Command::new(&java)
                .args(["-jar", &jar_path.to_string_lossy(), "--version"])
                .output()
                .map_err(|e| {
                    DbError::Connection(format!("Failed to run JAR validation: {}", e))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let _ = std::fs::remove_file(&jar_path);
                last_err = Some(format!(
                    "Bridge JAR validation failed (exit: {}): {}",
                    output.status, stderr
                ));
                continue;
            }

            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stdout.is_empty() || stdout == "unknown" {
                let _ = std::fs::remove_file(&jar_path);
                last_err = Some(format!(
                    "Bridge JAR --version returned invalid output: {}",
                    stdout
                ));
                continue;
            }
        }

        last_err = None;
        break;
    }

    if let Some(err) = last_err {
        let tmp_path = jar_path.with_extension("tmp");
        let _ = std::fs::remove_file(&tmp_path);
        return Err(DbError::Connection(format!(
            "Failed to download valid bridge JAR after retry: {}",
            err
        )));
    }

    cleanup_old_bridge_versions().await?;

    Ok(())
}

/// Clean up old bridge JARs and stale version directories.
/// Removes `jdbc-bridge-*.jar` files for versions other than current,
/// and any leftover `{version}/` directories from the old folder layout.
async fn cleanup_old_bridge_versions() -> DbResult<()> {
    let dir = bridge_dir();
    if !dir.exists() {
        return Ok(());
    }
    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to list bridge dir: {}", e)))?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| DbError::Connection(format!("Failed to read bridge entry: {}", e)))?
    {
        let path = entry.path();
        let fname = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Remove stale version subdirectories from old folder layout
        if path.is_dir() && fname != "drivers" {
            let old_jar = path.join("jdbc-bridge.jar");
            if old_jar.exists() {
                tokio::fs::remove_dir_all(&path)
                    .await
                    .map_err(|e| DbError::Connection(format!("Failed to remove old bridge dir: {}", e)))?;
            }
            continue;
        }

        // Remove old flat jdbc-bridge-*.jar files (not current version)
        if fname.starts_with(BRIDGE_JAR_PREFIX) && fname.ends_with(BRIDGE_JAR_SUFFIX) {
            let ver = &fname[BRIDGE_JAR_PREFIX.len()..fname.len() - BRIDGE_JAR_SUFFIX.len()];
            if ver != APP_VERSION {
                tokio::fs::remove_file(&path)
                    .await
                    .map_err(|e| DbError::Connection(format!("Failed to remove old bridge JAR: {}", e)))?;
            }
        }
    }
    Ok(())
}


