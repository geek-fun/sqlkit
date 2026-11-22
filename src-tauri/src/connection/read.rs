use crate::connection::handle::ConnectionHandle;
use crate::database::error::{DbError, DbResult};
use crate::state::{ActiveConnection, AppState};
use std::future::Future;

/// Retry a table read once after replacing a failed connection. Never use for writes.
pub(crate) async fn execute_read<T, F, Fut>(
    state: &AppState,
    connection_id: &str,
    operation: F,
) -> Result<T, String>
where
    F: Fn(ActiveConnection) -> Fut,
    Fut: Future<Output = DbResult<T>>,
{
    let mut connection = state.ensure_connection(connection_id).await?;
    let start = std::time::Instant::now();
    let result = match read_once(connection.clone(), &operation).await {
        Err(error) if error.is_connection_failure() => {
            connection = state
                .reconnect_connection(connection_id, &connection)
                .await?;
            read_once(connection.clone(), &operation).await
        }
        result => result,
    };

    let gate = state.cache.reconnect_gate(connection_id).await;
    let _guard = gate.lock().await;
    let is_current = state
        .connections
        .read()
        .await
        .get(connection_id)
        .is_some_and(|current| current.same_session(&connection));
    if let Some(guardian) = crate::GUARDIAN.get().filter(|_| is_current) {
        let latency = Some(start.elapsed().as_secs_f64() * 1000.0);
        match &result {
            Ok(_) => guardian.mark_healthy(connection_id, latency).await,
            Err(error) if error.is_connection_failure() => {
                guardian.mark_dead(connection_id, &error.to_string()).await;
            }
            _ => {}
        }
    }
    result.map_err(|error| error.to_string())
}

async fn read_once<T, F, Fut>(connection: ActiveConnection, operation: &F) -> DbResult<T>
where
    F: Fn(ActiveConnection) -> Fut,
    Fut: Future<Output = DbResult<T>>,
{
    let timeout = connection.query_timeout_secs().await;
    tokio::time::timeout(
        std::time::Duration::from_secs(timeout),
        operation(connection),
    )
    .await
    .map_err(|_| DbError::Timeout(format!("Table read timed out after {} seconds", timeout)))?
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::connection::cache::PoolKey;
    use crate::database::QueryValue;
    use crate::state::ServerConfig;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub(crate) async fn fixture() -> (tempfile::TempDir, AppState, ActiveConnection) {
        let directory = tempfile::tempdir().unwrap();
        let state = AppState::new();
        let mut config = ServerConfig::new(
            "reconnection test".into(),
            "sqlite".into(),
            directory
                .path()
                .join("test.sqlite")
                .to_string_lossy()
                .into(),
            0,
            String::new(),
        );
        config.id = "test".into();
        state
            .configs
            .write()
            .await
            .insert(config.id.clone(), config);
        let connection = state.ensure_connection("test").await.unwrap();
        connection
            .execute_query("CREATE TABLE items (id INTEGER PRIMARY KEY)")
            .await
            .unwrap();
        connection
            .execute_query("INSERT INTO items VALUES (1), (2), (3)")
            .await
            .unwrap();
        (directory, state, connection)
    }

    #[tokio::test]
    async fn closed_table_session_is_replaced_and_cached_handles_are_invalidated() {
        let (_directory, state, stale) = fixture().await;
        state
            .cache
            .get_or_create(&PoolKey::new("test", None), &state)
            .await
            .unwrap();
        stale.disconnect().await.unwrap();

        let result = execute_read(&state, "test", |connection| async move {
            connection
                .execute_query("SELECT id FROM items WHERE id > 1 LIMIT 1 OFFSET 1")
                .await
        })
        .await
        .unwrap();

        assert_eq!(result.rows[0]["id"], QueryValue::Int(3));
        let replacement = state.ensure_connection("test").await.unwrap();
        assert!(!replacement.same_session(&stale));
        assert!(state.cache.is_empty().await);
    }

    #[tokio::test]
    async fn concurrent_table_reads_share_one_replacement() {
        let (_directory, state, stale) = fixture().await;
        stale.disconnect().await.unwrap();
        let barrier = tokio::sync::Barrier::new(2);
        let operation = |connection: ActiveConnection| {
            let barrier = &barrier;
            let stale = &stale;
            async move {
                if connection.same_session(stale) {
                    barrier.wait().await;
                    return Err(DbError::Connection("connection closed".into()));
                }
                let result = connection
                    .execute_query("SELECT COUNT(*) FROM items")
                    .await?;
                Ok((connection, result))
            }
        };
        let (data, count) = tokio::join!(
            execute_read(&state, "test", operation),
            execute_read(&state, "test", operation),
        );
        let (data, count) = (data.unwrap(), count.unwrap());
        assert!(data.0.same_session(&count.0));
        assert!(!data.0.same_session(&stale));
        assert_eq!(data.1.rows.len(), 1);
    }

    #[tokio::test]
    async fn concurrent_missing_connections_share_one_session() {
        let (_directory, state, stale) = fixture().await;
        state.connections.write().await.remove("test");
        stale.disconnect().await.unwrap();
        let (first, second) = tokio::join!(
            state.ensure_connection("test"),
            state.ensure_connection("test")
        );
        assert!(first.unwrap().same_session(&second.unwrap()));
    }

    #[tokio::test]
    async fn query_errors_and_timeouts_are_not_retried() {
        let (_directory, state, original) = fixture().await;
        let attempts = AtomicUsize::new(0);
        for timeout in [false, true] {
            let result = execute_read(&state, "test", |_| {
                attempts.fetch_add(1, Ordering::SeqCst);
                async move {
                    if timeout {
                        Err::<(), _>(DbError::Timeout("statement timeout".into()))
                    } else {
                        Err(DbError::InvalidQuery("invalid column".into()))
                    }
                }
            })
            .await;
            assert!(result.is_err());
        }
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert!(state
            .ensure_connection("test")
            .await
            .unwrap()
            .same_session(&original));
    }

    #[tokio::test]
    async fn a_second_connection_failure_is_returned_without_another_retry() {
        let (_directory, state, _) = fixture().await;
        let attempts = AtomicUsize::new(0);
        let result = execute_read(&state, "test", |_| {
            attempts.fetch_add(1, Ordering::SeqCst);
            async { Err::<(), _>(DbError::Connection("connection closed".into())) }
        })
        .await;
        assert!(result.unwrap_err().contains("connection closed"));
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn removed_configuration_is_not_resurrected_after_a_failed_read() {
        let (_directory, state, stale) = fixture().await;
        stale.disconnect().await.unwrap();
        state.configs.write().await.remove("test");
        let result = execute_read(&state, "test", |connection| async move {
            connection.execute_query("SELECT * FROM items").await
        })
        .await;
        assert!(result.unwrap_err().contains("No active connection found"));
        assert!(state.connections.read().await.is_empty());
    }
}
