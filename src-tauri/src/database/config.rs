//! Configuration structures for database connections.
//!
//! This module defines configuration structures for establishing database connections
//! with support for various database types and connection options.

use crate::ssh::config::TransportLayerConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Database type enumeration.
///
/// This enum covers all supported databases, including protocol-compatible aliases.
/// Use [`resolve_effective_type()`](super::strategy::resolve_effective_type) to map
/// protocol-compatible variants to their native adapter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    // ── Native adapters (have dedicated implementations) ──
    /// PostgreSQL.
    PostgreSQL,
    /// MySQL.
    MySQL,
    /// SQL Server.
    SqlServer,
    /// SQLite.
    SQLite,
    /// SQLCipher — encrypted SQLite variant (same adapter, requires encryption key).
    SQLCipher,
    /// DuckDB — JDBC bridge.
    DuckDb,
    /// ClickHouse (HTTP protocol).
    ClickHouse,
    /// Firebird — JDBC bridge.
    Firebird,

    // ── PG wire protocol compatible (reuse PostgresAdapter) ──
    /// CockroachDB — PG wire protocol.
    CockroachDB,
    /// Amazon Redshift — PG wire protocol.
    Redshift,
    /// YugabyteDB — PG wire protocol.
    YugabyteDB,
    /// TimescaleDB — PG wire protocol.
    TimescaleDB,
    /// 人大金仓 KingbaseES — PG wire protocol.
    KingbaseES,
    /// 华为 GaussDB — PG wire protocol.
    GaussDB,
    /// 瀚高 HighGo — PG wire protocol.
    HighGo,
    /// 优炫 UXDB — PG wire protocol.
    UXDB,
    /// openGauss — PG wire protocol.
    OpenGauss,
    /// 南大通用 GBase 8c — PG wire protocol.
    GBase8c,
    /// QuestDB — PG wire protocol.
    QuestDB,
    /// 海量数据库 Vastbase — PG wire protocol.
    Vastbase,
    /// 崖山数据库 YashanDB — PG wire protocol.
    YashanDB,
    /// Greenplum / Cloudberry / Greengage — MPP analytics, PG wire protocol.
    Greenplum,
    /// EnterpriseDB (EDB) — PostgreSQL enterprise, PG wire protocol.
    EnterpriseDB,
    /// CrateDB — distributed real-time SQL, PG wire protocol.
    CrateDB,
    /// Materialize — streaming materialized SQL, PG wire protocol.
    Materialize,
    /// Google AlloyDB — cloud PostgreSQL fork, PG wire protocol.
    AlloyDB,
    /// Cloud SQL for PostgreSQL — Google managed PG, PG wire protocol.
    CloudSQLPG,
    /// Fujitsu Enterprise Postgres — PG fork, PG wire protocol.
    FujitsuPG,

    // ── MySQL wire protocol compatible (reuse MySQLAdapter) ──
    /// MariaDB — MySQL wire protocol.
    MariaDB,
    /// TiDB — MySQL wire protocol.
    TiDB,
    ///  OceanBase (MySQL mode) — MySQL wire protocol.
    OceanBase,
    ///  OceanBase Oracle mode — JDBC bridge (enterprise edition).
    OceanbaseOracle,
    /// 腾讯 TDSQL — MySQL wire protocol.
    TDSQL,
    /// 阿里云 PolarDB (MySQL mode) — MySQL wire protocol.
    PolarDB,
    /// Apache Doris — MySQL wire protocol.
    Doris,
    /// SelectDB (Doris fork) — MySQL wire protocol.
    SelectDB,
    /// StarRocks — MySQL wire protocol.
    StarRocks,
    /// Databend — MySQL wire protocol.
    Databend,
    /// GoldenDB — MySQL wire protocol.
    GoldenDB,
    /// Manticore Search — MySQL wire protocol.
    ManticoreSearch,
    /// SingleStore (MemSQL) — distributed real-time SQL, MySQL wire protocol.
    SingleStoreMemSQL,
    /// Cloud SQL for MySQL — Google managed MySQL, MySQL wire protocol.
    CloudSQLMySQL,
    // ── JDBC bridge (Java subprocess, lazy download) ──
    /// Oracle Database — JDBC bridge.
    Oracle,
    /// IBM DB2 — JDBC bridge.
    DB2,
    /// H2 — JDBC bridge.
    H2,
    /// Snowflake — JDBC bridge.
    Snowflake,
    /// TDengine — JDBC bridge.
    TDengine,
    /// Apache Derby — JDBC bridge.
    Derby,
    /// Apache Hive — JDBC bridge.
    Hive,
    /// Databricks SQL — JDBC bridge.
    Databricks,
    /// SAP HANA — JDBC bridge.
    Hana,
    /// Teradata — JDBC bridge.
    Teradata,
    /// Vertica — JDBC bridge.
    Vertica,
    /// Exasol — JDBC bridge.
    Exasol,
    /// Google BigQuery — JDBC bridge.
    BigQuery,
    /// IBM Informix — JDBC bridge.
    Informix,
    /// Apache Kylin — JDBC bridge.
    Kylin,
    /// Apache Cassandra — JDBC bridge.
    Cassandra,
    /// InterSystems IRIS — JDBC bridge.
    Iris,
    /// Microsoft Access — JDBC bridge.
    Access,
    /// 达梦 Dameng DM8 — JDBC bridge.
    Dameng,
    /// 虚谷 XuguDB — JDBC bridge.
    XuguDB,
    /// 南大通用 GBase 8a — JDBC bridge.
    GBase8a,

    // ── HTTP SQL bridge ──
    /// Trino — HTTP SQL API.
    Trino,
    /// Presto — HTTP SQL API.
    Presto,
    /// RQLite — HTTP SQL API.
    RQLite,
    /// Turso (libsql) — HTTP SQL API.
    Turso,
}

