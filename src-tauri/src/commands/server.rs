//! Server configuration management commands.
//!
//! This module provides Tauri commands for managing database server configurations,
//! including saving, listing, deleting, and testing connections.

use crate::state::{AppState, ServerConfig};
use crate::database::{ConnectionStatus, DatabaseAdapter};
use tauri::State;

/// Save or update a server configuration.
///
/// # Arguments
///
/// * `config` - Server configuration to save
/// * `state` - Application state
///
/// # Returns
///
/// The ID of the saved server configuration.
#[tauri::command]
pub async fn save_server(
    config: ServerConfig,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut app_config = state.config.lock().await;

    let server_id = config.id.clone();
    app_config.servers.insert(server_id.clone(), config);

    // Save to persistent storage
    drop(app_config);
    state.save_config()?;

    Ok(server_id)
}

/// List all saved server configurations.
///
/// # Arguments
///
/// * `state` - Application state
///
/// # Returns
///
/// Vector of all saved server configurations.
#[tauri::command]
pub async fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerConfig>, String> {
    let app_config = state.config.lock().await;
    Ok(app_config.servers.values().cloned().collect())
}

/// Delete a server configuration.
///
/// # Arguments
///
/// * `id` - ID of the server to delete
/// * `state` - Application state
///
/// # Returns
///
/// Empty result on success.
#[tauri::command]
pub async fn delete_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut app_config = state.config.lock().await;

    if app_config.servers.remove(&id).is_none() {
        return Err(format!("Server with ID '{}' not found", id));
    }

    // Save to persistent storage
    drop(app_config);
    state.save_config()?;

    Ok(())
}

/// Test a database connection without saving the configuration.
///
/// # Arguments
///
/// * `config` - Server configuration to test
///
/// # Returns
///
/// Connection status indicating success or failure.
#[tauri::command]
pub async fn test_connection(config: ServerConfig) -> Result<ConnectionStatus, String> {
    let conn_config = config.to_connection_config()?;
    
    // Use helper function to test connection
    crate::commands::helpers::test_connection(&config.db_type, conn_config).await
}

// Tests for server commands are temporarily disabled.
// TODO: Convert to integration tests with full Tauri context support.
// When re-enabling, remove the #[ignore] attribute or convert to integration tests.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

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
    async fn test_save_and_list_servers() {
        let state = create_test_state();
        let server = create_test_server();
        let server_id = server.id.clone();

        // Save server
        let result = save_server(server, State::from(&state)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), server_id);

        // List servers
        let servers = list_servers(State::from(&state)).await.unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].id, server_id);
    }

    #[tokio::test]
    async fn test_delete_server() {
        let state = create_test_state();
        let server = create_test_server();
        let server_id = server.id.clone();

        // Save and delete server
        save_server(server, State::from(&state)).await.unwrap();
        let result = delete_server(server_id.clone(), State::from(&state)).await;
        assert!(result.is_ok());

        // Verify deletion
        let servers = list_servers(State::from(&state)).await.unwrap();
        assert_eq!(servers.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_server() {
        let state = create_test_state();
        let result = delete_server("nonexistent".to_string(), State::from(&state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_sqlite() {
        let server = create_test_server();
        let result = test_connection(server).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.is_connected);
    }
}
