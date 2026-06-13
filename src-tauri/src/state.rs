//! Application state management.
//!
//! This module provides the application-wide state that is shared across all Tauri commands.
//! The state includes connection managers for each database type and application configuration.

use crate::database::config::ConnectionConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Core adapter types used in dispatch logic.
use crate::database::{
    clickhouse::ClickHouseAdapter, duckdb::DuckDbAdapter, http_sql::HttpSqlAdapter,
    mysql::MySQLAdapter, odbc::OdbcAdapter, postgres::PostgresAdapter, sqlite::SQLiteAdapter,
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
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
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
            metadata: None,
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
            "oracle" => Ok(DatabaseType::Oracle),
            "db2" => Ok(DatabaseType::DB2),
            "h2" => Ok(DatabaseType::H2),
            "snowflake" => Ok(DatabaseType::Snowflake),
            "dm8" | "dm" => Ok(DatabaseType::DM8),
            "dm8_oracle" => Ok(DatabaseType::DM8Oracle),
            "trino" => Ok(DatabaseType::Trino),
            "presto" => Ok(DatabaseType::Presto),
            "cockroachdb" => Ok(DatabaseType::CockroachDB),
            "redshift" => Ok(DatabaseType::Redshift),
            "mariadb" => Ok(DatabaseType::MariaDB),
            "tidb" => Ok(DatabaseType::TiDB),
            "oceanbase" => Ok(DatabaseType::OceanBase),
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
            _ => Err(format!("Unsupported database type: {}", self.db_type)),
        }
    }

    /// Convert to ConnectionConfig for database operations.
    pub fn to_connection_config(&self) -> Result<ConnectionConfig, String> {
        let db_type = self.parse_db_type()?;

        let mut config = ConnectionConfig::new(db_type, &self.host, self.port, &self.username);

        if let Some(ref password) = self.password {
            config = config.with_password(password);
        }

        let db_lower = self.db_type.to_lowercase();
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
                _ => crate::database::SslMode::Prefer,
            };
            config = config.with_ssl_mode(ssl);
        }

        Ok(config)
    }
}

/// Active database connection wrapper used by the application state.
#[derive(Clone)]
pub enum ActiveConnection {
    Postgres(Arc<Mutex<PostgresAdapter>>),
    MySQL(Arc<Mutex<MySQLAdapter>>),
    SQLite(Arc<Mutex<SQLiteAdapter>>),
    SQLServer(Arc<Mutex<SqlServerAdapter>>),
    DuckDb(Arc<Mutex<DuckDbAdapter>>),
    ClickHouse(Arc<Mutex<ClickHouseAdapter>>),
    Odbc(Arc<Mutex<OdbcAdapter>>),
    HttpSql(Arc<Mutex<HttpSqlAdapter>>),
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
    pub connections: Arc<Mutex<HashMap<String, ActiveConnection>>>,
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
