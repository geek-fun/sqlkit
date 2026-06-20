use crate::connection::handle::ConnectionHandle;
use crate::database::ConnectionStatus;
use crate::state::AppState;
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

    let timeout_secs = conn_config.connect_timeout_secs;

    let connection = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        crate::commands::helpers::create_and_connect_adapter(&config.db_type, conn_config),
    )
    .await
    .map_err(|_| format!("Connection timed out after {} seconds", timeout_secs))??;

    let mut connections = state.connections.write().await;
    connections.insert(id.clone(), connection.clone());
    drop(connections);
    let status = connection
        .test_connection()
        .await
        .map_err(|e| format!("Failed to get connection status: {}", e))?;

    Ok(status)
}

#[tauri::command]
pub async fn disconnect_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.tunnels.stop_tunnel(&id).await;

    let mut connections = state.connections.write().await;

    let connection = connections
        .remove(&id)
        .ok_or_else(|| format!("No active connection found for server '{}'", id))?;

    let disconnect_result = connection.disconnect().await;

    if let Err(e) = disconnect_result {
        eprintln!(
            "Warning: Error during disconnect cleanup for '{}': {}",
            id, e
        );
    }

    state.cache.remove_all(&id).await;

    Ok(())
}

#[tauri::command]
pub async fn get_connection_status(
    id: String,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let connections = state.connections.read().await;
    let is_connected = connections.contains_key(&id);

    Ok(ConnectionStatus {
        is_connected,
        server_version: None,
        current_database: None,
        current_user: None,
        metadata: Default::default(),
    })
}

/// Get connection quality score and health metrics for a connection.
///
/// Returns quality data including latency, error count, and a composite score (0-100).
/// Returns an error if no health data exists for the given connection.
#[tauri::command]
pub async fn get_connection_quality(connection_id: String) -> Result<crate::connection::guardian::ConnectionQuality, String> {
    let guardian = crate::GUARDIAN
        .get()
        .ok_or_else(|| "Guardian not initialized".to_string())?;
    guardian
        .quality_score(&connection_id)
        .await
        .ok_or_else(|| format!("No quality data for connection '{}'", connection_id))
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