impl DatabaseType {
    /// Whether this connection type has an SSL/TLS concept at all.
    ///
    /// Embedded / file-based engines mirror the frontend
    /// `SSL_UNSUPPORTED_DATABASES`: their settings form never shows SSL, so
    /// no ssl mode is ever persisted and the stored value is just the
    /// `Prefer` default. The bridge must not receive one either — injecting
    /// SSL properties into such drivers breaks the connection (DuckDB
    /// rejects unknown options, issue #158).
    pub fn ssl_supported(self) -> bool {
        !matches!(
            self,
            DatabaseType::SQLite
                | DatabaseType::SQLCipher
                | DatabaseType::DuckDb
                | DatabaseType::Access
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_types_have_no_ssl_concept() {
        assert!(!DatabaseType::SQLite.ssl_supported());
        assert!(!DatabaseType::SQLCipher.ssl_supported());
        assert!(!DatabaseType::DuckDb.ssl_supported());
        assert!(!DatabaseType::Access.ssl_supported());
        // network types keep their ssl mode
        assert!(DatabaseType::PostgreSQL.ssl_supported());
        assert!(DatabaseType::MySQL.ssl_supported());
        assert!(DatabaseType::Oracle.ssl_supported());
        assert!(DatabaseType::H2.ssl_supported());
        assert!(DatabaseType::Firebird.ssl_supported());
    }
}

/// SSL/TLS mode for connections.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SslMode {
    /// Disable SSL/TLS.
    Disable,
    /// Prefer SSL/TLS but allow unencrypted connections.
    #[default]
    Prefer,
    /// Require SSL/TLS.
    Require,
    /// Verify CA certificate.
    VerifyCA,
    /// Verify full certificate chain.
    VerifyFull,
}

/// Connection pooling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections in the pool.
    pub min_connections: u32,
    /// Maximum number of connections in the pool.
    pub max_connections: u32,
    /// Maximum time to wait for a connection from the pool.
    #[serde(with = "duration_serde")]
    pub connection_timeout: Duration,
    /// Maximum lifetime of a connection in the pool.
    #[serde(with = "duration_serde")]
    pub max_lifetime: Duration,
    /// Maximum idle time for a connection before it's closed.
    #[serde(with = "duration_serde")]
    pub idle_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
            max_lifetime: Duration::from_secs(1800), // 30 minutes
            idle_timeout: Duration::from_secs(600),  // 10 minutes
        }
    }
}

