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
    let gate = state.cache.reconnect_gate(&id).await;
    let _guard = gate.lock().await;
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

    let status = match connection.check_connection().await {
        Ok(status) if status.is_connected => status,
        failed => {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                connection.disconnect(),
            )
            .await;
            return Err(match failed {
                Err(error) => error.to_string(),
                Ok(_) => "Connection check reported a disconnected session".into(),
            });
        }
    };
    state.cache.remove_all(&id).await;
    let previous = state
        .connections
        .write()
        .await
        .insert(id.clone(), connection.clone());
    state.configs.write().await.insert(id.clone(), config);
    if let Some(previous) = previous {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            previous.disconnect(),
        )
        .await;
    }
    state.cache.mark_validated(&id, &connection).await;
    if let Some(guardian) = crate::GUARDIAN.get() {
        guardian.mark_healthy(&id, None).await;
        guardian.touch(&id).await;
    }
    Ok(status)
}

#[tauri::command]
pub async fn disconnect_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let gate = state.cache.reconnect_gate(&id).await;
    let _guard = gate.lock().await;
    state.tunnels.stop_tunnel(&id).await;
    if let Some(guardian) = crate::GUARDIAN.get() {
        guardian.forget(&id).await;
    }

    let connection = state.connections.write().await.remove(&id);
    let timeout = state
        .configs
        .write()
        .await
        .remove(&id)
        .map(|config| config.connect_timeout_secs)
        .unwrap_or(10);
    state.cache.remove_all(&id).await;

    if let Some(connection) = connection {
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            connection.disconnect(),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => log::warn!("Error during disconnect cleanup for '{}': {}", id, error),
            Err(_) => log::warn!("Disconnect cleanup timed out for '{}'", id),
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_connection_status(
    id: String,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    if state.configs.read().await.contains_key(&id)
        || state.connections.read().await.contains_key(&id)
    {
        let connection = state.ensure_connection(&id).await?;
        return connection
            .check_connection()
            .await
            .map_err(|error| error.to_string());
    }
    Ok(ConnectionStatus {
        is_connected: false,
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
pub async fn get_connection_quality(
    connection_id: String,
) -> Result<crate::connection::guardian::ConnectionQuality, String> {
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
