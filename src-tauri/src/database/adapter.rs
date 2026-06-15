//! Database adapter trait definition.
//!
//! This module defines the core `DatabaseAdapter` trait that all database adapters
//! must implement to provide a consistent interface for database operations.

use crate::database::{
    config::ConnectionConfig,
    error::{DbError, DbResult},
    pool::ConnectionPool,
    types::{ColumnInfo, ConnectionStatus, DatabaseSchema, ForeignKeyInfo, QueryResult, TableInfo},
};
use async_trait::async_trait;
use std::sync::Arc;

/// Core database adapter trait.
///
/// This trait defines the interface that all database adapters must implement.
/// It provides methods for connection management, query execution, and metadata retrieval.
///
/// # Example
///
/// ```ignore
/// use sqlkit::database::{DatabaseAdapter, ConnectionConfig};
///
/// async fn example(adapter: impl DatabaseAdapter) {
///     // Connect to the database
///     adapter.connect().await.unwrap();
///     
///     // Test the connection
///     let status = adapter.test_connection().await.unwrap();
///     println!("Connected: {}", status.is_connected);
///     
///     // Execute a query
///     let result = adapter.execute_query("SELECT * FROM users").await.unwrap();
///     println!("Found {} rows", result.rows.len());
///     
///     // Disconnect
///     adapter.disconnect().await.unwrap();
/// }
/// ```
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// Type of connection pool used by this adapter.
    type Pool: ConnectionPool;

    /// Connect to the database.
    ///
    /// This method establishes a connection to the database using the provided
    /// configuration. It may also initialize a connection pool.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the connection was successful, or an error if the connection failed.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The database server is unreachable
    /// - Authentication fails
    /// - The database does not exist
    /// - Connection configuration is invalid
    async fn connect(&mut self) -> DbResult<()>;

    /// Disconnect from the database.
    ///
    /// This method closes the database connection and cleans up any resources,
    /// including closing all connections in the pool.
    ///
    /// # Returns
    ///
    /// `Ok(())` if disconnection was successful, or an error if cleanup failed.
    async fn disconnect(&mut self) -> DbResult<()>;

    /// Test the database connection.
    ///
    /// This method verifies that the database connection is active and functional
    /// by executing a simple query or ping operation.
    ///
    /// # Returns
    ///
    /// Connection status information including server version and current database.
    ///
    /// # Errors
    ///
    /// This method will return an error if the connection is not active or the
    /// test query fails.
    async fn test_connection(&self) -> DbResult<ConnectionStatus>;

    /// Execute a SQL query.
    ///
    /// This method executes a SQL query and returns the result. It supports both
    /// SELECT queries (which return rows) and DML statements like INSERT, UPDATE,
    /// DELETE (which return the number of affected rows).
    ///
    /// # Arguments
    ///
    /// * `query` - The SQL query to execute
    ///
    /// # Returns
    ///
    /// A `QueryResult` containing columns, rows, and/or affected row count.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The query syntax is invalid
    /// - The query execution fails
    /// - A timeout occurs
    /// - The connection is not active
    async fn execute_query(&self, query: &str) -> DbResult<QueryResult>;

    /// List all databases on the server.
    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        Err(DbError::unsupported("list_databases"))
    }

    /// List all schemas in a database.
    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        Err(DbError::unsupported("list_schemas"))
    }

    /// List all tables in a schema.
    async fn list_tables(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        Err(DbError::unsupported("list_tables"))
    }

    /// List all columns in a table.
    async fn list_columns(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
        _table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        Err(DbError::unsupported("list_columns"))
    }

    /// Get detailed information about a table.
    async fn get_table_info(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
        _table: &str,
    ) -> DbResult<TableInfo> {
        Err(DbError::unsupported("get_table_info"))
    }

    /// Get foreign key relationships for tables in a schema.
    async fn get_foreign_keys(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
    ) -> DbResult<Vec<ForeignKeyInfo>> {
        Err(DbError::unsupported("get_foreign_keys"))
    }

    /// Get the connection pool.
    ///
    /// This method returns a reference to the connection pool used by this adapter.
    /// The pool can be used to manage connections and retrieve pool statistics.
    ///
    /// # Returns
    ///
    /// An optional reference to the connection pool. Returns None if pooling is
    /// not enabled or supported.
    fn get_pool(&self) -> Option<Arc<Self::Pool>>;

    /// Get the connection configuration.
    ///
    /// This method returns a reference to the connection configuration used by
    /// this adapter.
    ///
    /// # Returns
    ///
    /// A reference to the connection configuration.
    fn get_config(&self) -> &ConnectionConfig;
}
