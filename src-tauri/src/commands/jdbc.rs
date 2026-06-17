use crate::database::config::DatabaseType;
use crate::database::jdbc_bridge::{download, jre, launcher::JdbcBridgeLauncher, protocol::{JdbcMethod, JdbcRequest}, registry::DriverRegistry};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct JreStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: String,
}

#[derive(Serialize)]
pub struct DriverInfo {
    pub db_type: String,
    pub name: String,
    pub installed: bool,
    pub version_cap: Option<String>,
    pub filename: Option<String>,
    pub file_size: Option<u64>,
    pub resolved_version: Option<String>,
}

#[tauri::command]
pub async fn check_jre_status() -> Result<JreStatus, String> {
    let managed_path = jre::managed_jre_java_path();
    if managed_path.exists() {
        Ok(JreStatus {
            installed: true,
            version: jre::read_jre_version().or_else(|| Some("21".to_string())),
            path: Some(managed_path.to_string_lossy().to_string()),
            source: "managed".to_string(),
        })
    } else if let Some(system_java) = jre::JreDetector::detect_system_java() {
        Ok(JreStatus {
            installed: true,
            version: Some("system".to_string()),
            path: Some(system_java.to_string_lossy().to_string()),
            source: "system".to_string(),
        })
    } else {
        Ok(JreStatus {
            installed: false,
            version: None,
            path: None,
            source: "none".to_string(),
        })
    }
}

#[tauri::command]
pub async fn download_jre() -> Result<(), String> {
    jre::download_managed_jre().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_jre() -> Result<(), String> {
    jre::remove_managed_jre().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_drivers() -> Result<Vec<DriverInfo>, String> {
    let registry = DriverRegistry::load();
    let mut result = Vec::new();
    for (key, config) in &registry.databases {
        let cached = driver_cache_info(&config.maven_artifact);
        result.push(DriverInfo {
            db_type: key.clone(),
            name: config.name.clone(),
            installed: cached.is_some(),
            version_cap: config.version_cap.clone(),
            filename: cached.as_ref().map(|c| c.0.clone()),
            file_size: cached.as_ref().map(|c| c.1),
            resolved_version: cached.as_ref().and_then(|c| parse_jar_version(&c.0)),
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn download_driver(db_type: String) -> Result<(), String> {
    let dt = parse_db_type(&db_type).map_err(|e| e.to_string())?;
    let registry = DriverRegistry::load();
    let config = registry
        .get_config(dt)
        .ok_or_else(|| format!("No registry entry for {}", db_type))?;

    // Start a temporary Java bridge process to resolve the driver
    let bridge_jar = download::bridge_jar_path();
    let mut launcher = JdbcBridgeLauncher::new(bridge_jar);
    launcher.start().map_err(|e| e.to_string())?;

    let req = JdbcRequest::new(
        JdbcMethod::ResolveDriver,
        serde_json::json!({
            "maven_group": config.maven_group,
            "maven_artifact": config.maven_artifact,
            "version_cap": config.version_cap,
            "maven_classifier": config.maven_classifier,
        }),
    );
    let resp = launcher.send_request(&req).map_err(|e| e.to_string())?;
    if let Some(err) = resp.error {
        launcher.shutdown();
        return Err(err);
    }

    // Driver is now cached on disk by the Java bridge
    launcher.shutdown();
    Ok(())
}

#[tauri::command]
pub async fn remove_driver(db_type: String) -> Result<(), String> {
    let dt = parse_db_type(&db_type).map_err(|e| e.to_string())?;
    let registry = DriverRegistry::load();
    let config = registry
        .get_config(dt)
        .ok_or_else(|| format!("No registry entry for {}", db_type))?;
    let artifact_dir = drivers_cache_dir().join(&config.maven_artifact);
    if artifact_dir.exists() {
        std::fs::remove_dir_all(&artifact_dir)
            .map_err(|e| format!("Failed to remove driver: {}", e))?;
    }
    Ok(())
}

fn parse_db_type(s: &str) -> Result<DatabaseType, String> {
    match s.to_lowercase().as_str() {
        "oracle" => Ok(DatabaseType::Oracle),
        "duckdb" | "duck" => Ok(DatabaseType::DuckDb),
        "firebird" => Ok(DatabaseType::Firebird),
        "db2" => Ok(DatabaseType::DB2),
        "h2" => Ok(DatabaseType::H2),
        "derby" => Ok(DatabaseType::Derby),
        "snowflake" => Ok(DatabaseType::Snowflake),
        "dm8_oracle" | "dm8oracle" => Ok(DatabaseType::DM8Oracle),
        "xugudb" | "xugu" => Ok(DatabaseType::XuguDB),
        "gbase8a" | "gbase_8a" => Ok(DatabaseType::GBase8a),
        "hive" => Ok(DatabaseType::Hive),
        "databricks" => Ok(DatabaseType::Databricks),
        "hana" => Ok(DatabaseType::Hana),
        "teradata" => Ok(DatabaseType::Teradata),
        "vertica" => Ok(DatabaseType::Vertica),
        "exasol" => Ok(DatabaseType::Exasol),
        "bigquery" => Ok(DatabaseType::BigQuery),
        "informix" => Ok(DatabaseType::Informix),
        "kylin" => Ok(DatabaseType::Kylin),
        "cassandra" => Ok(DatabaseType::Cassandra),
        "iris" => Ok(DatabaseType::Iris),
        "access" => Ok(DatabaseType::Access),
        _ => Err(format!("Unknown JDBC database type: {}", s)),
    }
}

/// Get info about a cached driver: filename and file size.
/// Returns `None` if no JAR is cached for the given artifact.
fn driver_cache_info(artifact: &str) -> Option<(String, u64)> {
    let dir = drivers_cache_dir().join(artifact);
    if !dir.exists() {
        return None;
    }
    std::fs::read_dir(&dir).ok()?.flatten().find_map(|e| {
        let path = e.path();
        if path.extension() == Some(std::ffi::OsStr::new("jar")) {
            let size = std::fs::metadata(&path).ok()?.len();
            let name = path.file_name()?.to_str()?.to_string();
            Some((name, size))
        } else {
            None
        }
    })
}

/// Parse a version from a JAR filename.
/// e.g. "h2-2.2.224.jar" -> "2.2.224", "ojdbc11-21.15.0.0.jar" -> "21.15.0.0"
fn parse_jar_version(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".jar")?;
    // Find the first digit-segment after a hyphen
    let dash_idx = stem.rfind('-')?;
    let version_part = &stem[dash_idx + 1..];
    if version_part.is_empty() || !version_part.chars().next()?.is_ascii_digit() {
        return None;
    }
    Some(version_part.to_string())
}

/// Check whether a driver JAR is already cached on disk for the given artifact name.
/// Looks in `~/.sqlkit/jdbc-bridge/drivers/{artifact}/` for any `.jar` file.
fn is_driver_cached(artifact: &str) -> bool {
    let dir = drivers_cache_dir().join(artifact);
    if !dir.exists() {
        return false;
    }
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.path().extension() == Some(std::ffi::OsStr::new("jar"))
            })
        })
        .unwrap_or(false)
}

/// Get the path to the JDBC driver cache directory (`~/.sqlkit/jdbc-bridge/drivers/`).
fn drivers_cache_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".sqlkit")
        .join("jdbc-bridge")
        .join("drivers")
}