/// Database connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Type of database.
    pub db_type: DatabaseType,
    /// Hostname or IP address.
    pub host: String,
    /// Port number.
    pub port: u16,
    /// Database name.
    pub database: Option<String>,
    /// Username for authentication.
    pub username: String,
    /// Password for authentication.
    pub password: Option<String>,
    /// Connection timeout in seconds (default: 10).
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,
    /// Query timeout in seconds (default: 30).
    #[serde(default = "default_query_timeout")]
    pub query_timeout_secs: u64,
    /// SSL/TLS mode.
    #[serde(default)]
    pub ssl_mode: SslMode,
    /// Path to CA certificate file.
    #[serde(default)]
    pub ssl_ca_cert: Option<String>,
    /// Path to client certificate file.
    #[serde(default)]
    pub ssl_client_cert: Option<String>,
    /// Path to client private key file.
    #[serde(default)]
    pub ssl_client_key: Option<String>,
    /// Trust server certificate (SQL Server specific).
    #[serde(default)]
    pub trust_server_certificate: bool,
    /// Oracle-specific connection options.
    #[serde(default)]
    pub oracle_options: Option<OracleConnectionOptions>,
    /// Additional connection options.
    #[serde(default)]
    pub options: std::collections::HashMap<String, String>,
    /// Connection pooling configuration.
    #[serde(default)]
    pub pool_config: PoolConfig,
    /// Transport layer configuration (SSH tunnels, proxies).
    #[serde(default)]
    pub transport_layers: Vec<TransportLayerConfig>,
}

/// Oracle-specific connection options for non-basic connection methods.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OracleConnectionOptions {
    pub connection_method: String,
    pub sid_or_service: Option<String>,
    pub role: Option<String>,
    pub tns_admin_dir: Option<String>,
    pub tns_alias: Option<String>,
    pub wallet_password: Option<String>,
    pub service_level: Option<String>,
}

fn default_connect_timeout() -> u64 {
    10
}
fn default_query_timeout() -> u64 {
    30
}

impl ConnectionConfig {
    /// Create a new connection configuration.
    pub fn new(
        db_type: DatabaseType,
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
    ) -> Self {
        Self {
            db_type,
            host: host.into(),
            port,
            database: None,
            username: username.into(),
            password: None,
            connect_timeout_secs: default_connect_timeout(),
            query_timeout_secs: default_query_timeout(),
            ssl_mode: SslMode::default(),
            ssl_ca_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            trust_server_certificate: false,
            oracle_options: None,
            options: std::collections::HashMap::new(),
            pool_config: PoolConfig::default(),
            transport_layers: Vec::new(),
        }
    }

    /// Set the database name.
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Set the password.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set the SSL mode.
    pub fn with_ssl_mode(mut self, ssl_mode: SslMode) -> Self {
        self.ssl_mode = ssl_mode;
        self
    }

    pub fn with_ssl_ca_cert(mut self, ca_cert: Option<String>) -> Self {
        self.ssl_ca_cert = ca_cert;
        self
    }
    pub fn with_ssl_client_cert(mut self, client_cert: Option<String>) -> Self {
        self.ssl_client_cert = client_cert;
        self
    }
    pub fn with_ssl_client_key(mut self, client_key: Option<String>) -> Self {
        self.ssl_client_key = client_key;
        self
    }
    pub fn with_trust_server_certificate(mut self, trust: bool) -> Self {
        self.trust_server_certificate = trust;
        self
    }

    /// Set the pool configuration.
    pub fn with_pool_config(mut self, pool_config: PoolConfig) -> Self {
        self.pool_config = pool_config;
        self
    }

    /// Set the Oracle-specific options.
    pub fn with_oracle_options(mut self, opts: OracleConnectionOptions) -> Self {
        self.oracle_options = Some(opts);
        self
    }

    /// Set the transport layer configuration.
    pub fn with_transport_layers(mut self, layers: Vec<TransportLayerConfig>) -> Self {
        self.transport_layers = layers;
        self
    }

    /// Set the connection timeout in seconds.
    pub fn with_connect_timeout(mut self, secs: u64) -> Self {
        self.connect_timeout_secs = secs;
        self
    }

    /// Set the query timeout in seconds.
    pub fn with_query_timeout(mut self, secs: u64) -> Self {
        self.query_timeout_secs = secs;
        self
    }

