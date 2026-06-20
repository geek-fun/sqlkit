//! Unified connection handle trait — eliminates enum dispatch in Tauri commands.
//!
//! Instead of matching on `ActiveConnection` variants in every command, callers
//! use the trait methods directly: `connection.execute_query(&sql).await`.
//!
//! The trait is implemented on `ActiveConnection` itself via delegation to the
//! inner adapter through its `Arc<Mutex<...>>`.

use crate::database::{
    error::DbResult,
    types::{ColumnInfo, ConnectionStatus, DatabaseSchema, ForeignKeyInfo, IndexInfo, ObjectInfo, QueryResult, TableInfo, TriggerInfo},
};
use async_trait::async_trait;

/// Unified interface for all database connections.
///
/// Implemented on `ActiveConnection` to eliminate variant matching in
/// Tauri command handlers.
#[async_trait]
pub trait ConnectionHandle: Send + Sync {
    async fn execute_query(&self, sql: &str) -> DbResult<QueryResult>;
    async fn test_connection(&self) -> DbResult<ConnectionStatus>;
    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>>;
    async fn list_schemas(&self, database: Option<&str>) -> DbResult<Vec<String>>;
    async fn list_tables(&self, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<TableInfo>>;
    async fn list_columns(&self, database: Option<&str>, schema: Option<&str>, table: &str) -> DbResult<Vec<ColumnInfo>>;
    async fn get_table_info(&self, database: Option<&str>, schema: Option<&str>, table: &str) -> DbResult<TableInfo>;
    async fn get_foreign_keys(&self, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<ForeignKeyInfo>>;
    async fn list_views(&self, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<ObjectInfo>>;
    async fn list_procedures(&self, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<ObjectInfo>>;
    async fn list_functions(&self, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<ObjectInfo>>;
    async fn list_triggers(&self, database: Option<&str>, schema: Option<&str>, table: &str) -> DbResult<Vec<TriggerInfo>>;
    async fn list_indexes(&self, database: Option<&str>, schema: Option<&str>, table: &str) -> DbResult<Vec<IndexInfo>>;
    async fn list_foreign_keys_for_table(&self, database: Option<&str>, schema: Option<&str>, table: &str) -> DbResult<Vec<ForeignKeyInfo>>;
    async fn get_object_ddl(&self, database: Option<&str>, schema: Option<&str>, object_name: &str, object_type: &str) -> DbResult<String>;
    async fn drop_object(&self, database: Option<&str>, schema: Option<&str>, object_name: &str, object_type: &str) -> DbResult<()>;
    async fn rename_object(&self, database: Option<&str>, schema: Option<&str>, object_name: &str, object_type: &str, new_name: &str) -> DbResult<()>;
    async fn disconnect(&self) -> DbResult<()>;
    async fn query_timeout_secs(&self) -> u64;
}
