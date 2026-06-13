//! JDBC bridge connection pool stub.
//!
//! Connection pooling is handled by the Java side (HikariCP).
//! This pool is a pass-through stub that satisfies the crate's `ConnectionPool` trait.

use crate::database::error::{DbError, DbResult};
use crate::database::pool::ConnectionPool;
use async_trait::async_trait;
use std::sync::Arc;

/// Dummy connection type for the JDBC bridge.
///
/// Real connections are managed inside the Java process.
pub struct JdbcBridgeConnection;

unsafe impl Send for JdbcBridgeConnection {}
unsafe impl Sync for JdbcBridgeConnection {}

/// JDBC bridge connection pool stub.
///
/// All connection management happens in the Java subprocess (HikariCP pool).
/// This stub exists only to satisfy the `ConnectionPool` trait bound.
pub struct JdbcBridgePool;

#[async_trait]
impl ConnectionPool for JdbcBridgePool {
    type Connection = JdbcBridgeConnection;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        Err(DbError::UnsupportedOperation(
            "JDBC bridge connections are managed by the Java process".to_string(),
        ))
    }

    async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
        Ok(())
    }

    fn active_connections(&self) -> usize {
        0
    }

    fn idle_connections(&self) -> usize {
        0
    }

    fn max_connections(&self) -> usize {
        1
    }

    async fn close(&self) -> DbResult<()> {
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        Ok(())
    }
}
