//! Connection lifecycle management commands.

use crate::database::{ConnectionStatus, DatabaseAdapter};
use crate::state::{ActiveConnection, AppState};
use tauri::State;

/// Connect to a server using the provided configuration.
#[tauri::command]
pub async fn connect_server(
    config: crate::state::ServerConfig,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let id = config.id.clone();
    let conn_config = config.to_connection_config()?;

    let connection =
        crate::commands::helpers::create_and_connect_adapter(&config.db_type, conn_config).await?;

    let mut connections = state.connections.lock().await;
    connections.insert(id.clone(), connection.clone());

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
#[tauri::command]
pub async fn disconnect_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut connections = state.connections.lock().await;

    let connection = connections
        .remove(&id)
        .ok_or_else(|| format!("No active connection found for server '{}'", id))?;

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

    if let Err(e) = disconnect_result {
        eprintln!(
            "Warning: Error during disconnect cleanup for '{}': {}",
            id, e
        );
    }

    Ok(())
}

/// Get the connection status for a server.
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
// The tests below require a Tauri State which cannot be created in unit tests.
// Integration tests should be added in src-tauri/tests/ directory.
#[cfg(test)]
mod tests {
    #[test]
    fn test_server_config_creation() {
        let config = crate::state::ServerConfig::new(
            "Test Server".to_string(),
            "sqlite".to_string(),
            ":memory:".to_string(),
            0,
            "user".to_string(),
        );

        assert!(!config.id.is_empty());
        assert_eq!(config.name, "Test Server");
        assert_eq!(config.db_type, "sqlite");
        assert_eq!(config.host, ":memory:");
    }
}
