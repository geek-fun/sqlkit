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

/// Active database connection wrapper.
pub enum ActiveConnection {
    Postgres(Arc<Mutex<crate::database::postgres::PostgresAdapter>>),
    MySQL(Arc<Mutex<crate::database::mysql::MySQLAdapter>>),
    SQLite(Arc<Mutex<crate::database::sqlite::SQLiteAdapter>>),
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
    pub fn load_config(&self) -> Result<(), String> {
        // TODO: Implement file-based configuration loading
        // For now, we use in-memory configuration
        Ok(())
    }

    /// Save configuration to storage (placeholder for file-based config).
    pub fn save_config(&self) -> Result<(), String> {
        // TODO: Implement file-based configuration saving
        // For now, configuration is only stored in memory
        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
