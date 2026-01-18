//! Connection pooling interface.
//!
//! This module defines the interface for connection pooling that can be
//! implemented by different database adapters.

use crate::database::error::DbResult;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for a connection pool.
///
/// This trait defines the interface for managing a pool of database connections.
/// Implementations should handle connection lifecycle, reuse, and resource management.
#[async_trait]
pub trait ConnectionPool: Send + Sync {
    /// Type representing a single connection from the pool.
    type Connection: Send;

    /// Get a connection from the pool.
    ///
    /// This method should wait for an available connection if the pool is at capacity,
    /// up to the configured timeout.
    ///
    /// # Returns
    ///
    /// A connection wrapped in an Arc, or an error if no connection is available
    /// within the timeout period.
    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>>;

    /// Return a connection to the pool.
    ///
    /// This method is called when a connection is no longer needed.
    /// The pool implementation should handle validation and potential cleanup
    /// of the connection before making it available for reuse.
    ///
    /// # Arguments
    ///
    /// * `connection` - The connection to return to the pool
    async fn return_connection(&self, connection: Arc<Self::Connection>) -> DbResult<()>;

    /// Get the current number of active connections in the pool.
    fn active_connections(&self) -> usize;

    /// Get the current number of idle connections in the pool.
    fn idle_connections(&self) -> usize;

    /// Get the maximum number of connections allowed in the pool.
    fn max_connections(&self) -> usize;

    /// Close all connections in the pool.
    ///
    /// This method should gracefully close all active and idle connections
    /// and prevent new connections from being created.
    async fn close(&self) -> DbResult<()>;

    /// Check the health of the pool.
    ///
    /// This method should verify that the pool is operational and can provide
    /// connections. It may perform cleanup of stale connections.
    async fn health_check(&self) -> DbResult<()>;
}

/// A simple connection pool statistics structure.
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of connections (active + idle).
    pub total_connections: usize,
    /// Number of currently active connections.
    pub active_connections: usize,
    /// Number of currently idle connections.
    pub idle_connections: usize,
    /// Maximum allowed connections.
    pub max_connections: usize,
    /// Number of times a connection was requested.
    pub connection_requests: u64,
    /// Number of times a connection request waited.
    pub wait_count: u64,
    /// Average wait time in milliseconds.
    pub average_wait_time_ms: u64,
}

impl PoolStats {
    /// Create new pool statistics.
    pub fn new(max_connections: usize) -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            idle_connections: 0,
            max_connections,
            connection_requests: 0,
            wait_count: 0,
            average_wait_time_ms: 0,
        }
    }
}
