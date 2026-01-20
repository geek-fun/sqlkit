//! Connection lifecycle management commands.
//!
//! This module provides Tauri commands for managing active database connections,
//! including connecting, disconnecting, and checking connection status.

use crate::database::{ConnectionStatus, DatabaseAdapter};
use crate::state::{ActiveConnection, AppState};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Connect to a saved server.
///
/// Creates a new connection using a saved server configuration and stores it
/// in the application state for future queries.
///
/// # Arguments
///
/// * `id` - ID of the saved server to connect to
/// * `state` - Application state
///
/// # Returns
///
/// Connection status indicating success or failure.
#[tauri::command]
pub async fn connect_server(
    id: String,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    // Get server config
    let config = {
        let app_config = state.config.lock().await;

        app_config
            .servers
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("Server with ID '{}' not found", id))?
    };

    let conn_config = config.to_connection_config()?;

    // Create connection based on database type
    let connection = match config.db_type.to_lowercase().as_str() {
        "postgresql" | "postgres" => {
            use crate::database::postgres::PostgresAdapter;
            let mut adapter = PostgresAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            ActiveConnection::Postgres(Arc::new(Mutex::new(adapter)))
        }
        "mysql" => {
            use crate::database::mysql::MySQLAdapter;
            let mut adapter = MySQLAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            ActiveConnection::MySQL(Arc::new(Mutex::new(adapter)))
        }
        "sqlserver" | "mssql" => {
            use crate::database::sqlserver::SqlServerAdapter;
            let mut adapter = SqlServerAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            ActiveConnection::SQLServer(Arc::new(Mutex::new(adapter)))
        }
        "sqlite" => {
            use crate::database::sqlite::SQLiteAdapter;
            let mut adapter = SQLiteAdapter::new(conn_config);
            adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            ActiveConnection::SQLite(Arc::new(Mutex::new(adapter)))
        }
        _ => return Err(format!("Unsupported database type: {}", config.db_type)),
    };

    // Store connection
    let mut connections = state.connections.lock().await;
    connections.insert(id.clone(), connection);

    Ok(ConnectionStatus {
        is_connected: true,
        server_version: None,
        current_database: None,
        current_user: None,
        metadata: Default::default(),
    })
}

/// Disconnect from a server.
///
/// Removes the active connection from the application state and cleans up resources.
///
/// # Arguments
///
/// * `id` - ID of the server to disconnect from
/// * `state` - Application state
#[tauri::command]
pub async fn disconnect_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut connections = state.connections.lock().await;

    if connections.remove(&id).is_none() {
        return Err(format!("No active connection found for server '{}'", id));
    }

    Ok(())
}

/// Get the connection status for a server.
///
/// # Arguments
///
/// * `id` - ID of the server to check
/// * `state` - Application state
///
/// # Returns
///
/// Connection status indicating if the server is connected.
#[tauri::command]
pub async fn get_connection_status(
    id: String,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let connections = state.connections.lock().await;

    let is_connected = connections.contains_key(&id);

    Ok(ConnectionStatus {
        is_connected,
        server_version: None,
        current_database: None,
        current_user: None,
        metadata: Default::default(),
    })
}

#[cfg(test)]
#[cfg(not(test))] // Temporarily disabled - need to convert to integration tests with Tauri context
mod tests {
    use super::*;
    use crate::state::{AppState, ServerConfig};

    fn create_test_state() -> AppState {
        AppState::new()
    }

    fn create_test_server() -> ServerConfig {
        ServerConfig::new(
            "Test Server".to_string(),
            "sqlite".to_string(),
            ":memory:".to_string(),
            0,
            "".to_string(),
        )
    }

    #[tokio::test]
    async fn test_connect_and_disconnect() {
        let state = create_test_state();
        let server = create_test_server();
        let server_id = server.id.clone();

        // Save server first
        {
            let mut config = state.config.blocking_lock();
            config.servers.insert(server_id.clone(), server);
        }

        // Connect
        let result = connect_server(server_id.clone(), State::from(&state)).await;
        assert!(result.is_ok());

        // Check status
        let status = get_connection_status(server_id.clone(), State::from(&state))
            .await
            .unwrap();
        assert!(status.is_connected);

        // Disconnect
        let result = disconnect_server(server_id.clone(), State::from(&state)).await;
        assert!(result.is_ok());

        // Check status after disconnect
        let status = get_connection_status(server_id.clone(), State::from(&state))
            .await
            .unwrap();
        assert!(!status.is_connected);
    }

    #[tokio::test]
    async fn test_connect_nonexistent_server() {
        let state = create_test_state();
        let result = connect_server("nonexistent".to_string(), State::from(&state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_connection() {
        let state = create_test_state();
        let result = disconnect_server("nonexistent".to_string(), State::from(&state)).await;
        assert!(result.is_err());
    }
}