#[derive(Serialize)]
pub struct BridgeStatus {
    pub installed: bool,
    pub current_version: String,
    pub path: Option<String>,
}

#[tauri::command]
pub async fn check_bridge_status() -> Result<BridgeStatus, String> {
    let jar_path = download::bridge_jar_path();
    Ok(BridgeStatus {
        installed: jar_path.exists(),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        path: Some(jar_path.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub async fn download_bridge_jar() -> Result<(), String> {
    download::download_bridge_plugin()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_bridge_jar() -> Result<(), String> {
    let jar_path = download::bridge_jar_path();
    if jar_path.exists() {
        if let Some(parent) = jar_path.parent() {
            std::fs::remove_dir_all(parent)
                .map_err(|e| format!("Failed to remove bridge JAR: {}", e))?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
pub struct JreUpdateStatus {
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

#[tauri::command]
pub async fn check_jre_update() -> Result<JreUpdateStatus, String> {
    let current = jre::read_jre_version();
    let redirect_url = jre::check_adoptium_update().await;
    let latest = redirect_url
        .as_ref()
        .and_then(|url| jre::parse_adoptium_build_version(url));
    let update_available = match (&current, &latest) {
        (Some(c), Some(l)) => compare_versions(l, c) > 0,
        (None, Some(_)) => true,
        _ => false,
    };
    Ok(JreUpdateStatus {
        current_version: current,
        latest_version: latest,
        update_available,
    })
}

#[tauri::command]
pub async fn get_jdbc_needed() -> Result<bool, String> {
    Ok(!jdbc_not_needed_path().exists())
}

#[tauri::command]
pub async fn set_jdbc_needed(needed: bool) -> Result<(), String> {
    let path = jdbc_not_needed_path();
    if needed {
        let _ = std::fs::remove_file(&path);
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        std::fs::write(&path, "")
            .map_err(|e| format!("Failed to write JDBC preference: {}", e))?;
    }
    Ok(())
}

/// Compare two dotted version strings numerically (e.g. "21.0.11" > "9.0.0").
/// Returns negative if a < b, positive if a > b, 0 if equal.
fn compare_versions(a: &str, b: &str) -> i32 {
    let parts_a: Vec<&str> = a.split('.').collect();
    let parts_b: Vec<&str> = b.split('.').collect();
    let max_len = parts_a.len().max(parts_b.len());
    for i in 0..max_len {
        let na: u32 = parts_a.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        let nb: u32 = parts_b.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        if na != nb {
            return if na > nb { 1 } else { -1 };
        }
    }
    0
}

fn jdbc_not_needed_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".sqlkit").join(".jdbc_not_needed")
}
