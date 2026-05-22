//! Connection pool manager for lifecycle management and metadata tracking.
//!
//! This module provides the `ConnectionManager` struct that wraps existing pool
//! implementations to add connection tracking, health checks, and graceful shutdown.

use crate::database::{
    error::{DbError, DbResult},
    pool::ConnectionPool,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Metadata for a tracked connection.
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    /// Unique identifier for the connection.
    pub connection_id: String,
    /// When the connection was created.
    pub created_at: Instant,
    /// When the connection was last used.
    pub last_used: Instant,
    /// Number of queries executed on this connection.
    pub query_count: u64,
    /// Whether the connection is currently in use.
    pub in_use: bool,
    /// Whether the connection is healthy.
    pub is_healthy: bool,
}

impl ConnectionMetadata {
    /// Create new connection metadata.
    pub fn new(connection_id: String) -> Self {
        let now = Instant::now();
        Self {
            connection_id,
            created_at: now,
            last_used: now,
            query_count: 0,
            in_use: false,
            is_healthy: true,
        }
    }

    /// Mark the connection as used and increment query count.
    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.query_count += 1;
        self.in_use = true;
    }

    /// Mark the connection as released.
    pub fn mark_released(&mut self) {
        self.in_use = false;
    }

    /// Mark the connection as unhealthy.
    pub fn mark_unhealthy(&mut self) {
        self.is_healthy = false;
    }

    /// Get the age of the connection.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get the idle time of the connection.
    pub fn idle_time(&self) -> Duration {
        self.last_used.elapsed()
    }
}

/// Statistics for the connection manager.
#[derive(Debug, Clone)]
pub struct ManagerStats {
    /// Total number of connections tracked.
    pub total_connections: usize,
    /// Number of connections currently in use.
    pub active_connections: usize,
    /// Number of idle connections.
    pub idle_connections: usize,
    /// Number of unhealthy connections.
    pub unhealthy_connections: usize,
    /// Total number of connection requests.
    pub connection_requests: u64,
    /// Number of successful connection acquisitions.
    pub successful_acquisitions: u64,
    /// Number of failed connection acquisitions.
    pub failed_acquisitions: u64,
    /// Number of timeouts.
    pub timeout_count: u64,
    /// Average query count per connection.
    pub avg_queries_per_connection: f64,
    /// Maximum connections allowed.
    pub max_connections: usize,
}

/// Connection pool manager that wraps a ConnectionPool implementation
/// to provide lifecycle management, tracking, and health monitoring.
pub struct ConnectionManager<P: ConnectionPool> {
    /// The underlying connection pool.
    pool: Arc<P>,
    /// Metadata for tracked connections.
    metadata: Arc<RwLock<HashMap<String, ConnectionMetadata>>>,
    /// Statistics for the manager.
    stats: Arc<RwLock<ManagerStats>>,
    /// Whether the manager is shutting down.
    shutting_down: Arc<RwLock<bool>>,
    /// Connection acquisition timeout.
    connection_timeout: Duration,
    /// Maximum lifetime for a connection before it's considered stale.
    max_lifetime: Duration,
    /// Maximum idle time before a connection is considered stale.
    max_idle_time: Duration,
}

