use crate::database::config::{DatabaseType, OracleConnectionOptions};
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

/// Build an Oracle JDBC URL based on the connection method and options.
/// Supports: basic SID, basic service name, TNS alias, and cloud wallet formats.
fn build_oracle_url(
    config: &DatabaseDriverConfig,
    host: &str,
    port: u16,
    database: Option<&str>,
    oracle_options: Option<&OracleConnectionOptions>,
) -> String {
    let opts = match oracle_options {
        Some(o) => o,
        // No Oracle options — fall back to default SID format
        None => return super::registry::build_jdbc_url(config, host, port, database),
    };

    match opts.connection_method.as_str() {
        "basic" => {
            let use_service = matches!(opts.sid_or_service.as_deref(), Some("service_name"));
            if use_service {
                // Service name format: jdbc:oracle:thin:@//host:port/service_name
                if let Some(ref service_template) = config.jdbc_url_template_service {
                    super::registry::build_jdbc_url_from_template(service_template, host, port, database)
                } else {
                    super::registry::build_jdbc_url(config, host, port, database)
                }
            } else {
                // SID format: jdbc:oracle:thin:@host:port:sid (default template)
                super::registry::build_jdbc_url(config, host, port, database)
            }
        }
        "tns" | "cloud_wallet" => {
            let alias = opts.tns_alias.as_deref().unwrap_or("");
            if let Some(ref admin_dir) = opts.tns_admin_dir {
                if let Some(descriptor) = super::tns_parser::lookup_tns_descriptor(admin_dir, alias)
                {
                    return format!("jdbc:oracle:thin:@{}", descriptor);
                }
            }
            format!("jdbc:oracle:thin:@{}", alias)
        }
        _ => super::registry::build_jdbc_url(config, host, port, database),
    }
}

fn build_jvm_args(oracle_options: Option<&OracleConnectionOptions>) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(opts) = oracle_options {
        if let Some(ref admin_dir) = opts.tns_admin_dir {
            if let Some(ref pwd) = opts.wallet_password {
                if !pwd.is_empty() {
                    args.push(format!("-Djavax.net.ssl.keyStore={}/keystore.jks", admin_dir));
                    args.push(format!("-Djavax.net.ssl.keyStorePassword={}", pwd));
                    args.push("-Djavax.net.ssl.keyStoreType=JKS".to_string());
                }
            }
        }
    }
    args
}

