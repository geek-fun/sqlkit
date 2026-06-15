use crate::database::{ConnectionStatus, DatabaseAdapter};
use crate::ssh::TunnelManager;
use crate::state::{ActiveConnection, AppState};
use tauri::State;

#[tauri::command]
pub async fn connect_server(
    config: crate::state::ServerConfig,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let id = config.id.clone();
    let mut conn_config = config.to_connection_config()?;

    let (host, port) =
        crate::commands::helpers::connection_host_port(&id, &conn_config, &state.tunnels).await?;
    conn_config.host = host;
    conn_config.port = port;

    let connection =
        crate::commands::helpers::create_and_connect_adapter(&config.db_type, conn_config).await?;

    let mut connections = state.connections.lock().await;
    connections.insert(id.clone(), connection.clone());

    let status = match &connection {
        ActiveConnection::Postgres(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::MySQL(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::SQLServer(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::SQLite(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::DuckDb(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        #[cfg(feature = "firebird")]
        ActiveConnection::Firebird(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::HttpSql(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::Rqlite(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
        ActiveConnection::Turso(adapter) => {
            let a = adapter.lock().await;
            a.test_connection().await
        }
    }
    .map_err(|e| format!("Failed to get connection status: {}", e))?;

    Ok(status)
}

#[tauri::command]
pub async fn disconnect_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.tunnels.stop_tunnel(&id).await;

    let mut connections = state.connections.lock().await;

    let connection = connections
        .remove(&id)
        .ok_or_else(|| format!("No active connection found for server '{}'", id))?;

    let disconnect_result = match connection {
        ActiveConnection::Postgres(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::MySQL(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::SQLServer(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::SQLite(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::DuckDb(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::ClickHouse(adapter) => adapter.lock().await.disconnect().await,
        #[cfg(feature = "firebird")]
        ActiveConnection::Firebird(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::JdbcBridge(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::HttpSql(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::Rqlite(adapter) => adapter.lock().await.disconnect().await,
        ActiveConnection::Turso(adapter) => adapter.lock().await.disconnect().await,
    };

    if let Err(e) = disconnect_result {
        eprintln!(
            "Warning: Error during disconnect cleanup for '{}': {}",
            id, e
        );
    }

    Ok(())
}

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
