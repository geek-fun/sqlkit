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
            resolved_version: cached
                .as_ref()
                .and_then(|c| parse_jar_version(&config.maven_artifact, config.maven_classifier.as_deref(), &c.0)),
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
/// Handles Maven classifiers: e.g. "hive-jdbc-3.1.3-standalone.jar" -> "3.1.3"
fn parse_jar_version(artifact: &str, classifier: Option<&str>, filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".jar")?;
    let after_artifact = stem.strip_prefix(&format!("{}-", artifact))?;
    let version_str = if let Some(cls) = classifier {
        after_artifact.strip_suffix(&format!("-{}", cls))?
    } else {
        after_artifact
    };
    if version_str.is_empty() || !version_str.chars().next()?.is_ascii_digit() {
        return None;
    }
    Some(version_str.to_string())
}

/// Get the path to the JDBC driver cache directory (`~/.sqlkit/jdbc-bridge/drivers/`).
fn drivers_cache_dir() -> PathBuf {
    jre::home_dir()
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
    let current_version = if jar_path.exists() {
        jar_path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("jdbc-bridge-"))
            .unwrap_or(env!("CARGO_PKG_VERSION"))
            .to_string()
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };
    Ok(BridgeStatus {
        installed: jar_path.exists(),
        current_version,
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
        std::fs::remove_file(&jar_path)
            .map_err(|e| format!("Failed to remove bridge JAR: {}", e))?;
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
        (Some(c), Some(l)) => jre::compare_versions(l, c) > 0,
        (None, Some(_)) => true,
        _ => false,
    };
    Ok(JreUpdateStatus {
        current_version: current,
        latest_version: latest,
        update_available,
    })
}
