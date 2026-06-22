//! Application state management.
//!
//! This module provides the application-wide state that is shared across all Tauri commands.
//! The state includes connection managers for each database type and application configuration.

use crate::database::config::{ConnectionConfig, OracleConnectionOptions};
use crate::ssh::config::TransportLayerConfig;
use crate::ssh::TunnelManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::database::rqlite::RqliteAdapter;
use crate::database::turso::TursoAdapter;
/// Core adapter types used in dispatch logic.
use crate::database::{
    clickhouse::ClickHouseAdapter, http_sql::HttpSqlAdapter, jdbc_bridge::JdbcBridgeAdapter,
    mysql::MySQLAdapter, postgres::PostgresAdapter, sqlite::SQLiteAdapter,
    sqlserver::SqlServerAdapter,
};

/// Server configuration with connection details.
///
/// # Security Warning
///
/// This struct includes a `password` field that is serialized to JSON when
/// saved via `AppConfig`. While `skip_serializing_if` prevents `None` values
/// from being serialized, actual passwords will be stored in plaintext in the
/// configuration file.
///
/// **Recommendations for production use:**
/// - Encrypt passwords before storage using a secure encryption method
/// - Use a system credential manager (e.g., Windows Credential Manager, macOS Keychain)
/// - Implement a master password or key derivation function
/// - Document that `save_config()` will persist sensitive credentials
///
/// For now, this is a basic implementation suitable for development and testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Unique identifier for the server configuration.
    pub id: String,
    /// Human-readable name for the server.
    pub name: String,
    /// Database type (PostgreSQL, MySQL, SQLite, SQLServer).
    pub db_type: String,
    /// Hostname or IP address.
    pub host: String,
    /// Port number.
    pub port: u16,
    /// Username for authentication.
    pub username: String,
    /// Password for authentication (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Default database to connect to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    /// SSL mode configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl_ca_cert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl_client_cert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl_client_key: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not", deserialize_with = "deserialize_bool_or_null")]
    pub trust_server_certificate: bool,
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Oracle-specific connection options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle_options: Option<OracleConnectionOptions>,
    /// Connection timeout in seconds (default: 10).
    #[serde(default = "default_timeout_10")]
    pub connect_timeout_secs: u64,
    /// Query timeout in seconds (default: 30).
    #[serde(default = "default_timeout_30")]
    pub query_timeout_secs: u64,
    /// Transport layer configuration (SSH tunnels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_layers: Option<Vec<TransportLayerConfig>>,
}

fn default_timeout_10() -> u64 {
    10
}
fn default_timeout_30() -> u64 {
    30
}

fn deserialize_bool_or_null<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer).map(|v| v.unwrap_or(false))
}

