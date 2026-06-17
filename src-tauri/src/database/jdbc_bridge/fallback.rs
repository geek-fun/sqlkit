use crate::database::config::DatabaseType;
use crate::database::error::{DbError, DbResult};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::download;
use super::error_classifier::{classify_connection_error, ErrorCategory};
use super::launcher::JdbcBridgeLauncher;
use super::protocol::{ConnectParams, JdbcMethod, JdbcRequest, ResolveDriverResult};
use super::registry::DatabaseDriverConfig;

/// Result of a single driver attempt.
pub enum DriverAttempt {
    /// Connection succeeded, with conn_id and the launcher.
    Connected(String, Arc<Mutex<JdbcBridgeLauncher>>),
    /// Version incompatibility — connection failed due to version mismatch.
    VersionMismatch(String),
    /// Fatal error — abort.
    Fatal(DbError),
}

/// Try connecting using the JDBC bridge.
///
/// Starts the bridge, resolves the driver JAR via Java-side `ResolveDriver` RPC,
/// then sends a `Connect` RPC with the resolved JAR on the classpath.
/// When `use_cap` is true and the config has a `version_cap`, it is sent
/// to the Java side as a version constraint.
pub async fn try_driver(
    config: &DatabaseDriverConfig,
    host: &str,
    port: u16,
    database: Option<&str>,
    username: &str,
    password: &Option<String>,
    use_cap: bool,
) -> DriverAttempt {
    let url = super::registry::build_jdbc_url(config, host, port, database);

    // Start bridge (no driver JARs yet — ResolveDriver will download on the Java side)
    let bridge_jar = download::bridge_jar_path();
    let mut launcher = JdbcBridgeLauncher::new(bridge_jar);
    match launcher.start() {
        Ok(_) => {}
        Err(e) => return DriverAttempt::Fatal(e),
    }
    let launcher = Arc::new(Mutex::new(launcher));

    // Step 1: ResolveDriver — let the Java bridge download / resolve the driver JAR
    // On the first attempt, try without version_cap (LATEST).
    // If the config has no cap, or we're explicitly using it, send the cap.
    let effective_cap = if use_cap { config.version_cap.as_deref() } else { None };
    let resolve_params = serde_json::json!({
        "maven_group": config.maven_group,
        "maven_artifact": config.maven_artifact,
        "version_cap": effective_cap,
        "maven_classifier": config.maven_classifier,
    });
    let resolve_req = JdbcRequest::new(JdbcMethod::ResolveDriver, resolve_params);

    let resolve_result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        async {
            let mut guard = launcher.lock().await;
            guard.send_request(&resolve_req)
        },
    )
    .await;

    let jar_path = match resolve_result {
        Ok(Ok(resp)) => {
            if let Some(ref err) = resp.error {
                let mut guard = launcher.lock().await;
                guard.shutdown();
                return DriverAttempt::Fatal(DbError::Connection(format!(
                    "Failed to resolve JDBC driver: {}",
                    err,
                )));
            }
            // Parse the ResolveDriverResult to get the resolved JAR path
            match resp.result.and_then(|v| serde_json::from_value::<ResolveDriverResult>(v).ok()) {
                Some(result) => result.jar_path,
                None => {
                    let mut guard = launcher.lock().await;
                    guard.shutdown();
                    return DriverAttempt::Fatal(DbError::Connection(
                        "Invalid ResolveDriver response: missing jar_path".to_string(),
                    ));
                }
            }
        }
        Ok(Err(e)) => {
            let mut guard = launcher.lock().await;
            guard.shutdown();
            return DriverAttempt::Fatal(e);
        }
        Err(_) => {
            let mut guard = launcher.lock().await;
            guard.shutdown();
            return DriverAttempt::Fatal(DbError::Timeout(
                "ResolveDriver timed out after 120s".to_string(),
            ));
        }
    };

    // Step 2: Send connect request with the resolved JAR on the classpath
    let params = match serde_json::to_value(ConnectParams {
        url,
        username: username.to_string(),
        password: password.clone(),
        database: database.map(|d| d.to_string()),
        driver_class: config.class_name.clone(),
        driver_jars: vec![jar_path],
        pool_min: 1,
        pool_max: 5,
    }) {
        Ok(v) => v,
        Err(e) => {
            return DriverAttempt::Fatal(DbError::Connection(format!(
                "Failed to serialize connect params: {}",
                e,
            )))
        }
    };

    let req = JdbcRequest::new(JdbcMethod::Connect, params);

    // 30s timeout per attempt
    let conn_result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut guard = launcher.lock().await;
        guard.send_request(&req)
    })
    .await;

    match conn_result {
        Ok(Ok(resp)) => {
            if let Some(ref err) = resp.error {
                let category = classify_connection_error(
                    db_type_from_config(config),
                    err,
                    &config.version_error_signatures,
                );
                match category {
                    ErrorCategory::VersionIncompatible => {
                        DriverAttempt::VersionMismatch(err.clone())
                    }
                    _ => DriverAttempt::Fatal(DbError::Connection(err.clone())),
                }
            } else {
                let conn_id = resp
                    .result
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| format!("conn_{}", uuid::Uuid::new_v4()));
                DriverAttempt::Connected(conn_id, launcher)
            }
        }
        Ok(Err(e)) => {
            let msg = e.to_string();
            let category = classify_connection_error(
                db_type_from_config(config),
                &msg,
                &config.version_error_signatures,
            );
            match category {
                ErrorCategory::VersionIncompatible => DriverAttempt::VersionMismatch(msg),
                ErrorCategory::Timeout => DriverAttempt::Fatal(DbError::Timeout(msg)),
                _ => DriverAttempt::Fatal(e),
            }
        }
        Err(_) => DriverAttempt::Fatal(DbError::Timeout(
            "Connection attempt timed out after 30s".to_string(),
        )),
    }
}

