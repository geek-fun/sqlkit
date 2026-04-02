//! Integration tests for ConnectionManager.
//!
//! These tests verify the connection manager functionality including:
//! - Concurrent connection handling
//! - Connection metadata tracking
//! - Health checks
//! - Graceful shutdown
//! - Timeout handling
//! - Leak prevention

use sqlkit_lib::database::{ConnectionManager, ConnectionPool, DbError, DbResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

struct MockConnectionPool {
    max_connections: usize,
    active: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    fail_on_nth: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    delay_ms: u64,
}

impl MockConnectionPool {
    fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            active: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            fail_on_nth: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            delay_ms: 0,
        }
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    fn with_failure_on_nth(self, n: usize) -> Self {
        self.fail_on_nth
            .store(n, std::sync::atomic::Ordering::SeqCst);
        self
    }
}

#[async_trait::async_trait]
impl ConnectionPool for MockConnectionPool {
    type Connection = String;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        if self.delay_ms > 0 {
            sleep(Duration::from_millis(self.delay_ms)).await;
        }

        let fail_on = self.fail_on_nth.load(std::sync::atomic::Ordering::SeqCst);
        if fail_on > 0 {
            let current = self
                .fail_on_nth
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            if current == 1 {
                return Err(DbError::PoolError(
                    "Simulated connection failure".to_string(),
                ));
            }
        }

        let active = self
            .active
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if active >= self.max_connections {
            self.active
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            return Err(DbError::PoolError("Pool exhausted".to_string()));
        }

        Ok(Arc::new(format!("connection_{}", active)))
    }

    async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
        self.active
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn active_connections(&self) -> usize {
        self.active.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn idle_connections(&self) -> usize {
        self.max_connections - self.active_connections()
    }

    fn max_connections(&self) -> usize {
        self.max_connections
    }

    async fn close(&self) -> DbResult<()> {
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_single_connection() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    let conn = manager.get_connection().await;
    assert!(conn.is_ok(), "Should successfully get a connection");

    let stats = manager.get_stats().await;
    assert_eq!(stats.connection_requests, 1);
    assert_eq!(stats.successful_acquisitions, 1);
    assert_eq!(stats.failed_acquisitions, 0);
}

#[tokio::test]
async fn test_connection_isolation() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    let _conn1 = manager.get_connection().await.unwrap();
    let _conn2 = manager.get_connection().await.unwrap();

    let metadata = manager.get_all_metadata().await;
    assert_eq!(metadata.len(), 2, "Should have 2 tracked connections");

    assert_ne!(
        metadata[0].connection_id, metadata[1].connection_id,
        "Connection IDs should be unique"
    );
}

#[tokio::test]
async fn test_connection_metadata_tracking() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    let _conn = manager.get_connection().await.unwrap();

    let metadata = manager.get_all_metadata().await;
    assert_eq!(metadata.len(), 1);

    let meta = &metadata[0];
    assert!(meta.in_use, "Connection should be marked as in use");
    assert!(meta.is_healthy, "Connection should be healthy");
    assert_eq!(meta.query_count, 1, "Query count should be 1");
    assert!(
        meta.age() < Duration::from_secs(1),
        "Connection should be recent"
    );
}

#[tokio::test]
async fn test_timeout_handling() {
    let pool = Arc::new(MockConnectionPool::new(10).with_delay(500));
    let manager = ConnectionManager::new(pool);

    let result = manager
        .get_connection_with_timeout(Duration::from_millis(100))
        .await;

    assert!(result.is_err(), "Should timeout");

    let stats = manager.get_stats().await;
    assert_eq!(stats.timeout_count, 1, "Should have 1 timeout");
    assert_eq!(
        stats.failed_acquisitions, 1,
        "Should have 1 failed acquisition"
    );
}

#[tokio::test]
async fn test_timeout_success() {
    let pool = Arc::new(MockConnectionPool::new(10).with_delay(50));
    let manager = ConnectionManager::new(pool);

    let result = manager
        .get_connection_with_timeout(Duration::from_millis(200))
        .await;

    assert!(result.is_ok(), "Should succeed with adequate timeout");

    let stats = manager.get_stats().await;
    assert_eq!(stats.timeout_count, 0, "Should have 0 timeouts");
    assert_eq!(stats.successful_acquisitions, 1);
}

#[tokio::test]
async fn test_health_check() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    let _conn = manager.get_connection().await.unwrap();

    let result = manager.health_check().await;
    assert!(result.is_ok(), "Health check should succeed");
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    let shutdown_result = manager.shutdown().await;
    assert!(shutdown_result.is_ok(), "Shutdown should succeed");

    let conn_result = manager.get_connection().await;
    assert!(
        conn_result.is_err(),
        "Should not be able to get connection after shutdown"
    );
}

