//! Type definitions for database metadata and query results.
//!
//! This module defines structures for representing database metadata like
//! schemas, tables, columns, and query results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a database schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    /// Name of the database.
    pub name: String,
    /// Optional description or comment.
    pub description: Option<String>,
    /// Whether this is a system database.
    #[serde(default)]
    pub is_system: bool,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Represents a database table or view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Schema name (if applicable).
    pub schema: Option<String>,
    /// Table name.
    pub name: String,
    /// Table type (TABLE, VIEW, etc.).
    pub table_type: String,
    /// Number of rows (if available).
    pub row_count: Option<u64>,
    /// Table size in bytes (if available).
    pub size_bytes: Option<u64>,
    /// Optional description or comment.
    pub description: Option<String>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Represents a table column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// Column name.
    pub name: String,
    /// Data type.
    pub data_type: String,
    /// Whether the column is nullable.
    pub nullable: bool,
    /// Default value (if any).
    pub default_value: Option<String>,
    /// Whether the column is part of the primary key.
    pub is_primary_key: bool,
    /// Whether the column is auto-increment.
    pub is_auto_increment: bool,
    /// Maximum length (for string types).
    pub max_length: Option<u32>,
    /// Precision (for numeric types).
    pub precision: Option<u32>,
    /// Scale (for decimal types).
    pub scale: Option<u32>,
    /// Optional description or comment.
    pub description: Option<String>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Represents a query result value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryValue {
    /// Null value.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i64),
    /// Floating point value.
    Float(f64),
    /// String value.
    String(String),
    /// Binary data.
    Bytes(Vec<u8>),
    /// Date/time value (ISO 8601 string).
    DateTime(String),
}

/// Represents a single row in a query result.
pub type QueryRow = HashMap<String, QueryValue>;

/// Represents the result of a query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Column names in order.
    pub columns: Vec<String>,
    /// Rows of data.
    pub rows: Vec<QueryRow>,
    /// Number of rows affected (for INSERT, UPDATE, DELETE).
    pub rows_affected: Option<u64>,
    /// Execution time in milliseconds.
    pub execution_time_ms: Option<u64>,
}

impl QueryResult {
    /// Create a new empty query result.
    pub fn new(columns: Vec<String>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            rows_affected: None,
            execution_time_ms: None,
        }
    }

    /// Create a query result for a non-SELECT statement.
    pub fn affected(rows_affected: u64) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: Some(rows_affected),
            execution_time_ms: None,
        }
    }

    /// Add a row to the result.
    pub fn add_row(&mut self, row: QueryRow) {
        self.rows.push(row);
    }

    /// Set the execution time.
    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = Some(ms);
        self
    }
}

/// Information about a database object (view, procedure, function).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    /// Object name.
    pub name: String,
    /// Object type (VIEW, PROCEDURE, FUNCTION).
    pub object_type: String,
    /// Schema name.
    pub schema: Option<String>,
    /// Detail information (columns for views, params for procs, return type for funcs).
    pub detail: Option<String>,
}

/// Information about a table index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    /// Index name.
    pub name: String,
    /// Column names included in the index.
    pub columns: Vec<String>,
    /// Index type (BTREE, HASH, GIN, GIST, etc.).
    pub index_type: String,
    /// Whether the index enforces uniqueness.
    pub is_unique: bool,
    /// Whether this is the primary key index.
    pub is_primary: bool,
}

/// Information about a foreign key constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    /// Constraint name.
    pub constraint_name: String,
    /// Local columns in the constraint.
    pub columns: Vec<String>,
    /// Referenced schema.
    pub referenced_schema: Option<String>,
    /// Referenced table.
    pub referenced_table: String,
    /// Referenced columns.
    pub referenced_columns: Vec<String>,
    /// ON UPDATE action (CASCADE, SET NULL, RESTRICT, NO ACTION).
    pub on_update: Option<String>,
    /// ON DELETE action (CASCADE, SET NULL, RESTRICT, NO ACTION).
    pub on_delete: Option<String>,
}

/// Information about a table trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    /// Trigger name.
    pub name: String,
    /// Action timing (BEFORE, AFTER, INSTEAD OF).
    pub action_timing: String,
    /// Trigger event (INSERT, UPDATE, DELETE).
    pub event: String,
    /// DDL source of the trigger.
    pub ddl: Option<String>,
}

/// Connection status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    /// Whether the connection is active.
    pub is_connected: bool,
    /// Database server version.
    pub server_version: Option<String>,
    /// Current database name.
    pub current_database: Option<String>,
    /// Current user.
    pub current_user: Option<String>,
    /// Additional status information.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}