/// Connect using the configured JDBC driver.
///
/// Ensures JRE and bridge JAR are installed, then delegates to `try_driver`
/// which resolves the driver dynamically via Java-side `ResolveDriver` RPC.
pub async fn run_fallback_chain(
    db_type: DatabaseType,
    host: &str,
    port: u16,
    database: Option<&str>,
    username: &str,
    password: &Option<String>,
) -> DbResult<(String, String, Arc<Mutex<JdbcBridgeLauncher>>)> {
    let registry = super::registry::DriverRegistry::load();
    let config = registry.get_config(db_type).ok_or_else(|| {
        DbError::Connection(format!("No driver registry entry for {:?}", db_type))
    })?;

    // Ensure JRE is installed.
    if super::jre::JreDetector::detect().is_none() {
        super::jre::download_managed_jre().await?;
    } else {
        // Check for JRE update on connection if managed JRE is installed.
        if super::jre::is_managed_jre_installed() {
            if let Some(redirect_url) = super::jre::check_adoptium_update().await {
                if let Some(latest) = super::jre::parse_adoptium_build_version(&redirect_url) {
                    if let Some(current) = super::jre::read_jre_version() {
                        if compare_jre_versions(&latest, &current) > 0 {
                            super::jre::download_managed_jre().await?;
                        }
                    }
                }
            }
        }
    }
    if !download::is_bridge_installed() {
        download::download_bridge_plugin().await?;
    }

    // Two-phase driver resolution:
    // 1. Try LATEST (no version_cap)
    match try_driver(config, host, port, database, username, password, false).await {
        DriverAttempt::Connected(conn_id, launcher) => {
            return Ok(("resolved".to_string(), conn_id, launcher));
        }
        DriverAttempt::VersionMismatch(_) => {
            // 2. If LATEST fails with version_incompatible and a cap exists, retry with cap
            if config.version_cap.is_some() {
                match try_driver(config, host, port, database, username, password, true).await {
                    DriverAttempt::Connected(conn_id, launcher) => {
                        return Ok(("capped".to_string(), conn_id, launcher));
                    }
                    DriverAttempt::VersionMismatch(msg) => Err(DbError::Connection(format!(
                        "Could not find a compatible JDBC driver for this database version. \
                         Error: {}",
                        msg,
                    ))),
                    DriverAttempt::Fatal(e) => Err(e),
                }
            } else {
                Err(DbError::Connection(
                    "Could not find a compatible JDBC driver for this database version. \
                     Try updating the driver in Settings → JRE & Drivers."
                        .to_string(),
                ))
            }
        }
        DriverAttempt::Fatal(e) => Err(e),
    }
}

/// Compare two dotted JRE version strings numerically.
/// Returns positive if a > b, negative if a < b, 0 if equal.
fn compare_jre_versions(a: &str, b: &str) -> i32 {
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

fn db_type_from_config(config: &DatabaseDriverConfig) -> DatabaseType {
    match config.name.as_str() {
        "Oracle Database" => DatabaseType::Oracle,
        "IBM DB2" => DatabaseType::DB2,
        "H2 Database" => DatabaseType::H2,
        "Apache Derby" => DatabaseType::Derby,
        "Snowflake" => DatabaseType::Snowflake,
        "达梦 DM8" => DatabaseType::DM8Oracle,
        "虚谷 XuguDB" => DatabaseType::XuguDB,
        "GBase 8a" => DatabaseType::GBase8a,
        "Apache Hive" => DatabaseType::Hive,
        "Databricks SQL" => DatabaseType::Databricks,
        "SAP HANA" => DatabaseType::Hana,
        "Teradata" => DatabaseType::Teradata,
        "Vertica" => DatabaseType::Vertica,
        "Exasol" => DatabaseType::Exasol,
        "Google BigQuery" => DatabaseType::BigQuery,
        "IBM Informix" => DatabaseType::Informix,
        "Apache Kylin" => DatabaseType::Kylin,
        "Apache Cassandra" => DatabaseType::Cassandra,
        "InterSystems IRIS" => DatabaseType::Iris,
        "Microsoft Access" => DatabaseType::Access,
        _ => DatabaseType::PostgreSQL, // fallback, shouldn't happen
    }
}
