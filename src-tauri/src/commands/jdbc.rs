use serde::Serialize;
use crate::database::jdbc_bridge::{registry::DriverRegistry, jre, download};
use crate::database::config::DatabaseType;

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
    pub driver_count: usize,
    pub installed: bool,
}

#[tauri::command]
pub async fn check_jre_status() -> Result<JreStatus, String> {
    let managed_path = jre::managed_jre_java_path();
    if managed_path.exists() {
        Ok(JreStatus {
            installed: true,
            version: Some(jre::MANAGED_JRE_VERSION.to_string()),
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
    jre::download_managed_jre()
        .await
        .map_err(|e| e.to_string())
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
        let installed = config.versions.iter().any(|v| {
            let db_type = db_type_from_registry_key(key);
            db_type.map(|dt| download::is_driver_version_installed(dt, &v.version)).unwrap_or(false)
        });
        result.push(DriverInfo {
            db_type: key.clone(),
            name: config.name.clone(),
            driver_count: config.versions.len(),
            installed,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn download_driver(db_type: String) -> Result<(), String> {
    let dt = parse_db_type(&db_type).map_err(|e| e.to_string())?;
    let registry = DriverRegistry::load();
    let config = registry.get_config(dt).ok_or_else(|| format!("No registry entry for {}", db_type))?;
    if let Some(version) = config.versions.first() {
        let maven_group = version.maven_group_override.as_deref().unwrap_or(&config.maven_group);
        let maven_artifact = version.maven_artifact_override.as_deref().unwrap_or(&config.maven_artifact);
        let dest = download::driver_jar_path_for_version(dt, &version.version);
        download::download_driver_from_maven(maven_group, maven_artifact, &version.version, &dest, &version.jar_sha256)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_driver(db_type: String) -> Result<(), String> {
    let dt = parse_db_type(&db_type).map_err(|e| e.to_string())?;
    let registry = DriverRegistry::load();
    let config = registry.get_config(dt).ok_or_else(|| format!("No registry entry for {}", db_type))?;
    for version in &config.versions {
        let path = download::driver_jar_path_for_version(dt, &version.version);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("Failed to remove driver: {}", e))?;
        }
    }
    Ok(())
}

fn parse_db_type(s: &str) -> Result<DatabaseType, String> {
    match s.to_lowercase().as_str() {
        "oracle" => Ok(DatabaseType::Oracle),
        "db2" => Ok(DatabaseType::DB2),
        "h2" => Ok(DatabaseType::H2),
        "derby" => Ok(DatabaseType::Derby),
        "snowflake" => Ok(DatabaseType::Snowflake),
        "dm8_oracle" | "dm8oracle" => Ok(DatabaseType::DM8Oracle),
        "xugudb" | "xugu" => Ok(DatabaseType::XuguDB),
        "gbase8a" | "gbase_8a" => Ok(DatabaseType::GBase8a),
        _ => Err(format!("Unknown JDBC database type: {}", s)),
    }
}

fn db_type_from_registry_key(key: &str) -> Option<DatabaseType> {
    match key {
        "oracle" => Some(DatabaseType::Oracle),
        "db2" => Some(DatabaseType::DB2),
        "h2" => Some(DatabaseType::H2),
        "derby" => Some(DatabaseType::Derby),
        "snowflake" => Some(DatabaseType::Snowflake),
        "dm8_oracle" => Some(DatabaseType::DM8Oracle),
        "xugudb" => Some(DatabaseType::XuguDB),
        "gbase8a" => Some(DatabaseType::GBase8a),
        _ => None,
    }
}
