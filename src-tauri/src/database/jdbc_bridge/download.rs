//! JDBC bridge and driver download management.
//!
//! Downloads the bridge fat JAR and per-database JDBC driver JARs from
//! GitHub Releases on demand.

use crate::database::config::DatabaseType;
use crate::database::error::{DbError, DbResult};
use std::path::PathBuf;

/// Bridge JAR filename.
const BRIDGE_JAR: &str = "jdbc-bridge.jar";

/// Download URL base for bridge releases.
const BRIDGE_RELEASE_URL: &str =
    "https://github.com/geek-fun/sqlkit/releases/latest/download";

/// Subdirectory under user home for bridge data.
const BRIDGE_DIR: &str = ".sqlkit/jdbc-bridge";

/// Get the bridge data directory (~/.sqlkit/jdbc-bridge).
fn bridge_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(BRIDGE_DIR)
}

/// Get the drivers directory.
fn drivers_dir() -> PathBuf {
    bridge_dir().join("drivers")
}

/// Get the path to the bridge JAR.
pub fn bridge_jar_path() -> PathBuf {
    bridge_dir().join(BRIDGE_JAR)
}

/// Check if the bridge JAR is already installed.
pub fn is_bridge_installed() -> bool {
    bridge_jar_path().exists()
}

/// Check if a JDBC driver is available for the given database type.
pub fn is_driver_available(db_type: DatabaseType) -> bool {
    let name = driver_jar_name(db_type);
    drivers_dir().join(name).exists()
}

/// Ensure the bridge JAR and required driver are installed.
pub fn ensure_bridge_setup(db_type: DatabaseType) -> DbResult<()> {
    let bridge = bridge_jar_path();
    if !bridge.exists() {
        return Err(DbError::Connection(format!(
            "JDBC bridge not installed. Run download_bridge_plugin() first. \
             Expected JAR at: {}",
            bridge.display()
        )));
    }
    if !is_driver_available(db_type) {
        return Err(DbError::Connection(format!(
            "JDBC driver for {:?} not available. Run download_driver({:?}) first.",
            db_type, db_type
        )));
    }
    Ok(())
}

/// Download the bridge fat JAR from GitHub Releases.
pub async fn download_bridge_plugin() -> DbResult<()> {
    let dir = bridge_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to create bridge dir: {}", e)))?;

    let url = format!("{}/{}", BRIDGE_RELEASE_URL, BRIDGE_JAR);
    let response = reqwest::get(&url)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to download bridge: {}", e)))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| DbError::Connection(format!("Failed to read bridge download: {}", e)))?;

    let path = bridge_jar_path();
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to write bridge JAR: {}", e)))?;

    Ok(())
}

/// Download a JDBC driver JAR for the given database type.
pub async fn download_driver(db_type: DatabaseType) -> DbResult<()> {
    let dir = drivers_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to create drivers dir: {}", e)))?;

    let jar_name = driver_jar_name(db_type);
    let url = format!("{}/drivers/{}", BRIDGE_RELEASE_URL, jar_name);

    let response = reqwest::get(&url)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to download driver: {}", e)))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| DbError::Connection(format!("Failed to read driver download: {}", e)))?;

    let path = drivers_dir().join(&jar_name);
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to write driver JAR: {}", e)))?;

    Ok(())
}

/// Map a DatabaseType to a JDBC driver JAR filename.
fn driver_jar_name(db_type: DatabaseType) -> &'static str {
    use DatabaseType::*;
    match db_type {
        DB2 => "db2-jdbc.jar",
        H2 => "h2-2.4.240.jar",
        Snowflake => "snowflake-jdbc.jar",
        DM8Oracle => "dm-jdbc.jar",
        XuguDB => "xugudb-jdbc.jar",
        GBase8a => "gbase8a-jdbc.jar",
        _ => "unknown.jar",
    }
}

/// Get the full path to a driver JAR for the given database type.
pub fn driver_jar_path(db_type: DatabaseType) -> PathBuf {
    drivers_dir().join(driver_jar_name(db_type))
}

/// Map a DatabaseType to a JDBC driver class name.
pub fn driver_class(db_type: DatabaseType) -> &'static str {
    use DatabaseType::*;
    match db_type {
        DB2 => "com.ibm.db2.jcc.DB2Driver",
        H2 => "org.h2.Driver",
        Snowflake => "net.snowflake.client.jdbc.SnowflakeDriver",
        DM8Oracle => "dm.jdbc.driver.DmDriver",
        XuguDB => "com.xugudb.jdbc.Driver",
        GBase8a => "com.gbase.jdbc.Driver",
        _ => "",
    }
}

/// Build a JDBC URL from connection config.
pub fn build_jdbc_url(db_type: DatabaseType, host: &str, port: u16, database: Option<&str>) -> String {
    use DatabaseType::*;
    let db = database.unwrap_or("");
    match db_type {
        DB2 => format!("jdbc:db2://{}:{}/{}", host, port, db),
        H2 => {
            if db.is_empty() {
                format!("jdbc:h2:tcp://{}:{}/~/.sqlkit/h2/{}", host, port, host)
            } else {
                format!("jdbc:h2:tcp://{}:{}/{}", host, port, db)
            }
        }
        Snowflake => format!(
            "jdbc:snowflake://{}.snowflakecomputing.com/?warehouse={}&db={}",
            host, db, db
        ),
        DM8Oracle => format!("jdbc:dm://{}:{}", host, port),
        XuguDB => format!("jdbc:xugudb://{}:{}/{}", host, port, db),
        GBase8a => format!("jdbc:gbase://{}:{}/{}", host, port, db),
        _ => format!("jdbc:unknown://{}:{}/{}", host, port, db),
    }
}
