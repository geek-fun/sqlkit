//! Helper functions for Tauri commands.

use crate::database::{config::ConnectionConfig, ConnectionStatus, DatabaseAdapter};
use crate::state::ActiveConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create and connect a database adapter based on database type.
pub async fn create_and_connect_adapter(
    db_type: &str,
    conn_config: ConnectionConfig,
) -> Result<ActiveConnection, String> {
    match db_type.to_lowercase().as_str() {
        "postgresql" | "postgres" => {
            use crate::database::postgres::PostgresAdapter;
            let mut adapter = PostgresAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            Ok(ActiveConnection::Postgres(Arc::new(Mutex::new(adapter))))
        }
        "mysql" => {
            use crate::database::mysql::MySQLAdapter;
            let mut adapter = MySQLAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            Ok(ActiveConnection::MySQL(Arc::new(Mutex::new(adapter))))
        }
        "sqlserver" | "mssql" => {
            use crate::database::sqlserver::SqlServerAdapter;
            let mut adapter = SqlServerAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            Ok(ActiveConnection::SQLServer(Arc::new(Mutex::new(adapter))))
        }
        "sqlite" => {
            use crate::database::sqlite::SQLiteAdapter;
            let mut adapter = SQLiteAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            Ok(ActiveConnection::SQLite(Arc::new(Mutex::new(adapter))))
        }
        _ => Err(format!("Unsupported database type: {}", db_type)),
    }
}

/// Test connection for a given database configuration.
pub async fn test_connection(
    db_type: &str,
    conn_config: ConnectionConfig,
) -> Result<ConnectionStatus, String> {
    match db_type.to_lowercase().as_str() {
        "postgresql" | "postgres" => {
            use crate::database::postgres::PostgresAdapter;
            let mut adapter = PostgresAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        "mysql" => {
            use crate::database::mysql::MySQLAdapter;
            let mut adapter = MySQLAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        "sqlserver" | "mssql" => {
            use crate::database::sqlserver::SqlServerAdapter;
            let mut adapter = SqlServerAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        "sqlite" => {
            use crate::database::sqlite::SQLiteAdapter;
            let mut adapter = SQLiteAdapter::new(conn_config);
            adapter.connect().await.map_err(|e| e.to_string())?;
            adapter.test_connection().await.map_err(|e| e.to_string())
        }
        _ => Err(format!("Unsupported database type: {}", db_type)),
    }
}
