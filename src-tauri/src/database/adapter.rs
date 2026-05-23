//! Database adapter trait definition.
//!
//! This module defines the core `DatabaseAdapter` trait that all database adapters
//! must implement to provide a consistent interface for database operations.

use crate::database::{
    config::ConnectionConfig,
    error::DbResult,
    pool::ConnectionPool,
    types::{ColumnInfo, ConnectionStatus, DatabaseSchema, QueryResult, TableInfo},
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

    /// Execute a parameterized statement with N rows of M values each.
    /// Implementations MUST bind via the driver's native parameter API.
    async fn execute_batch_with_params(
        &self,
        statement: &str,
        column_count: usize,
        values: Vec<Vec<String>>,
    ) -> DbResult<u64>;

    /// List all databases on the server.
    ///
    /// This method retrieves a list of all databases accessible to the current user.
    ///
    /// # Returns
    ///
    /// A vector of `DatabaseSchema` objects representing available databases.
    ///
    /// # Errors
    ///
    /// This method will return an error if the metadata query fails or if the
    /// operation is not supported by the database.
    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>>;

    /// List all schemas in a database.
    ///
    /// This method retrieves a list of all schemas in the specified database.
    /// For databases that don't support schemas, this may return an empty list
    /// or a single default schema.
    ///
    /// # Arguments
    ///
    /// * `database` - The database name, or None for the current database
    ///
    /// # Returns
    ///
    /// A vector of schema names.
    ///
    /// # Errors
    ///
    /// This method will return an error if the database doesn't exist or if
    /// the metadata query fails.
    async fn list_schemas(&self, database: Option<&str>) -> DbResult<Vec<String>>;

    /// List all tables in a schema.
    ///
    /// This method retrieves a list of all tables (and optionally views) in the
    /// specified schema.
    ///
    /// # Arguments
    ///
    /// * `database` - The database name, or None for the current database
    /// * `schema` - The schema name, or None for the default schema
    ///
    /// # Returns
    ///
    /// A vector of `TableInfo` objects representing available tables.
    ///
    /// # Errors
    ///
    /// This method will return an error if the schema doesn't exist or if
    /// the metadata query fails.
    async fn list_tables(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>>;

    /// List all columns in a table.
    ///
    /// This method retrieves detailed information about all columns in the
    /// specified table.
    ///
    /// # Arguments
    ///
    /// * `database` - The database name, or None for the current database
    /// * `schema` - The schema name, or None for the default schema
    /// * `table` - The table name
    ///
    /// # Returns
    ///
    /// A vector of `ColumnInfo` objects representing the table's columns.
    ///
    /// # Errors
    ///
    /// This method will return an error if the table doesn't exist or if
    /// the metadata query fails.
    async fn list_columns(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>>;

    /// Get detailed information about a table.
    ///
    /// This method retrieves comprehensive information about a specific table,
    /// including its schema, metadata, and statistics.
    ///
    /// # Arguments
    ///
    /// * `database` - The database name, or None for the current database
    /// * `schema` - The schema name, or None for the default schema
    /// * `table` - The table name
    ///
    /// # Returns
    ///
    /// A `TableInfo` object with detailed table information.
    ///
    /// # Errors
    ///
    /// This method will return an error if the table doesn't exist or if
    /// the metadata query fails.
    async fn get_table_info(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo>;

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
