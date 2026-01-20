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
    pub fn new(
        name: String,
        db_type: String,
        host: String,
        port: u16,
        username: String,
    ) -> Self {
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

    /// Convert to ConnectionConfig for database operations.
    pub fn to_connection_config(&self) -> Result<ConnectionConfig, String> {
        let db_type = match self.db_type.to_lowercase().as_str() {
            "postgresql" | "postgres" => crate::database::DatabaseType::PostgreSQL,
            "mysql" => crate::database::DatabaseType::MySQL,
            "sqlserver" | "mssql" => crate::database::DatabaseType::SqlServer,
            "sqlite" => crate::database::DatabaseType::SQLite,
            _ => return Err(format!("Unsupported database type: {}", self.db_type)),
        };

        let mut config = ConnectionConfig::new(
            db_type,
            &self.host,
            self.port,
            &self.username,
        );

        if let Some(ref password) = self.password {
            config = config.with_password(password);
        }

        if let Some(ref database) = self.database {
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
///
/// This enum holds the currently active database adapters for a given connection
/// ID (see [`AppState::connections`]). Each variant corresponds to a concrete
/// database adapter implementation that conforms to the `DatabaseAdapter` trait
/// in `crate::database`.
///
/// Adapters are wrapped in `Arc<Mutex<...>>` so they can be:
///
/// - **Shared** across multiple Tauri commands and async tasks (`Arc`)
/// - **Mutably accessed** in an async context while preserving thread safety
///   (`tokio::sync::Mutex`)
///
/// This allows commands to clone an `ActiveConnection`, lock the underlying
/// adapter, and perform queries without needing to re-establish connections
/// or manage lifetimes manually.
///
/// # Example
///
/// ```ignore
/// let connections = state.connections.lock().await;
/// if let Some(ActiveConnection::Postgres(adapter)) = connections.get(&conn_id) {
///     let adapter = adapter.lock().await;
///     let result = adapter.execute_query("SELECT 1").await?;
/// }
/// ```
pub enum ActiveConnection {
    /// Active PostgreSQL connection backed by a [`PostgresAdapter`](crate::database::postgres::PostgresAdapter).
    ///
    /// The adapter is wrapped in `Arc<Mutex<_>>` so that multiple commands can
    /// share the same PostgreSQL connection pool/adapter instance and perform
    /// concurrent operations by acquiring the async mutex lock when needed.
    Postgres(Arc<Mutex<crate::database::postgres::PostgresAdapter>>),
    
    /// Active MySQL connection backed by a [`MySQLAdapter`](crate::database::mysql::MySQLAdapter).
    ///
    /// Stored inside `Arc<Mutex<_>>` for shared, synchronized access to the
    /// underlying MySQL connection pool/adapter from different Tauri commands.
    MySQL(Arc<Mutex<crate::database::mysql::MySQLAdapter>>),
    
    /// Active SQLite connection backed by a [`SQLiteAdapter`](crate::database::sqlite::SQLiteAdapter).
    ///
    /// The `Arc<Mutex<_>>` wrapper allows safe mutable access to the adapter
    /// even when it is shared across async tasks, which is important because
    /// SQLite connections are often single-threaded and must be coordinated.
    SQLite(Arc<Mutex<crate::database::sqlite::SQLiteAdapter>>),
    
    /// Active SQL Server connection backed by a [`SqlServerAdapter`](crate::database::sqlserver::SqlServerAdapter).
    ///
    /// As with the other variants, `Arc<Mutex<_>>` enables concurrent commands
    /// to share a single SQL Server adapter instance while serializing mutable
    /// access through the async mutex.
    SQLServer(Arc<Mutex<crate::database::sqlserver::SqlServerAdapter>>),
}

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Saved server configurations.
    pub servers: HashMap<String, ServerConfig>,
    /// Application settings.
    pub settings: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            servers: HashMap::new(),
            settings: HashMap::new(),
        }
    }
}

/// Application state shared across all Tauri commands.
pub struct AppState {
    /// Active database connections indexed by connection ID.
    pub connections: Arc<Mutex<HashMap<String, ActiveConnection>>>,
    /// Application configuration.
    pub config: Arc<tokio::sync::Mutex<AppConfig>>,
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            config: Arc::new(tokio::sync::Mutex::new(AppConfig::default())),
        }
    }

    /// Load configuration from storage (placeholder for file-based config).
    ///
    /// # Returns
    ///
    /// An error indicating configuration loading is not yet implemented.
    ///
    /// # Note
    ///
    /// This is a placeholder method. Until persistence is implemented, calling
    /// this method will return an error so that callers are aware that loading
    /// has not actually succeeded.
    pub fn load_config(&self) -> Result<(), String> {
        // TODO: Implement file-based configuration loading.
        // Until persistence is implemented, explicitly return an error
        // so that callers do not assume loading has succeeded.
        Err("Configuration loading is not yet implemented".to_string())
    }

    /// Save configuration to storage (placeholder for file-based config).
    ///
    /// # Returns
    ///
    /// An error indicating configuration saving is not yet implemented.
    ///
    /// # Note
    ///
    /// This is a placeholder method. Until persistence is implemented, calling
    /// this method will return an error so that callers are aware that saving
    /// has not actually succeeded. Configuration changes will only exist in memory.
    pub fn save_config(&self) -> Result<(), String> {
        // TODO: Implement file-based configuration saving.
        // Until persistence is implemented, explicitly return an error
        // so that callers do not assume saving has succeeded.
        Err("Configuration saving is not yet implemented".to_string())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
