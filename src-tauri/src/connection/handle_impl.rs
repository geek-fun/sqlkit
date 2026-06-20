//! ConnectionHandle implementation for ActiveConnection.
//!
//! Each method delegates to the inner adapter via Arc<Mutex<...>>.lock().await.
//! This eliminates variant matching in every Tauri command.

use crate::database::{
    adapter::DatabaseAdapter,
    error::DbResult,
    types::{
        ColumnInfo, ConnectionStatus, DatabaseSchema, ForeignKeyInfo, IndexInfo, ObjectInfo, QueryResult,
        TableInfo, TriggerInfo,
    },
};
use crate::state::ActiveConnection;

use super::handle::ConnectionHandle;
use async_trait::async_trait;

macro_rules! delegate {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            ActiveConnection::Postgres(a) => a.lock().await.$method($($arg),*).await,
            ActiveConnection::MySQL(a) => a.lock().await.$method($($arg),*).await,
            ActiveConnection::SQLServer(a) => a.lock().await.$method($($arg),*).await,
            ActiveConnection::SQLite(a) => a.lock().await.$method($($arg),*).await,
            ActiveConnection::ClickHouse(a) => a.lock().await.$method($($arg),*).await,
            ActiveConnection::JdbcBridge(a) => a.lock().await.$method($($arg),*).await,
            ActiveConnection::HttpSql(a) => a.lock().await.$method($($arg),*).await,
            ActiveConnection::Rqlite(a) => a.lock().await.$method($($arg),*).await,
            ActiveConnection::Turso(a) => a.lock().await.$method($($arg),*).await,
        }
    };
}

#[async_trait]
impl ConnectionHandle for ActiveConnection {
    async fn execute_query(&self, sql: &str) -> DbResult<QueryResult> {
        delegate!(self, execute_query, sql)
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        delegate!(self, test_connection)
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        delegate!(self, list_databases)
    }

    async fn list_schemas(&self, database: Option<&str>) -> DbResult<Vec<String>> {
        delegate!(self, list_schemas, database)
    }

    async fn list_tables(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        delegate!(self, list_tables, database, schema)
    }

    async fn list_columns(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        delegate!(self, list_columns, database, schema, table)
    }

    async fn get_table_info(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        delegate!(self, get_table_info, database, schema, table)
    }

    async fn get_foreign_keys(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<ForeignKeyInfo>> {
        delegate!(self, get_foreign_keys, database, schema)
    }

    async fn list_views(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<ObjectInfo>> {
        delegate!(self, list_views, database, schema)
    }

    async fn list_procedures(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<ObjectInfo>> {
        delegate!(self, list_procedures, database, schema)
    }

    async fn list_functions(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<ObjectInfo>> {
        delegate!(self, list_functions, database, schema)
    }

    async fn list_triggers(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<TriggerInfo>> {
        delegate!(self, list_triggers, database, schema, table)
    }

    async fn list_indexes(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<IndexInfo>> {
        delegate!(self, list_indexes, database, schema, table)
    }

    async fn list_foreign_keys_for_table(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ForeignKeyInfo>> {
        delegate!(self, list_foreign_keys, database, schema, table)
    }

    async fn get_object_ddl(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        object_name: &str,
        object_type: &str,
    ) -> DbResult<String> {
        delegate!(self, get_object_ddl, database, schema, object_name, object_type)
    }

    async fn drop_object(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        object_name: &str,
        object_type: &str,
    ) -> DbResult<()> {
        delegate!(self, drop_object, database, schema, object_name, object_type)
    }

    async fn rename_object(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        object_name: &str,
        object_type: &str,
        new_name: &str,
    ) -> DbResult<()> {
        delegate!(self, rename_object, database, schema, object_name, object_type, new_name)
    }

    async fn disconnect(&self) -> DbResult<()> {
        delegate!(self, disconnect)
    }
}
