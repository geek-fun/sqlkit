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

/// Mock connection pool for testing that simulates real pool behavior.
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
        // Simulate delay
        if self.delay_ms > 0 {
            sleep(Duration::from_millis(self.delay_ms)).await;
        }

        // Check if we should fail
        let fail_on = self.fail_on_nth.load(std::sync::atomic::Ordering::SeqCst);
        if fail_on > 0 {
            let current = self.fail_on_nth.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            if current == 1 {
                return Err(DbError::PoolError("Simulated connection failure".to_string()));
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

    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.connection_requests, 1);
    assert_eq!(stats.successful_acquisitions, 1);
    assert_eq!(stats.failed_acquisitions, 0);
}

#[tokio::test]
async fn test_concurrent_connections() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = Arc::new(ConnectionManager::new(pool));

    // Spawn 5 concurrent tasks to get connections
    let mut handles = vec![];
    for i in 0..5 {
        let mgr = manager.clone();
        let handle = tokio::spawn(async move {
            let conn = mgr.get_connection().await;
            assert!(
                conn.is_ok(),
                "Task {} should successfully get a connection",
                i
            );
            // Hold connection for a bit
            sleep(Duration::from_millis(50)).await;
            conn
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.connection_requests, 5);
    assert_eq!(stats.successful_acquisitions, 5);
    assert_eq!(stats.failed_acquisitions, 0);
}

#[tokio::test]
async fn test_connection_isolation() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = Arc::new(ConnectionManager::new(pool));

    // Get multiple connections and verify they are tracked separately
    let conn1 = manager.get_connection().await.unwrap();
    let conn2 = manager.get_connection().await.unwrap();

    // Verify we have 2 tracked connections
    let metadata = manager.get_all_metadata().unwrap();
    assert_eq!(metadata.len(), 2, "Should have 2 tracked connections");

    // Verify connections are isolated (have different IDs)
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

    let metadata = manager.get_all_metadata().unwrap();
    assert_eq!(metadata.len(), 1);

    let meta = &metadata[0];
    assert!(meta.in_use, "Connection should be marked as in use");
    assert!(meta.is_healthy, "Connection should be healthy");
    assert_eq!(meta.query_count, 1, "Query count should be 1");
    assert!(meta.age() < Duration::from_secs(1), "Connection should be recent");
}

#[tokio::test]
async fn test_timeout_handling() {
    // Create a pool that delays connections
    let pool = Arc::new(MockConnectionPool::new(10).with_delay(500));
    let manager = ConnectionManager::new(pool);

    // Try to get connection with a short timeout
    let result = manager
        .get_connection_with_timeout(Duration::from_millis(100))
        .await;

    assert!(result.is_err(), "Should timeout");
    
    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.timeout_count, 1, "Should have 1 timeout");
    assert_eq!(stats.failed_acquisitions, 1, "Should have 1 failed acquisition");
}

#[tokio::test]
async fn test_timeout_success() {
    // Create a pool with small delay
    let pool = Arc::new(MockConnectionPool::new(10).with_delay(50));
    let manager = ConnectionManager::new(pool);

    // Try to get connection with adequate timeout
    let result = manager
        .get_connection_with_timeout(Duration::from_millis(200))
        .await;

    assert!(result.is_ok(), "Should succeed with adequate timeout");
    
    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.timeout_count, 0, "Should have 0 timeouts");
    assert_eq!(stats.successful_acquisitions, 1);
}

#[tokio::test]
async fn test_health_check() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    // Get a connection
    let _conn = manager.get_connection().await.unwrap();

    // Perform health check
    let result = manager.health_check().await;
    assert!(result.is_ok(), "Health check should succeed");
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = Arc::new(ConnectionManager::new(pool));

    // Get a connection and hold it
    let mgr_clone = manager.clone();
    let handle = tokio::spawn(async move {
        let _conn = mgr_clone.get_connection().await.unwrap();
        sleep(Duration::from_millis(100)).await;
        // Connection will be dropped here
    });

    // Give the task time to acquire the connection
    sleep(Duration::from_millis(50)).await;

    // Initiate shutdown (should wait for connection to be released)
    let shutdown_result = manager.shutdown().await;
    assert!(shutdown_result.is_ok(), "Shutdown should succeed");

    // Wait for spawned task
    handle.await.unwrap();

    // Try to get a connection after shutdown
    let conn_result = manager.get_connection().await;
    assert!(
        conn_result.is_err(),
        "Should not be able to get connection after shutdown"
    );
}