impl ServerConfig {
    /// Create a new server configuration.
    pub fn new(name: String, db_type: String, host: String, port: u16, username: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            db_type,
            host,
            port,
            username,
            password: None,
            database: None,
            ssl_mode: None,
            ssl_ca_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            trust_server_certificate: false,
            metadata: None,
            oracle_options: None,
            connect_timeout_secs: default_timeout_10(),
            query_timeout_secs: default_timeout_30(),
            transport_layers: None,
        }
    }

    /// Parse database type string to DatabaseType enum.
    pub fn parse_db_type(&self) -> Result<crate::database::DatabaseType, String> {
        use crate::database::DatabaseType;
        match self.db_type.to_lowercase().as_str() {
            "postgresql" | "postgres" => Ok(DatabaseType::PostgreSQL),
            "mysql" => Ok(DatabaseType::MySQL),
            "sqlserver" | "mssql" => Ok(DatabaseType::SqlServer),
            "sqlite" => Ok(DatabaseType::SQLite),
            "duckdb" | "duck" => Ok(DatabaseType::DuckDb),
            "clickhouse" => Ok(DatabaseType::ClickHouse),
            "firebird" => Ok(DatabaseType::Firebird),
            "oracle" => Ok(DatabaseType::Oracle),
            "db2" => Ok(DatabaseType::DB2),
            "h2" => Ok(DatabaseType::H2),
            "snowflake" => Ok(DatabaseType::Snowflake),
            "tdengine" | "td" => Ok(DatabaseType::TDengine),
            "dameng" | "dm" | "dm8" | "dm8_oracle" => Ok(DatabaseType::Dameng),
            "trino" => Ok(DatabaseType::Trino),
            "presto" => Ok(DatabaseType::Presto),
            "rqlite" => Ok(DatabaseType::RQLite),
            "turso" | "libsql" => Ok(DatabaseType::Turso),
            "cockroachdb" => Ok(DatabaseType::CockroachDB),
            "redshift" => Ok(DatabaseType::Redshift),
            "mariadb" => Ok(DatabaseType::MariaDB),
            "tidb" => Ok(DatabaseType::TiDB),
            "oceanbase" => Ok(DatabaseType::OceanBase),
            "oceanbase-oracle" | "oceanbase_oracle" => Ok(DatabaseType::OceanbaseOracle),
            "tdsql" => Ok(DatabaseType::TDSQL),
            "polardb" => Ok(DatabaseType::PolarDB),
            "kingbasees" | "kingbase" => Ok(DatabaseType::KingbaseES),
            "gaussdb" => Ok(DatabaseType::GaussDB),
            "highgo" => Ok(DatabaseType::HighGo),
            "uxdb" => Ok(DatabaseType::UXDB),
            "opengauss" => Ok(DatabaseType::OpenGauss),
            "gbase8c" => Ok(DatabaseType::GBase8c),
            "xugudb" | "xugu" => Ok(DatabaseType::XuguDB),
            "gbase8a" => Ok(DatabaseType::GBase8a),
            "doris" => Ok(DatabaseType::Doris),
            "selectdb" => Ok(DatabaseType::SelectDB),
            "starrocks" => Ok(DatabaseType::StarRocks),
            "databend" => Ok(DatabaseType::Databend),
            "goldendb" => Ok(DatabaseType::GoldenDB),
            "manticore" | "manticore_search" => Ok(DatabaseType::ManticoreSearch),
            "questdb" => Ok(DatabaseType::QuestDB),
            "vastbase" => Ok(DatabaseType::Vastbase),
            "yashandb" => Ok(DatabaseType::YashanDB),
            "greenplum" | "cloudberry" | "greengage" => Ok(DatabaseType::Greenplum),
            "edb" | "enterprisedb" => Ok(DatabaseType::EnterpriseDB),
            "cratedb" | "crate" => Ok(DatabaseType::CrateDB),
            "materialize" => Ok(DatabaseType::Materialize),
            "alloydb" | "google_alloydb" => Ok(DatabaseType::AlloyDB),
            "cloudsqlpg" | "cloud_sql_pg" => Ok(DatabaseType::CloudSQLPG),
            "fujitsupg" | "fujitsu_pg" => Ok(DatabaseType::FujitsuPG),
            "singlestore" | "memsql" | "single_store" => Ok(DatabaseType::SingleStoreMemSQL),
            "cloudsqlmysql" | "cloud_sql_mysql" => Ok(DatabaseType::CloudSQLMySQL),
            "derby" => Ok(DatabaseType::Derby),
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
            _ => Err(format!("Unsupported database type: {}", self.db_type)),
        }
    }

    /// Convert to ConnectionConfig for database operations.
    pub fn to_connection_config(&self) -> Result<ConnectionConfig, String> {
        let db_type = self.parse_db_type()?;

        let mut config = ConnectionConfig::new(db_type, &self.host, self.port, &self.username)
            .with_connect_timeout(self.connect_timeout_secs)
            .with_query_timeout(self.query_timeout_secs);

        if let Some(ref password) = self.password {
            config = config.with_password(password);
        }

        let db_lower = self.db_type.to_lowercase();
        // DuckDB is file-based, uses JDBC bridge with jdbc:duckdb:{filepath}
        if db_lower == "sqlite" || db_lower == "duckdb" || db_lower == "duck" {
            config = config.with_database(&self.host);
        } else if let Some(ref database) = self.database {
            config = config.with_database(database);
        }

        if let Some(ref ssl_mode) = self.ssl_mode {
            let ssl = match ssl_mode.to_lowercase().as_str() {
                "disable" => crate::database::SslMode::Disable,
                "prefer" => crate::database::SslMode::Prefer,
                "require" => crate::database::SslMode::Require,
                "verify-ca" | "verify_ca" => crate::database::SslMode::VerifyCA,
                "verify-full" | "verify_full" => crate::database::SslMode::VerifyFull,
                _ => crate::database::SslMode::Prefer,
            };
            config = config.with_ssl_mode(ssl);
        } else {
            config = config.with_ssl_mode(crate::database::SslMode::Disable);
        }

        config = config
            .with_ssl_ca_cert(self.ssl_ca_cert.clone())
            .with_ssl_client_cert(self.ssl_client_cert.clone())
            .with_ssl_client_key(self.ssl_client_key.clone())
            .with_trust_server_certificate(self.trust_server_certificate);

        if let Some(ref layers) = self.transport_layers {
            config = config.with_transport_layers(layers.clone());
        }

        if let Some(ref oracle_opts) = self.oracle_options {
            if db_type == crate::database::DatabaseType::Oracle
                || db_type == crate::database::DatabaseType::OceanbaseOracle
            {
                config = config.with_oracle_options(oracle_opts.clone());
            }
        }

        Ok(config)
    }
}

/// Active database connection wrapper used by the application state.
/// DuckDB, Firebird, and Oracle now use the JdbcBridge variant.
#[derive(Clone)]
pub enum ActiveConnection {
    Postgres(Arc<Mutex<PostgresAdapter>>),
    MySQL(Arc<Mutex<MySQLAdapter>>),
    SQLite(Arc<Mutex<SQLiteAdapter>>),
    SQLServer(Arc<Mutex<SqlServerAdapter>>),
    ClickHouse(Arc<Mutex<ClickHouseAdapter>>),
    JdbcBridge(Arc<Mutex<JdbcBridgeAdapter>>),
    HttpSql(Arc<Mutex<HttpSqlAdapter>>),
    Rqlite(Arc<Mutex<RqliteAdapter>>),
    Turso(Arc<Mutex<TursoAdapter>>),
}

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Application settings.
    pub settings: HashMap<String, String>,
}

/// Application state shared across all Tauri commands.
pub struct AppState {
    /// Active database connections indexed by connection ID.
    pub connections: Arc<RwLock<HashMap<String, ActiveConnection>>>,
    /// LRU cache for cross-database connection handles.
    pub cache: crate::connection::cache::ConnectionCache,
    /// SSH tunnel lifecycle manager.
    pub tunnels: TunnelManager,
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            cache: crate::connection::cache::ConnectionCache::default(),
            tunnels: TunnelManager::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
