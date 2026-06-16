use crate::database::config::DatabaseType;
use crate::database::error::{DbError, DbResult};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::download;
use super::error_classifier::{classify_connection_error, ErrorCategory};
use super::launcher::JdbcBridgeLauncher;
use super::protocol::{ConnectParams, JdbcMethod, JdbcRequest};
use super::registry::{DatabaseDriverConfig, DriverRegistry, DriverVersion};

/// Result of trying a single driver version in the fallback chain.
pub enum DriverAttempt {
    /// Connection succeeded, with conn_id and the launcher.
    Connected(String, Arc<Mutex<JdbcBridgeLauncher>>),
    /// Version incompatibility — eligible for fallback.
    VersionMismatch(String),
    /// Fatal error — abort, do not fall back.
    Fatal(DbError),
}

/// Try connecting with a single driver version.
/// Returns the conn_id on success, or a categorized error for fallback decisions.
pub async fn try_driver(
    config: &DatabaseDriverConfig,
    version: &DriverVersion,
    host: &str,
    port: u16,
    database: Option<&str>,
    username: &str,
    password: &Option<String>,
) -> DriverAttempt {
    let url = super::registry::build_jdbc_url(config, host, port, database);
    let jar_path =
        download::driver_jar_path_for_version(db_type_from_config(config), &version.version);

    // Download driver if not cached
    if !jar_path.exists() {
        let maven_group = version
            .maven_group_override
            .as_deref()
            .unwrap_or(&config.maven_group);
        let maven_artifact = version
            .maven_artifact_override
            .as_deref()
            .unwrap_or(&config.maven_artifact);
        match download::download_driver_from_maven(
            maven_group,
            maven_artifact,
            &version.version,
            &jar_path,
            &version.jar_sha256,
            version.maven_classifier.as_deref(),
        )
        .await
        {
            Ok(_) => {}
            Err(e) => return DriverAttempt::Fatal(e),
        }
    }

    // Start launcher with this driver JAR on classpath
    let bridge_jar = download::bridge_jar_path();
    let mut launcher = JdbcBridgeLauncher::new(bridge_jar);
    match launcher.start_with_drivers(vec![jar_path]) {
        Ok(_) => {}
        Err(e) => return DriverAttempt::Fatal(e),
    }
    let launcher = Arc::new(Mutex::new(launcher));

    // Build combined error signatures
    let mut all_patterns = config.version_error_signatures.clone();
    all_patterns.extend(version.version_error_signatures.clone());

    // Send connect request
    let params = match serde_json::to_value(ConnectParams {
        url,
        username: username.to_string(),
        password: password.clone(),
        database: database.map(|d| d.to_string()),
        driver_class: config.class_name.clone(),
        driver_jars: vec![],
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
                let category =
                    classify_connection_error(db_type_from_config(config), err, &all_patterns);
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
            let category =
                classify_connection_error(db_type_from_config(config), &msg, &all_patterns);
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

/// Run the full fallback chain: try drivers in order until one succeeds
/// or we exhaust all options + fatal errors stop us.
pub async fn run_fallback_chain(
    db_type: DatabaseType,
    host: &str,
    port: u16,
    database: Option<&str>,
    username: &str,
    password: &Option<String>,
) -> DbResult<(String, String, Arc<Mutex<JdbcBridgeLauncher>>)> {
    let registry = DriverRegistry::load();
    let config = registry.get_config(db_type).ok_or_else(|| {
        DbError::Connection(format!("No driver registry entry for {:?}", db_type))
    })?;
    let chain = registry
        .get_driver_chain(db_type)
        .ok_or_else(|| DbError::Connection(format!("No driver chain for {:?}", db_type)))?;

    // Ensure JRE and bridge JAR are installed.
    // Only download managed JRE if no Java is available on the system
    // (managed JRE → JAVA_HOME → PATH).
    if super::jre::JreDetector::detect().is_none() {
        super::jre::download_managed_jre().await?;
    }
    if !download::is_bridge_installed() {
        download::download_bridge_plugin().await?;
    }

    let mut last_version_error: Option<String> = None;

    for version in chain.iter() {
        match try_driver(config, version, host, port, database, username, password).await {
            DriverAttempt::Connected(conn_id, launcher) => {
                return Ok((version.version.clone(), conn_id, launcher));
            }
            DriverAttempt::VersionMismatch(msg) => {
                last_version_error = Some(msg);
                // Continue to next driver in chain
            }
            DriverAttempt::Fatal(e) => {
                return Err(e);
            }
        }
    }

    Err(DbError::Connection(format!(
        "Could not find a compatible JDBC driver for this database version. \
         Attempted {} driver(s). Last error: {}",
        chain.len(),
        last_version_error.as_deref().unwrap_or("unknown"),
    )))
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
