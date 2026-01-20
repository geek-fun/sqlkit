//! Helper functions for Tauri commands.
//!
//! This module contains shared utilities to reduce code duplication across commands.

use crate::database::{config::ConnectionConfig, ConnectionStatus, DatabaseAdapter};
use crate::state::ActiveConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create and connect a database adapter based on database type.
///
/// This helper function encapsulates the logic of creating the appropriate
/// adapter for a given database type, connecting to it, and wrapping it
/// in an ActiveConnection enum for storage in application state.
///
/// # Arguments
///
/// * `db_type` - Database type string (e.g., "postgresql", "mysql", "sqlite", "sqlserver")
/// * `conn_config` - Connection configuration
///
/// # Returns
///
/// An `ActiveConnection` variant containing the connected adapter.
///
/// # Errors
///
/// Returns an error if the database type is unsupported or connection fails.
pub async fn create_and_connect_adapter(
    db_type: &str,
    conn_config: ConnectionConfig,
) -> Result<ActiveConnection, String> {
    match db_type.to_lowercase().as_str() {
        "postgresql" | "postgres" => {
            use crate::database::postgres::PostgresAdapter;
            let mut adapter = PostgresAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            Ok(ActiveConnection::Postgres(Arc::new(Mutex::new(adapter))))
        }
        "mysql" => {
            use crate::database::mysql::MySQLAdapter;
            let mut adapter = MySQLAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            Ok(ActiveConnection::MySQL(Arc::new(Mutex::new(adapter))))
        }
        "sqlserver" | "mssql" => {
            use crate::database::sqlserver::SqlServerAdapter;
            let mut adapter = SqlServerAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            Ok(ActiveConnection::SQLServer(Arc::new(Mutex::new(adapter))))
        }
        "sqlite" => {
            use crate::database::sqlite::SQLiteAdapter;
            let mut adapter = SQLiteAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            Ok(ActiveConnection::SQLite(Arc::new(Mutex::new(adapter))))
        }
        _ => Err(format!("Unsupported database type: {}", db_type)),
    }
}

/// Test connection for a given database configuration.
///
/// This helper creates a temporary adapter, connects, tests the connection,
/// and returns the connection status without storing the adapter.
///
/// # Arguments
///
/// * `db_type` - Database type string
/// * `conn_config` - Connection configuration
///
/// # Returns
///
/// Connection status with server metadata.
///
/// # Errors
///
/// Returns an error if connection or test fails.
pub async fn test_connection(
    db_type: &str,
    conn_config: ConnectionConfig,
) -> Result<ConnectionStatus, String> {
    match db_type.to_lowercase().as_str() {
        "postgresql" | "postgres" => {
            use crate::database::postgres::PostgresAdapter;
            let mut adapter = PostgresAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Connection test failed: {}", e))?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        "mysql" => {
            use crate::database::mysql::MySQLAdapter;
            let mut adapter = MySQLAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Connection test failed: {}", e))?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        "sqlserver" | "mssql" => {
            use crate::database::sqlserver::SqlServerAdapter;
            let mut adapter = SqlServerAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Connection test failed: {}", e))?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        "sqlite" => {
            use crate::database::sqlite::SQLiteAdapter;
            let mut adapter = SQLiteAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Connection test failed: {}", e))?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        _ => Err(format!("Unsupported database type: {}", db_type)),
    }
}
