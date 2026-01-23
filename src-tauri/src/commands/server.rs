//! Server connection management commands.
//!
//! This module provides Tauri commands for managing database server configurations
//! and testing connections.

use crate::database::ConnectionStatus;
use crate::state::ServerConfig;
use tauri::State;

/// Save or update a server connection configuration.
///
/// # Arguments
///
/// * `config` - Server configuration to save
/// * `state` - Store state
///
/// # Returns
///
/// The ID of the saved server configuration.
#[tauri::command]
pub async fn save_connection(
    config: ServerConfig,
    state: State<'_, crate::commands::store::Store>,
) -> Result<String, String> {
    let id = config.id.clone();
    
    // Get store
    let store = state.get_store().await?;
    
    // Get existing connections
    let mut connections: Vec<ServerConfig> = match store.get("connections") {
        Some(value) => {
            if let Some(arr) = value.as_array() {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            } else {
                Vec::new()
            }
        }
        None => Vec::new(),
    };

    // Update or insert
    if let Some(pos) = connections.iter().position(|c| c.id == id) {
        connections[pos] = config;
    } else {
        connections.push(config);
    }

    // Save back to store
    store.set("connections".to_string(), serde_json::to_value(&connections).map_err(|e| e.to_string())?);
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok(id)
}

/// List all saved server connection configurations.
///
/// # Arguments
///
/// * `state` - Store state
///
/// # Returns
///
/// Vector of all saved server configurations.
#[tauri::command]
pub async fn list_connections(
    state: State<'_, crate::commands::store::Store>,
) -> Result<Vec<ServerConfig>, String> {
    let store = state.get_store().await?;
    
    match store.get("connections") {
        Some(value) => {
            if let Some(arr) = value.as_array() {
                let connections: Vec<ServerConfig> = arr
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                Ok(connections)
            } else {
                Ok(Vec::new())
            }
        }
        None => Ok(Vec::new()),
    }
}

/// Delete a server connection configuration.
///
/// # Arguments
///
/// * `id` - ID of the server to delete
/// * `state` - Store state
///
/// # Returns
///
/// Empty result on success.
#[tauri::command]
pub async fn delete_connection(
    id: String,
    state: State<'_, crate::commands::store::Store>,
) -> Result<(), String> {
    let store = state.get_store().await?;
    
    // Get existing connections
    let mut connections: Vec<ServerConfig> = match store.get("connections") {
        Some(value) => {
            if let Some(arr) = value.as_array() {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            } else {
                return Err(format!("Connection with ID '{}' not found", id));
            }
        }
        None => return Err(format!("Connection with ID '{}' not found", id)),
    };

    // Remove the connection
    let initial_len = connections.len();
    connections.retain(|c| c.id != id);
    
    if connections.len() == initial_len {
        return Err(format!("Connection with ID '{}' not found", id));
    }

    // Save back to store
    store.set("connections".to_string(), serde_json::to_value(&connections).map_err(|e| e.to_string())?);
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

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

#[cfg(test)]
mod tests {
    use super::*;

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
    async fn test_connection_sqlite() {
        let server = create_test_server();
        let result = test_connection(server).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.is_connected);
    }
}