    /// Add a connection option.
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Expand a leading `~` to the current user's home directory.
///
/// File-based engines pass the path straight to the driver, which does not
/// expand shell short-hands, so `~/data/app.duckdb` would otherwise create a
/// literal `~` directory next to the process working directory.
pub fn expand_tilde(path: &str) -> String {
    let trimmed = path.trim();
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));
    match home {
        Ok(home) if trimmed == "~" => home,
        Ok(home) if trimmed.starts_with("~/") || trimmed.starts_with("~\\") => {
            format!("{}{}", home, &trimmed[1..])
        }
        _ => trimmed.to_string(),
    }
}

/// Build the `{database}` component of the DuckDB JDBC URL template
/// (`jdbc:duckdb:{database}`).
///
/// DuckDB accepts JDBC URL options after the database part, so read-only mode is
/// expressed as `access_mode=READ_ONLY`. Paths are tilde-expanded because the
/// driver treats the value literally.
///
/// The driver splits the URL on `;` before opening the database (verified
/// against `duckdb_jdbc` 1.5.6: `semi;colon.duckdb` fails with
/// `Invalid URL entry`), so such paths are rejected instead of being silently
/// turned into options — or, worse, into a read-write connection when the user
/// asked for read-only.
pub fn duckdb_database_value(host: &str, read_only: bool) -> Result<String, String> {
    let path = expand_tilde(host);
    if path.contains(';') {
        return Err(
            "DuckDB's JDBC driver splits the connection URL on ';', so a database path \
             containing ';' cannot be opened. Rename the file and try again."
                .to_string(),
        );
    }
    if !read_only {
        return Ok(path);
    }
    Ok(format!("{};access_mode=READ_ONLY", path))
}

/// Serialization helpers for Duration.
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod file_path_tests {
    use super::*;

    #[test]
    fn expand_tilde_leaves_plain_paths_untouched() {
        assert_eq!(expand_tilde("/tmp/app.duckdb"), "/tmp/app.duckdb");
        assert_eq!(expand_tilde(":memory:"), ":memory:");
        assert_eq!(expand_tilde("  /tmp/app.duckdb  "), "/tmp/app.duckdb");
    }

    #[test]
    fn expand_tilde_replaces_leading_home() {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));
        let Ok(home) = home else {
            return;
        };
        assert_eq!(
            expand_tilde("~/data/app.duckdb"),
            format!("{}/data/app.duckdb", home)
        );
        assert_eq!(expand_tilde("~"), home);
    }

    #[test]
    fn duckdb_database_value_appends_read_only_option() {
        assert_eq!(
            duckdb_database_value("/tmp/app.duckdb", false),
            Ok("/tmp/app.duckdb".to_string())
        );
        assert_eq!(
            duckdb_database_value("/tmp/app.duckdb", true),
            Ok("/tmp/app.duckdb;access_mode=READ_ONLY".to_string())
        );
    }

    #[test]
    fn duckdb_database_value_keeps_named_memory_instances() {
        assert_eq!(
            duckdb_database_value("memory:sqlkit_ab12cd34", false),
            Ok("memory:sqlkit_ab12cd34".to_string())
        );
        assert_eq!(
            duckdb_database_value("memory:sqlkit_ab12cd34", true),
            Ok("memory:sqlkit_ab12cd34;access_mode=READ_ONLY".to_string())
        );
    }

    #[test]
    fn duckdb_database_value_rejects_pre_joined_options() {
        // Callers pass a bare path; a value that already carries URL options is
        // rejected so options can never be applied twice by accident.
        let once = duckdb_database_value("/tmp/app.duckdb", true).expect("value");
        assert!(duckdb_database_value(&once, true).is_err());
    }

    #[test]
    fn duckdb_database_value_rejects_paths_with_semicolons() {
        // Verified against duckdb_jdbc 1.5.6: `jdbc:duckdb:/tmp/semi;colon.duckdb`
        // fails with `Invalid URL entry: colon.duckdb`.
        assert!(duckdb_database_value("/tmp/semi;colon.duckdb", false).is_err());
        assert!(duckdb_database_value("/tmp/semi;colon.duckdb", true).is_err());
    }
}