impl<P: ConnectionPool> ConnectionManager<P> {
    /// Create a new connection manager wrapping a pool.
    pub fn new(pool: Arc<P>) -> Self {
        let max_connections = pool.max_connections();
        Self {
            pool,
            metadata: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ManagerStats {
                total_connections: 0,
                active_connections: 0,
                idle_connections: 0,
                unhealthy_connections: 0,
                connection_requests: 0,
                successful_acquisitions: 0,
                failed_acquisitions: 0,
                timeout_count: 0,
                avg_queries_per_connection: 0.0,
                max_connections,
            })),
            shutting_down: Arc::new(RwLock::new(false)),
            connection_timeout: Duration::from_secs(30),
            max_lifetime: Duration::from_secs(1800),
            max_idle_time: Duration::from_secs(600),
        }
    }

    /// Create a new connection manager with custom timeouts.
    pub fn with_timeouts(
        pool: Arc<P>,
        connection_timeout: Duration,
        max_lifetime: Duration,
        max_idle_time: Duration,
    ) -> Self {
        let mut manager = Self::new(pool);
        manager.connection_timeout = connection_timeout;
        manager.max_lifetime = max_lifetime;
        manager.max_idle_time = max_idle_time;
        manager
    }

    /// Get a connection ID for tracking.
    fn generate_connection_id(&self) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("conn_{}", id)
    }

    /// Track a new connection.
    async fn track_connection(&self, connection_id: String) {
        let mut metadata = self.metadata.write().await;
        metadata.insert(
            connection_id.clone(),
            ConnectionMetadata::new(connection_id),
        );

        let mut stats = self.stats.write().await;
        stats.total_connections = metadata.len();
    }

    /// Mark a connection as used.
    async fn mark_connection_used(&self, connection_id: &str) {
        let mut metadata = self.metadata.write().await;
        if let Some(meta) = metadata.get_mut(connection_id) {
            meta.mark_used();
        }
        self.update_stats_from_metadata(&metadata).await;
    }

    async fn update_stats_from_metadata(&self, metadata: &HashMap<String, ConnectionMetadata>) {
        let mut stats = self.stats.write().await;
        stats.active_connections = metadata.values().filter(|m| m.in_use).count();
        stats.idle_connections = metadata
            .values()
            .filter(|m| !m.in_use && m.is_healthy)
            .count();
        stats.unhealthy_connections = metadata.values().filter(|m| !m.is_healthy).count();

        let total_queries: u64 = metadata.values().map(|m| m.query_count).sum();
        stats.avg_queries_per_connection = if metadata.is_empty() {
            0.0
        } else {
            total_queries as f64 / metadata.len() as f64
        };
    }

    /// Get connection with timeout support.
    pub async fn get_connection_with_timeout(
        &self,
        timeout: Duration,
    ) -> DbResult<Arc<P::Connection>> {
        {
            let shutting_down = self.shutting_down.read().await;
            if *shutting_down {
                return Err(DbError::PoolError("Manager is shutting down".to_string()));
            }
        }

        {
            let mut stats = self.stats.write().await;
            stats.connection_requests += 1;
        }

        let result = tokio::time::timeout(timeout, self.pool.get_connection()).await;

        match result {
            Ok(Ok(conn)) => {
                let conn_id = self.generate_connection_id();
                self.track_connection(conn_id.clone()).await;
                self.mark_connection_used(&conn_id).await;

                let mut stats = self.stats.write().await;
                stats.successful_acquisitions += 1;

                Ok(conn)
            }
            Ok(Err(e)) => {
                let mut stats = self.stats.write().await;
                stats.failed_acquisitions += 1;
                Err(e)
            }
            Err(_) => {
                let mut stats = self.stats.write().await;
                stats.timeout_count += 1;
                stats.failed_acquisitions += 1;
                Err(DbError::Timeout(format!(
                    "Connection acquisition timed out after {:?}",
                    timeout
                )))
            }
        }
    }

    /// Get connection using the default timeout.
    pub async fn get_connection(&self) -> DbResult<Arc<P::Connection>> {
        self.get_connection_with_timeout(self.connection_timeout)
            .await
    }

    /// Perform health check on all connections.
    pub async fn health_check(&self) -> DbResult<()> {
        self.pool.health_check().await?;

        let unhealthy_ids = {
            let metadata = self.metadata.read().await;
            metadata
                .iter()
                .filter(|(_, meta)| {
                    meta.age() > self.max_lifetime
                        || (!meta.in_use && meta.idle_time() > self.max_idle_time)
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        };

        if !unhealthy_ids.is_empty() {
            let mut metadata = self.metadata.write().await;
            for id in unhealthy_ids {
                if let Some(meta) = metadata.get_mut(&id) {
                    meta.mark_unhealthy();
                }
            }
            self.update_stats_from_metadata(&metadata).await;
        }

        Ok(())
    }

    /// Clean up stale and unhealthy connections.
    pub async fn cleanup_stale_connections(&self) -> DbResult<usize> {
        let mut metadata = self.metadata.write().await;
        let initial_count = metadata.len();
        metadata.retain(|_, meta| meta.is_healthy || meta.in_use);
        let removed_count = initial_count - metadata.len();
        self.update_stats_from_metadata(&metadata).await;
        Ok(removed_count)
    }

    /// Get current statistics.
    pub async fn get_stats(&self) -> ManagerStats {
        self.stats.read().await.clone()
    }

    /// Get connection metadata for a specific connection.
    pub async fn get_connection_metadata(&self, connection_id: &str) -> Option<ConnectionMetadata> {
        self.metadata.read().await.get(connection_id).cloned()
    }

    /// Get all connection metadata.
    pub async fn get_all_metadata(&self) -> Vec<ConnectionMetadata> {
        self.metadata.read().await.values().cloned().collect()
    }

    /// Get the number of active connections.
    pub fn active_connections(&self) -> usize {
        self.pool.active_connections()
    }

    /// Get the number of idle connections.
    pub fn idle_connections(&self) -> usize {
        self.pool.idle_connections()
    }

    /// Get the maximum number of connections.
    pub fn max_connections(&self) -> usize {
        self.pool.max_connections()
    }

    /// Initiate graceful shutdown.
    pub async fn shutdown(&self) -> DbResult<()> {
        {
            let mut shutting_down = self.shutting_down.write().await;
            *shutting_down = true;
        }

        let shutdown_timeout = Duration::from_secs(30);
        let start = Instant::now();

        loop {
            let active_count = {
                let metadata = self.metadata.read().await;
                metadata.values().filter(|m| m.in_use).count()
            };

            if active_count == 0 {
                break;
            }

            if start.elapsed() > shutdown_timeout {
                return Err(DbError::Timeout(format!(
                    "Shutdown timed out waiting for {} active connections",
                    active_count
                )));
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        self.pool.close().await?;

        {
            let mut metadata = self.metadata.write().await;
            metadata.clear();
        }

        Ok(())
    }

    /// Get the underlying pool.
    pub fn pool(&self) -> &Arc<P> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPool {
        max_connections: usize,
    }

    #[async_trait::async_trait]
    impl ConnectionPool for MockPool {
        type Connection = String;

        async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
            Ok(Arc::new("mock_connection".to_string()))
        }

        async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
            Ok(())
        }

        fn active_connections(&self) -> usize {
            0
        }

        fn idle_connections(&self) -> usize {
            5
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

    #[test]
    fn test_connection_metadata_creation() {
        let meta = ConnectionMetadata::new("test_conn".to_string());
        assert_eq!(meta.connection_id, "test_conn");
        assert_eq!(meta.query_count, 0);
        assert!(!meta.in_use);
        assert!(meta.is_healthy);
    }

    #[test]
    fn test_connection_metadata_usage() {
        let mut meta = ConnectionMetadata::new("test_conn".to_string());
        meta.mark_used();
        assert_eq!(meta.query_count, 1);
        assert!(meta.in_use);

        meta.mark_released();
        assert!(!meta.in_use);
        assert_eq!(meta.query_count, 1);
    }

    #[test]
    fn test_connection_metadata_health() {
        let mut meta = ConnectionMetadata::new("test_conn".to_string());
        assert!(meta.is_healthy);

        meta.mark_unhealthy();
        assert!(!meta.is_healthy);
    }

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let pool = Arc::new(MockPool {
            max_connections: 10,
        });
        let manager = ConnectionManager::new(pool);
        assert_eq!(manager.max_connections(), 10);
    }

    #[tokio::test]
    async fn test_connection_manager_get_connection() {
        let pool = Arc::new(MockPool {
            max_connections: 10,
        });
        let manager = ConnectionManager::new(pool);

        let conn = manager.get_connection().await;
        assert!(conn.is_ok());

        let stats = manager.get_stats().await;
        assert_eq!(stats.connection_requests, 1);
        assert_eq!(stats.successful_acquisitions, 1);
    }

    #[tokio::test]
    async fn test_connection_manager_stats() {
        let pool = Arc::new(MockPool {
            max_connections: 10,
        });
        let manager = ConnectionManager::new(pool);

        let _conn = manager.get_connection().await.unwrap();

        let stats = manager.get_stats().await;
        assert_eq!(stats.connection_requests, 1);
        assert_eq!(stats.successful_acquisitions, 1);
        assert_eq!(stats.failed_acquisitions, 0);
        assert_eq!(stats.max_connections, 10);
    }

    #[tokio::test]
    async fn test_connection_manager_health_check() {
        let pool = Arc::new(MockPool {
            max_connections: 10,
        });
        let manager = ConnectionManager::new(pool);

        let result = manager.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connection_manager_with_custom_timeouts() {
        let pool = Arc::new(MockPool {
            max_connections: 10,
        });
        let manager = ConnectionManager::with_timeouts(
            pool,
            Duration::from_secs(5),
            Duration::from_secs(300),
            Duration::from_secs(60),
        );

        assert_eq!(manager.connection_timeout, Duration::from_secs(5));
        assert_eq!(manager.max_lifetime, Duration::from_secs(300));
        assert_eq!(manager.max_idle_time, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_connection_manager_metadata_tracking() {
        let pool = Arc::new(MockPool {
            max_connections: 10,
        });
        let manager = ConnectionManager::new(pool);

        let _conn = manager.get_connection().await.unwrap();

        let all_metadata = manager.get_all_metadata().await;
        assert_eq!(all_metadata.len(), 1);
        assert!(all_metadata[0].in_use);
        assert_eq!(all_metadata[0].query_count, 1);
    }

    #[tokio::test]
    async fn test_connection_manager_shutdown() {
        let pool = Arc::new(MockPool {
            max_connections: 10,
        });
        let manager = ConnectionManager::new(pool);

        let result = manager.shutdown().await;
        assert!(result.is_ok());

        let conn_result = manager.get_connection().await;
        assert!(conn_result.is_err());
    }
}
