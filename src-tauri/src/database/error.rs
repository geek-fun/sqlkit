//! Error types for database operations.
//!
//! This module defines comprehensive error types for database adapter operations,
//! covering connection, query execution, and data retrieval scenarios.

use thiserror::Error;

/// Result type alias for database operations.
pub type DbResult<T> = Result<T, DbError>;

/// Comprehensive error type for database operations.
#[derive(Error, Debug)]
pub enum DbError {
    /// Connection-related errors.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Authentication failures.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Query execution errors.
    #[error("Query execution error: {0}")]
    QueryExecution(String),

    /// Invalid query syntax or structure.
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Timeout during operation.
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Connection pool errors.
    #[error("Connection pool error: {0}")]
    PoolError(String),

    /// Database not found.
    #[error("Database not found: {0}")]
    DatabaseNotFound(String),

    /// Table not found.
    #[error("Table not found: {0}")]
    TableNotFound(String),

    /// Column not found.
    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    /// Data type conversion errors.
    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    /// Serialization/deserialization errors.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Configuration errors.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Unsupported operation for this database type.
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Generic database error.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// I/O errors.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with source.
    #[error("Error: {message}")]
    Other {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl DbError {
    /// Create a new error with a message and optional source.
    pub fn new(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new error with a message and source.
    pub fn with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Other {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}
