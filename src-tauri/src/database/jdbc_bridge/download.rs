//! JDBC bridge download management.
//!
//! Downloads the bridge fat JAR from GitHub Releases, version-pinned
//! by the app version. JARs are stored flat in `~/.sqlkit/jdbc-bridge/`
//! with versioned filenames (`jdbc-bridge-{version}.jar`).
//! JDBC driver JARs can be downloaded directly from Maven Central
//! (fallback if the Java bridge resolution is unavailable).

use crate::database::error::{DbError, DbResult};
use crate::APP_HANDLE;
use futures::StreamExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Emitter;

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
/// Emits Tauri progress events if the global APP_HANDLE is set.
pub async fn download_to_path(url: &str, dest: &Path, event_label: &str, expected_size_hint: u64) -> DbResult<()> {
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

    let total = expected_size_hint.max(1);
    let mut downloaded: u64 = 0;

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to create dir: {}", e)))?;
    }

    // Stream chunks and write to temp file
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to create temp file: {}", e)))?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| DbError::Connection(format!("Download stream error: {}", e)))?;
        downloaded += chunk.len() as u64;
        use tokio::io::AsyncWriteExt;
        file.write_all(&chunk)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to write chunk: {}", e)))?;

        // Emit progress event
        if let Some(handle) = crate::APP_HANDLE.get() {
            let _ = handle.emit(
                "connection-progress",
                serde_json::json!({
                    "step": event_label,
                    "downloaded": downloaded,
                    "total": total,
                }),
            );
        }
    }

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
    for attempt in 0..2 {
        if attempt > 0 {
            // Emit retry event
            if let Some(handle) = APP_HANDLE.get() {
                let _ = handle.emit(
                    "connection-progress",
                    serde_json::json!({
                        "step": "retry",
                        "message": format!("Download failed, retrying... ({})", last_err.as_deref().unwrap_or("unknown error")),
                        "downloaded": 0,
                        "total": 1,
                    }),
                );
            }
        }
        if let Err(e) = download_to_path(&url, &jar_path, "bridge_jar", 10_000_000).await {
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
            // Try to validate the JAR. If Java isn't actually runnable (e.g. macOS stub),
            // skip validation rather than deleting the downloaded JAR.
            match Command::new(&java)
                .args(["-jar", &jar_path.to_string_lossy(), "--version"])
                .output()
            {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if stderr.contains("Unable to locate") || stderr.contains("no java") {
                            // macOS stub or missing JRE — skip validation, JAR is fine
                        } else {
                            let _ = std::fs::remove_file(&jar_path);
                            last_err = Some(format!(
                                "Bridge JAR validation failed (exit: {}): {}",
                                output.status, stderr
                            ));
                            continue;
                        }
                    } else {
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
                }
                Err(_) => {
                    // Can't run Java at all — skip validation, JAR is fine
                }
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

/// Download a JDBC driver JAR directly from Maven Central via HTTP.
/// Uses the driver registry config (version_cap, maven coordinates) to
/// construct the download URL. Stores the JAR in the driver cache directory.
/// Does NOT require Java — purely HTTP.
pub async fn download_jdbc_driver_direct(db_type: &str) -> DbResult<()> {
    use super::registry::{resolve_maven_url, DriverRegistry};
    use crate::database::config::DatabaseType;

    let dt = match db_type {
        "oracle" => DatabaseType::Oracle,
        "db2" => DatabaseType::DB2,
        "h2" => DatabaseType::H2,
        _ => return Err(DbError::Connection(format!("No direct JDBC driver download for {}", db_type))),
    };

    let registry = DriverRegistry::load();
    let config = registry.get_config(dt).ok_or_else(|| {
        DbError::Connection(format!("No driver registry entry for {}", db_type))
    })?;

    let version = config.version_cap.as_deref().unwrap_or("latest");
    let classifier = config.maven_classifier.as_deref();
    let url = resolve_maven_url(&config.maven_group, &config.maven_artifact, version, classifier);

    let dest_dir = super::jre::home_dir()
        .join(".sqlkit")
        .join("jdbc-bridge")
        .join("drivers")
        .join(&config.maven_artifact);
    let jar_name = format!("{}-{}.jar", config.maven_artifact, version);
    let dest = dest_dir.join(&jar_name);

    if dest.exists() {
        return Ok(()); // Already cached
    }

    download_to_path(&url, &dest, "jdbc_driver", 5_000_000).await
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