#[tokio::test]
async fn test_leak_prevention() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool.clone());

    {
        let _conn = manager.get_connection().await.unwrap();
        let _conn2 = manager.get_connection().await.unwrap();
        let _conn3 = manager.get_connection().await.unwrap();
    }

    let stats = manager.get_stats().await;
    assert_eq!(
        stats.total_connections, 3,
        "Should have tracked 3 connections"
    );
}

#[tokio::test]
async fn test_connection_failure_handling() {
    let pool = Arc::new(MockConnectionPool::new(10).with_failure_on_nth(1));
    let manager = ConnectionManager::new(pool);

    let result = manager.get_connection().await;
    assert!(result.is_err(), "First connection should fail");

    let stats = manager.get_stats().await;
    assert_eq!(stats.failed_acquisitions, 1);

    let result = manager.get_connection().await;
    assert!(result.is_ok(), "Second connection should succeed");

    let stats = manager.get_stats().await;
    assert_eq!(stats.successful_acquisitions, 1);
}

#[tokio::test]
async fn test_stats_tracking() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    let stats = manager.get_stats().await;
    assert_eq!(stats.connection_requests, 0);
    assert_eq!(stats.successful_acquisitions, 0);
    assert_eq!(stats.max_connections, 10);

    let _conn = manager.get_connection().await.unwrap();

    let stats = manager.get_stats().await;
    assert_eq!(stats.connection_requests, 1);
    assert_eq!(stats.successful_acquisitions, 1);
    assert_eq!(stats.total_connections, 1);
}

#[tokio::test]
async fn test_max_connections_limit() {
    let pool = Arc::new(MockConnectionPool::new(2));
    let manager = ConnectionManager::new(pool);

    let _conn1 = manager.get_connection().await.unwrap();
    let _conn2 = manager.get_connection().await.unwrap();

    let result = manager.get_connection().await;
    assert!(result.is_err(), "Should fail when pool is exhausted");
}

#[tokio::test]
async fn test_custom_timeouts() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::with_timeouts(
        pool,
        Duration::from_secs(1),
        Duration::from_secs(300),
        Duration::from_secs(60),
    );

    let conn = manager.get_connection().await;
    assert!(conn.is_ok(), "Should get connection with custom timeouts");
}

#[tokio::test]
async fn test_stale_connection_cleanup() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::with_timeouts(
        pool,
        Duration::from_secs(1),
        Duration::from_millis(100),
        Duration::from_millis(50),
    );

    let _conn = manager.get_connection().await.unwrap();

    sleep(Duration::from_millis(150)).await;

    manager.health_check().await.unwrap();

    let _removed = manager.cleanup_stale_connections().await.unwrap();
}

#[tokio::test]
async fn test_metadata_query_count() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    let _conn1 = manager.get_connection().await.unwrap();
    let _conn2 = manager.get_connection().await.unwrap();
    let _conn3 = manager.get_connection().await.unwrap();

    let stats = manager.get_stats().await;
    assert_eq!(stats.total_connections, 3);
    assert_eq!(stats.avg_queries_per_connection, 1.0);
}

#[tokio::test]
async fn test_concurrent_connections() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = Arc::new(ConnectionManager::new(pool));

    let mut set = tokio::task::JoinSet::new();
    for _ in 0..5 {
        let mgr = Arc::clone(&manager);
        set.spawn(async move { mgr.get_connection().await });
    }

    let mut success_count = 0;
    while let Some(result) = set.join_next().await {
        assert!(result.is_ok(), "Spawn should not panic");
        assert!(result.unwrap().is_ok(), "Each connection should succeed");
        success_count += 1;
    }

    assert_eq!(success_count, 5, "All 5 tasks should complete");

    let stats = manager.get_stats().await;
    assert_eq!(stats.successful_acquisitions, 5, "All 5 acquisitions should succeed");
    assert_eq!(stats.connection_requests, 5, "Should have 5 connection requests");
}

#[tokio::test]
async fn test_concurrent_isolation_no_interference() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = Arc::new(ConnectionManager::new(pool));

    let mut set = tokio::task::JoinSet::new();
    for _ in 0..3 {
        let mgr = Arc::clone(&manager);
        set.spawn(async move { mgr.get_connection().await });
    }

    while let Some(result) = set.join_next().await {
        assert!(result.unwrap().is_ok(), "Each concurrent connection should succeed");
    }

    let metadata = manager.get_all_metadata().await;
    assert_eq!(metadata.len(), 3, "Should track 3 independent connections");

    let ids: std::collections::HashSet<_> = metadata.iter().map(|m| &m.connection_id).collect();
    assert_eq!(ids.len(), 3, "All connection IDs must be unique");
}