fn stderr_from_launcher(launcher: &Arc<Mutex<JdbcBridgeLauncher>>) -> String {
    match launcher.try_lock() {
        Ok(guard) => guard.stderr_snapshot(),
        Err(_) => String::new(),
    }
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
    oracle_options: Option<&OracleConnectionOptions>,
    ssl_mode: Option<&str>,
    ssl_ca_cert: Option<&str>,
    ssl_client_cert: Option<&str>,
    ssl_client_key: Option<&str>,
    trust_server_certificate: bool,
) -> DriverAttempt {
    let url = build_oracle_url(config, host, port, database, oracle_options);

    // Start bridge (no driver JARs yet — ResolveDriver will download on the Java side)
    let bridge_jar = download::bridge_jar_path();
    let mut launcher = JdbcBridgeLauncher::new(bridge_jar);
    let jvm_args = build_jvm_args(oracle_options);
    match launcher.start(&jvm_args) {
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
        "download_url": config.download_url,
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
        oracle_options: oracle_options.cloned(),
        credentials_in_url: config.credentials_in_url,
        ssl_mode: ssl_mode.map(|s| s.to_string()),
        ssl_ca_cert: ssl_ca_cert.map(|s| s.to_string()),
        ssl_client_cert: ssl_client_cert.map(|s| s.to_string()),
        ssl_client_key: ssl_client_key.map(|s| s.to_string()),
        trust_server_certificate,
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
                    _ => {
                        let stderr = stderr_from_launcher(&launcher);
                        let detail = if stderr.is_empty() { err.clone() } else { format!("{}. stderr: {}", err, stderr) };
                        DriverAttempt::Fatal(DbError::Connection(detail))
                    }
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
            let stderr = stderr_from_launcher(&launcher);
            let msg = if stderr.is_empty() { e.to_string() } else { format!("{}. stderr: {}", e, stderr) };
            let category = classify_connection_error(
                db_type_from_config(config),
                &msg,
                &config.version_error_signatures,
            );
            match category {
                ErrorCategory::VersionIncompatible => DriverAttempt::VersionMismatch(msg),
                ErrorCategory::Timeout => DriverAttempt::Fatal(DbError::Timeout(msg)),
                _ => DriverAttempt::Fatal(DbError::Connection(msg)),
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
/// Supports Oracle-specific URL construction via `oracle_options`.
pub async fn run_fallback_chain(
    db_type: DatabaseType,
    host: &str,
    port: u16,
    database: Option<&str>,
    username: &str,
    password: &Option<String>,
    oracle_options: Option<&OracleConnectionOptions>,
    ssl_mode: Option<&str>,
    ssl_ca_cert: Option<&str>,
    ssl_client_cert: Option<&str>,
    ssl_client_key: Option<&str>,
    trust_server_certificate: bool,
) -> DbResult<(String, String, Arc<Mutex<JdbcBridgeLauncher>>)> {
    let registry = super::registry::DriverRegistry::load();
    let config = registry.get_config(db_type).ok_or_else(|| {
        DbError::Connection(format!("No driver registry entry for {:?}", db_type))
    })?;

    if !super::jre::is_managed_jre_installed() {
        if let Err(download_err) = super::jre::download_managed_jre().await {
            match super::jre::JreDetector::detect_system_java() {
                Some(system_path) => match super::jre::system_java_version(&system_path) {
                    Some(version) if version >= 25 => {}
                    Some(version) => {
                        return Err(DbError::Connection(format!(
                            "System Java version is {} but Java 25+ is required. \
                             SQLKit could not download a managed JRE: {}. \
                             Install Java 25 manually or retry with internet access.",
                            version, download_err
                        )));
                    }
                    None => {
                        return Err(DbError::Connection(format!(
                            "Could not determine system Java version. \
                             SQLKit could not download a managed JRE: {}. \
                             Install Java 25 manually or retry with internet access.",
                            download_err
                        )));
                    }
                },
                None => {
                    return Err(DbError::Connection(format!(
                        "No Java 25+ found. SQLKit could not download a managed JRE: {}. \
                         Check your internet connection or install Java 25 manually.",
                        download_err
                    )));
                }
            }
        }
    } else {
        if let Some(redirect_url) = super::jre::check_adoptium_update().await {
            if let Some(latest) = super::jre::parse_adoptium_build_version(&redirect_url) {
                if let Some(current) = super::jre::read_jre_version() {
                    if super::jre::compare_versions(&latest, &current) > 0 {
                        super::jre::download_managed_jre().await?;
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
    match try_driver(config, host, port, database, username, password, false, oracle_options, ssl_mode, ssl_ca_cert, ssl_client_cert, ssl_client_key, trust_server_certificate).await {
        DriverAttempt::Connected(conn_id, launcher) => {
            return Ok(("resolved".to_string(), conn_id, launcher));
        }
        DriverAttempt::VersionMismatch(_) => {
            // 2. If LATEST fails with version_incompatible and a cap exists, retry with cap
            if config.version_cap.is_some() {
                match try_driver(config, host, port, database, username, password, true, oracle_options, ssl_mode, ssl_ca_cert, ssl_client_cert, ssl_client_key, trust_server_certificate).await {
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

fn db_type_from_config(config: &DatabaseDriverConfig) -> DatabaseType {
    match config.name.as_str() {
        "Oracle Database" => DatabaseType::Oracle,
        "IBM DB2" => DatabaseType::DB2,
        "H2 Database" => DatabaseType::H2,
        "Apache Derby" => DatabaseType::Derby,
        "Snowflake" => DatabaseType::Snowflake,
        "达梦 Dameng" => DatabaseType::Dameng,
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
