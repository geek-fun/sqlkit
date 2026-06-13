//! Configuration structures for database connections.
//!
//! This module defines configuration structures for establishing database connections
//! with support for various database types and connection options.

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
    /// DuckDB (embedded, bundled C lib via `duckdb` crate).
    DuckDb,
    /// ClickHouse (HTTP protocol).
    ClickHouse,

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

    // ── MySQL wire protocol compatible (reuse MySQLAdapter) ──
    /// MariaDB — MySQL wire protocol.
    MariaDB,
    /// TiDB — MySQL wire protocol.
    TiDB,
    ///  OceanBase (MySQL mode) — MySQL wire protocol.
    OceanBase,
    /// 腾讯 TDSQL — MySQL wire protocol.
    TDSQL,
    /// 阿里云 PolarDB (MySQL mode) — MySQL wire protocol.
    PolarDB,
    /// 达梦 DM8 (MySQL mode, secondary) — MySQL wire protocol alias.
    DM8,

    // ── JDBC bridge (Java subprocess, lazy download) ──
    /// Oracle Database — oracle-rs (native, optional feature).
    Oracle,
    /// IBM DB2 — JDBC bridge.
    DB2,
    /// H2 — JDBC bridge.
    H2,
    /// Snowflake — JDBC bridge.
    Snowflake,
    /// 达梦 DM8 (Oracle mode, primary) — JDBC bridge.
    DM8Oracle,
    /// 虚谷 XuguDB — JDBC bridge.
    XuguDB,
    /// 南大通用 GBase 8a — JDBC bridge.
    GBase8a,

    // ── HTTP SQL bridge ──
    /// Trino — HTTP SQL API.
    Trino,
    /// Presto — HTTP SQL API.
    Presto,
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
    /// Additional connection options.
    #[serde(default)]
    pub options: std::collections::HashMap<String, String>,
    /// Connection pooling configuration.
    #[serde(default)]
    pub pool_config: PoolConfig,
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
            ssl_mode: SslMode::default(),
            ssl_ca_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            trust_server_certificate: false,
            options: std::collections::HashMap::new(),
            pool_config: PoolConfig::default(),
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

    /// Set the pool configuration.
    pub fn with_pool_config(mut self, pool_config: PoolConfig) -> Self {
        self.pool_config = pool_config;
        self
    }

    /// Add a connection option.
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
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
