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

    // Use helper function to create and connect adapter
    let connection = crate::commands::helpers::create_and_connect_adapter(
        &config.db_type,
        conn_config,
    )
    .await?;

    // Store connection
    let mut connections = state.connections.lock().await;
    connections.insert(id.clone(), connection.clone());
    
    // Get connection status with server metadata by calling test_connection
    let status = match &connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            adapter.test_connection().await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter.test_connection().await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            adapter.test_connection().await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.test_connection().await
        }
    }
    .map_err(|e| format!("Failed to get connection status: {}", e))?;

    Ok(status)
}

/// Disconnect from a server.
///
/// Removes the active connection from the application state and explicitly
/// disconnects from the database to ensure proper cleanup of resources,
/// including closing connections, releasing locks, and cleaning up transactions.
///
/// # Arguments
///
/// * `id` - ID of the server to disconnect from
/// * `state` - Application state
#[tauri::command]
pub async fn disconnect_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut connections = state.connections.lock().await;

    // Remove and explicitly disconnect
    let connection = connections
        .remove(&id)
        .ok_or_else(|| format!("No active connection found for server '{}'", id))?;
    
    // Call disconnect on the adapter to ensure proper cleanup
    let disconnect_result = match connection {
        ActiveConnection::Postgres(adapter) => {
            let mut adapter = adapter.lock().await;
            adapter.disconnect().await
        }
        ActiveConnection::MySQL(adapter) => {
            let mut adapter = adapter.lock().await;
            adapter.disconnect().await
        }
        ActiveConnection::SQLServer(adapter) => {
            let mut adapter = adapter.lock().await;
            adapter.disconnect().await
        }
        ActiveConnection::SQLite(adapter) => {
            let mut adapter = adapter.lock().await;
            adapter.disconnect().await
        }
    };
    
    // Log disconnect errors but don't fail the command since connection is already removed
    if let Err(e) = disconnect_result {
        eprintln!("Warning: Error during disconnect cleanup for '{}': {}", id, e);
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

// Tests for connection commands are temporarily disabled.
// TODO: Convert to integration tests with full Tauri context support.
// When re-enabling, remove the #[ignore] attribute or convert to integration tests.
#[cfg(test)]
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
