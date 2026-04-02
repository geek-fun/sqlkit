//! Database adapter module.
//!
//! This module provides a unified interface for interacting with various database systems.
//! It includes trait definitions, error types, configuration structures, and connection
//! pooling interfaces.
//!
//! # Overview
//!
//! The core of this module is the `DatabaseAdapter` trait, which defines a consistent
//! interface for database operations across different database systems including:
//! - PostgreSQL
//! - MySQL
//! - Oracle
//! - SQL Server
//! - IBM DB2
//! - SQLite
//! - H2
//! - ClickHouse
//!
//! # Architecture
//!
//! The module is organized into several submodules:
//!
//! - `adapter`: Core trait definition for database adapters
//! - `config`: Connection configuration and database type definitions
//! - `error`: Comprehensive error types for database operations
//! - `pool`: Connection pooling interface
//! - `types`: Data structures for query results and metadata
//!
//! # Example Usage
//!
//! ```ignore
//! use sqlkit::database::{
//!     DatabaseAdapter, ConnectionConfig, DatabaseType,
//! };
//!
//! async fn example() {
//!     // Create a connection configuration
//!     let config = ConnectionConfig::new(
//!         DatabaseType::PostgreSQL,
//!         "localhost",
//!         5432,
//!         "user",
//!     )
//!     .with_database("mydb")
//!     .with_password("password");
//!
//!     // Create an adapter (implementation specific)
//!     // let mut adapter = PostgresAdapter::new(config);
//!     
//!     // Connect to the database
//!     // adapter.connect().await?;
//!     
//!     // Execute queries
//!     // let result = adapter.execute_query("SELECT * FROM users").await?;
//!     
//!     // Disconnect
//!     // adapter.disconnect().await?;
//! }
//! ```

pub mod adapter;
pub mod config;
pub mod error;
pub mod manager;
pub mod mysql;
pub mod pool;
pub mod postgres;
pub mod sqlite;
pub mod sqlserver;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use adapter::DatabaseAdapter;
pub use config::{ConnectionConfig, DatabaseType, PoolConfig, SslMode};
pub use error::{DbError, DbResult};
pub use manager::{ConnectionManager, ConnectionMetadata, ManagerStats};
pub use mysql::{MySQLAdapter, MySQLPool};
pub use pool::{ConnectionPool, PoolStats};
pub use postgres::{PostgresAdapter, PostgresPool};
pub use sqlite::{SQLiteAdapter, SQLitePool};
pub use sqlserver::{SqlServerAdapter, SqlServerPool};
pub use types::{
    ColumnInfo, ConnectionStatus, DatabaseSchema, QueryResult, QueryRow, QueryValue, TableInfo,
};