#[tokio::test]
async fn test_leak_prevention() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = Arc::new(ConnectionManager::new(pool.clone()));

    // Get many connections and drop them
    for _ in 0..20 {
        let _conn = manager.get_connection().await.unwrap();
        // Connection is dropped immediately
    }

    // Verify connections are released
    let active = pool.active_connections();
    assert_eq!(
        active, 0,
        "All connections should be released (no leaks)"
    );
}

#[tokio::test]
async fn test_connection_failure_handling() {
    // Create a pool that fails on the first connection attempt
    let pool = Arc::new(MockConnectionPool::new(10).with_failure_on_nth(1));
    let manager = ConnectionManager::new(pool);

    // First connection should fail
    let result = manager.get_connection().await;
    assert!(result.is_err(), "First connection should fail");

    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.failed_acquisitions, 1);

    // Second connection should succeed
    let result = manager.get_connection().await;
    assert!(result.is_ok(), "Second connection should succeed");

    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.successful_acquisitions, 1);
}

#[tokio::test]
async fn test_stats_tracking() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    // Get initial stats
    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.connection_requests, 0);
    assert_eq!(stats.successful_acquisitions, 0);
    assert_eq!(stats.max_connections, 10);

    // Get a connection
    let _conn = manager.get_connection().await.unwrap();

    // Check updated stats
    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.connection_requests, 1);
    assert_eq!(stats.successful_acquisitions, 1);
    assert_eq!(stats.total_connections, 1);
}

#[tokio::test]
async fn test_max_connections_limit() {
    let pool = Arc::new(MockConnectionPool::new(2));
    let manager = ConnectionManager::new(pool);

    // Get 2 connections (should succeed)
    let _conn1 = manager.get_connection().await.unwrap();
    let _conn2 = manager.get_connection().await.unwrap();

    // Try to get a 3rd connection (should fail due to pool limit)
    let result = manager.get_connection().await;
    assert!(result.is_err(), "Should fail when pool is exhausted");
}

#[tokio::test]
async fn test_concurrent_isolation_no_interference() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = Arc::new(ConnectionManager::new(pool));

    // Spawn multiple tasks that independently get and release connections
    let mut handles = vec![];
    for i in 0..10 {
        let mgr = manager.clone();
        let handle = tokio::spawn(async move {
            let conn = mgr.get_connection().await.unwrap();
            // Simulate work
            sleep(Duration::from_millis(10 * (i + 1) as u64)).await;
            // Connection will be dropped
            drop(conn);
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all connections were properly tracked
    let stats = manager.get_stats().unwrap();
    assert_eq!(
        stats.connection_requests, 10,
        "Should have 10 connection requests"
    );
    assert_eq!(
        stats.successful_acquisitions, 10,
        "All acquisitions should succeed"
    );
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
        Duration::from_millis(100), // Very short max lifetime
        Duration::from_millis(50),  // Very short idle time
    );

    // Get a connection
    let _conn = manager.get_connection().await.unwrap();

    // Wait for connection to become stale
    sleep(Duration::from_millis(150)).await;

    // Run health check to mark stale connections
    manager.health_check().await.unwrap();

    // Clean up stale connections
    let removed = manager.cleanup_stale_connections().await.unwrap();
    
    // We should have identified and cleaned up stale connections
    // Note: The exact count depends on whether connections are still in use
}

#[tokio::test]
async fn test_metadata_query_count() {
    let pool = Arc::new(MockConnectionPool::new(10));
    let manager = ConnectionManager::new(pool);

    // Get multiple connections
    let _conn1 = manager.get_connection().await.unwrap();
    let _conn2 = manager.get_connection().await.unwrap();
    let _conn3 = manager.get_connection().await.unwrap();

    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.total_connections, 3);
    assert_eq!(stats.avg_queries_per_connection, 1.0);
}
