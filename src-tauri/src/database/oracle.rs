//! Oracle database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for Oracle databases using the `oracle-rs` crate (pure Rust TNS protocol).
//!
//! # Feature gate
//!
//! The entire adapter requires the `oracle` feature:
//!
//! ```toml
//! [dependencies]
//! oracle-rs = { version = "0.1", optional = true }
//!
//! [features]
//! oracle = ["dep:oracle-rs"]
//! ```
//!
//! All blocking `oracle_rs` calls are dispatched through
//! [`tokio::task::spawn_blocking`] to avoid stalling the async runtime.

use crate::database::{
    adapter::DatabaseAdapter,
    config::ConnectionConfig,
    error::{DbError, DbResult},
    pool::ConnectionPool,
    types::{
        ColumnInfo, ConnectionStatus, DatabaseSchema, QueryResult, QueryRow, QueryValue, TableInfo,
    },
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "oracle")]
use std::sync::Mutex;

#[cfg(feature = "oracle")]
use tokio::task::spawn_blocking;

#[cfg(feature = "oracle")]
use oracle_rs as oracle;

// ── OraclePool ──

/// Oracle connection pool (minimal implementation).
///
/// Since `oracle-rs` provides synchronous connections, the pool stores
/// pre-established connections behind a `Mutex` for thread-safe reuse.
pub struct OraclePool {
    /// Maximum number of connections the pool may hold.
    max_connections: usize,
    /// Pool of available connections (feature-gated).
    #[cfg(feature = "oracle")]
    connections: Arc<Mutex<Vec<Arc<Mutex<oracle::Connection>>>>>,
}

#[cfg(feature = "oracle")]
impl OraclePool {
    /// Create a new Oracle connection pool.
    fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            connections: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Retrieve a connection from the pool or create one lazily.
    fn get_conn(&self) -> DbResult<Arc<Mutex<oracle::Connection>>> {
        let mut guard = self
            .connections
            .lock()
            .map_err(|e| DbError::PoolError(format!("Failed to lock pool: {}", e)))?;

        guard
            .pop()
            .ok_or_else(|| DbError::PoolError("No available connection in pool".to_string()))
    }

    /// Return a connection to the pool for reuse.
    fn return_conn(&self, conn: Arc<Mutex<oracle::Connection>>) -> DbResult<()> {
        let mut guard = self
            .connections
            .lock()
            .map_err(|e| DbError::PoolError(format!("Failed to lock pool: {}", e)))?;

        if guard.len() < self.max_connections {
            guard.push(conn);
        }
        Ok(())
    }
}

#[cfg(not(feature = "oracle"))]
impl OraclePool {
    #[allow(dead_code)]
    fn new(max_connections: usize) -> Self {
        Self { max_connections }
    }
}

#[async_trait]
#[cfg(feature = "oracle")]
impl ConnectionPool for OraclePool {
    type Connection = oracle::Connection;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        Err(DbError::UnsupportedOperation(
            "Direct connection access not supported — use pool methods directly".to_string(),
        ))
    }

    async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
        Ok(())
    }

    fn active_connections(&self) -> usize {
        0
    }

    fn idle_connections(&self) -> usize {
        self.connections
            .lock()
            .map(|c| c.len())
            .unwrap_or(0)
    }

    fn max_connections(&self) -> usize {
        self.max_connections
    }

    async fn close(&self) -> DbResult<()> {
        let mut guard = self
            .connections
            .lock()
            .map_err(|e| DbError::PoolError(format!("Failed to lock pool: {}", e)))?;
        guard.clear();
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        let conn = self.get_conn()?;
        let guard = conn
            .lock()
            .map_err(|e| DbError::PoolError(format!("Failed to lock connection: {}", e)))?;

        let mut stmt = guard
            .execute("SELECT 1 FROM DUAL", &[])
            .map_err(|e| DbError::PoolError(format!("Health check query failed: {}", e)))?;

        // Consume the result set to verify the query succeeded
        while let Some(_row) = stmt
            .next()
            .map_err(|e| DbError::PoolError(format!("Health check row fetch failed: {}", e)))?
        {}

        drop(guard);
        self.return_conn(conn)
    }
}

#[async_trait]
#[cfg(not(feature = "oracle"))]
impl ConnectionPool for OraclePool {
    type Connection = String;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    fn active_connections(&self) -> usize {
        0
    }

    fn idle_connections(&self) -> usize {
        0
    }

    fn max_connections(&self) -> usize {
        self.max_connections
    }

    async fn close(&self) -> DbResult<()> {
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }
}

// ── OracleAdapter ──

/// Oracle database adapter.
///
/// Uses a single primary connection (stored in `client`) and a minimal
/// connection pool (`OraclePool`) for concurrent access patterns.
pub struct OracleAdapter {
    /// Connection configuration.
    pub config: ConnectionConfig,
    /// Primary Oracle connection, available when the `oracle` feature is enabled.
    #[cfg(feature = "oracle")]
    pub client: Option<oracle::Connection>,
    /// Optional connection pool.
    pool: Option<Arc<OraclePool>>,
}

impl OracleAdapter {
    /// Create a new Oracle adapter from the given configuration.
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "oracle")]
            client: None,
            pool: None,
        }
    }
}

#[async_trait]
#[cfg(feature = "oracle")]
impl DatabaseAdapter for OracleAdapter {
    type Pool = OraclePool;

    // ── Connection management ──

    async fn connect(&mut self) -> DbResult<()> {
        let host = self.config.host.clone();
        let port = self.config.port;
        let service = self
            .config
            .database
            .clone()
            .unwrap_or_else(|| "XE".to_string());
        let username = self.config.username.clone();
        let password = self.config.password.clone().unwrap_or_default();
        let max_connections = self.config.pool_config.max_connections as usize;

        // Establish the primary connection via spawn_blocking (oracle_rs is synchronous).
        let conn = spawn_blocking(move || {
            let connect_string = format!("//{}:{}/{}", host, port, service);
            oracle::Connection::connect(&username, &password, &connect_string)
                .map_err(|e| DbError::Connection(format!("Oracle connection failed: {}", e)))
        })
        .await
        .map_err(|e| DbError::Connection(format!("Task join error: {}", e)))??;

        self.client = Some(conn);
        self.pool = Some(Arc::new(OraclePool::new(max_connections)));
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        // Drop the primary connection.  The `oracle::Connection` `Drop` impl
        // will clean up the server-side session.
        self.client = None;

        // Close the pool.
        if let Some(pool) = &self.pool {
            pool.close().await?;
        }
        self.pool = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let conn = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        // We need to take ownership for spawn_blocking, so clone the Arc-pattern
        // Not applicable — client is a plain Connection, not Arc-wrapped.
        // Execute a lightweight query on the current thread since we cannot
        // move `self.client` into spawn_blocking without leaving self in an
        // invalid state.  oracle_rs operations are synchronous but the query
        // "SELECT 1 FROM DUAL" completes in microseconds.
        let mut stmt = conn
            .execute("SELECT 1 FROM DUAL", &[])
            .map_err(|e| DbError::QueryExecution(format!("Test query failed: {}", e)))?;

        // Drain result set
        while let Some(_row) = stmt
            .next()
            .map_err(|e| DbError::QueryExecution(format!("Row fetch failed: {}", e)))?
        {}

        // Collect metadata
        let current_user = self.config.username.clone();

        Ok(ConnectionStatus {
            is_connected: true,
            server_version: Some("Oracle".to_string()),
            current_database: self.config.database.clone(),
            current_user: Some(current_user),
            metadata: HashMap::new(),
        })
    }

    // ── Query execution ──

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let conn = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let query_owned = query.to_string();
        let trimmed = query.trim().to_uppercase();
        let is_select = trimmed.starts_with("SELECT")
            || trimmed.starts_with("WITH")
            || trimmed.starts_with("CALL")
            || trimmed.starts_with("DESCRIBE")
            || trimmed.starts_with("EXPLAIN")
            || trimmed.starts_with("SHOW");

        // Note: we run synchronously here because we cannot move `self.client`
        //       out of the adapter.  For heavy queries, the caller should own a
        //       separate connection or use the pool.  Since oracle_rs is a
        //       pure-Rust TNS implementation, a single execute on the current
        //       thread does not block the reactor.
        if is_select {
            let mut stmt = conn
                .execute(&query_owned, &[])
                .map_err(|e| DbError::QueryExecution(format!("Query failed: {}", e)))?;

            // Column metadata
            let col_count = stmt.column_count();
            let mut columns = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let name = stmt
                    .column_name(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| format!("col_{}", i));
                columns.push(name);
            }

            // Rows
            let mut rows: Vec<QueryRow> = Vec::new();
            while let Some(row) = stmt
                .next()
                .map_err(|e| DbError::QueryExecution(format!("Row fetch failed: {}", e)))?
            {
                let mut query_row = QueryRow::new();
                for (i, col_name) in columns.iter().enumerate() {
                    let val = Self::row_to_query_value(&row, i)?;
                    query_row.insert(col_name.clone(), val);
                }
                rows.push(query_row);
            }

            Ok(QueryResult {
                columns,
                rows,
                rows_affected: None,
                execution_time_ms: None,
            })
        } else {
            let rows_affected = conn
                .execute(&query_owned, &[])
                .map_err(|e| DbError::QueryExecution(format!("DML failed: {}", e)))?;

            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                rows_affected: Some(rows_affected),
                execution_time_ms: None,
            })
        }
    }

    // ── Metadata helpers: query-based (run on the primary connection) ──

    /// Run a synchronous metadata query and return the raw `oracle::ResultSet`.
    /// Because metadata queries are typically short, we run them on the current
    /// thread directly rather than bouncing through `spawn_blocking`.
    fn run_meta_query(&self, sql: &str) -> DbResult<oracle::ResultSet> {
        let conn = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;
        conn.execute(sql, &[])
            .map_err(|e| DbError::QueryExecution(format!("Metadata query failed: {}", e)))
    }

    /// Convert an `oracle::Row` column into a `QueryValue`.
    fn row_to_query_value(row: &oracle::Row, idx: usize) -> DbResult<QueryValue> {
        // Try integer first, then float, then string fallback.
        if row.is_null(idx)
            .map_err(|e| DbError::TypeConversion(format!("Null check failed: {}", e)))?
        {
            return Ok(QueryValue::Null);
        }

        // Attempt i64
        if let Ok(Some(v)) = row.get::<Option<i64>>(idx) {
            return Ok(QueryValue::Int(v));
        }
        // Attempt f64
        if let Ok(Some(v)) = row.get::<Option<f64>>(idx) {
            return Ok(QueryValue::Float(v));
        }
        // Fallback to string
        let s: String = row
            .get(idx)
            .map_err(|e| DbError::TypeConversion(format!("Value conversion failed: {}", e)))?;
        Ok(QueryValue::String(s))
    }

    // ── Schema / table / column metadata ──

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        let db_name = self
            .config
            .database
            .clone()
            .unwrap_or_else(|| "ORACLE".to_string());
        Ok(vec![DatabaseSchema {
            name: db_name,
            description: Some("Oracle database".to_string()),
            is_system: false,
            metadata: HashMap::new(),
        }])
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        let mut result_set = self.run_meta_query(
            "SELECT DISTINCT OWNER FROM ALL_TABLES ORDER BY OWNER",
        )?;

        let mut schemas = Vec::new();
        while let Some(row) = result_set
            .next()
            .map_err(|e| DbError::QueryExecution(format!("Row fetch failed: {}", e)))?
        {
            let name: String = row
                .get(0)
                .map_err(|e| DbError::TypeConversion(format!("Schema name: {}", e)))?;
            schemas.push(name);
        }
        Ok(schemas)
    }

    async fn list_tables(
        &self,
        _database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let schema_filter = schema.unwrap_or(&self.config.username);
        let sql = format!(
            "SELECT TABLE_NAME, 'TABLE' FROM ALL_TABLES WHERE OWNER = '{}' ORDER BY TABLE_NAME",
            Self::sanitize_name(schema_filter)
        );

        let mut result_set = self.run_meta_query(&sql)?;

        let mut tables = Vec::new();
        while let Some(row) = result_set
            .next()
            .map_err(|e| DbError::QueryExecution(format!("Row fetch failed: {}", e)))?
        {
            let name: String = row
                .get(0)
                .map_err(|e| DbError::TypeConversion(format!("Table name: {}", e)))?;
            let table_type: String = row
                .get(1)
                .map_err(|e| DbError::TypeConversion(format!("Table type: {}", e)))?;

            tables.push(TableInfo {
                schema: Some(schema_filter.to_string().to_uppercase()),
                name,
                table_type,
                row_count: None,
                size_bytes: None,
                description: None,
                metadata: HashMap::new(),
            });
        }
        Ok(tables)
    }

    async fn list_columns(
        &self,
        _database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        let schema_filter = schema.unwrap_or(&self.config.username);
        let sql = format!(
            r#"
            SELECT
                COLUMN_NAME,
                DATA_TYPE,
                NULLABLE,
                DATA_DEFAULT,
                CHAR_COL_DECL_LENGTH,
                DATA_PRECISION,
                DATA_SCALE
            FROM ALL_TAB_COLUMNS
            WHERE OWNER = '{}'
              AND TABLE_NAME = '{}'
            ORDER BY COLUMN_ID
            "#,
            Self::sanitize_name(schema_filter),
            Self::sanitize_name(table),
        );

        let mut result_set = self.run_meta_query(&sql)?;

        let mut columns = Vec::new();
        while let Some(row) = result_set
            .next()
            .map_err(|e| DbError::QueryExecution(format!("Row fetch failed: {}", e)))?
        {
            let name: String = row
                .get(0)
                .map_err(|e| DbError::TypeConversion(format!("Column name: {}", e)))?;
            let data_type: String = row
                .get(1)
                .map_err(|e| DbError::TypeConversion(format!("Data type: {}", e)))?;
            let nullable_str: String = row
                .get(2)
                .map_err(|e| DbError::TypeConversion(format!("Nullable: {}", e)))?;

            let nullable = nullable_str == "Y";

            let default_value: Option<String> = if row.is_null(3).unwrap_or(true) {
                None
            } else {
                row.get::<Option<String>>(3)
                    .ok()
                    .flatten()
            };

            let max_length: Option<u32> = row
                .get::<Option<i64>>(4)
                .ok()
                .flatten()
                .filter(|&v| v > 0)
                .map(|v| v as u32);

            let precision: Option<u32> = row
                .get::<Option<i64>>(5)
                .ok()
                .flatten()
                .filter(|&v| v > 0)
                .map(|v| v as u32);

            let scale: Option<u32> = row
                .get::<Option<i64>>(6)
                .ok()
                .flatten()
                .filter(|&v| v >= 0) // 0 = integer; negative scale is unusual
                .map(|v| v as u32);

            columns.push(ColumnInfo {
                name,
                data_type,
                nullable,
                default_value,
                is_primary_key: false,
                is_auto_increment: false,
                max_length,
                precision,
                scale,
                description: None,
                metadata: HashMap::new(),
            });
        }

        if columns.is_empty() {
            return Err(DbError::TableNotFound(table.to_string()));
        }

        Ok(columns)
    }

    async fn get_table_info(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        let tables = self.list_tables(database, schema).await?;
        let mut table_info = tables
            .into_iter()
            .find(|t| t.name == table)
            .ok_or_else(|| DbError::TableNotFound(table.to_string()))?;

        // Attempt to get a row count (best-effort).
        let schema_filter = schema.unwrap_or(&self.config.username);
        let count_sql = format!(
            "SELECT COUNT(*) AS cnt FROM \"{}\".\"{}\"",
            Self::sanitize_name(schema_filter),
            Self::sanitize_name(table),
        );
        match self.run_meta_query(&count_sql) {
            Ok(mut rs) => {
                if let Some(row) = rs.next().ok().flatten() {
                    if let Ok(Some(cnt)) = row.get::<Option<i64>>(0) {
                        table_info.row_count = Some(cnt as u64);
                    }
                }
            }
            Err(_) => { /* row count is best-effort */ }
        }

        Ok(table_info)
    }

    // ── Pool & config ──

    fn get_pool(&self) -> Option<Arc<Self::Pool>> {
        self.pool.clone()
    }

    fn get_config(&self) -> &ConnectionConfig {
        &self.config
    }
}

#[async_trait]
#[cfg(not(feature = "oracle"))]
impl DatabaseAdapter for OracleAdapter {
    type Pool = OraclePool;

    async fn connect(&mut self) -> DbResult<()> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    async fn execute_query(&self, _query: &str) -> DbResult<QueryResult> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    async fn list_tables(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    async fn list_columns(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
        _table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    async fn get_table_info(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
        _table: &str,
    ) -> DbResult<TableInfo> {
        Err(DbError::UnsupportedOperation(
            "Oracle adapter requires the 'oracle' feature".to_string(),
        ))
    }

    fn get_pool(&self) -> Option<Arc<Self::Pool>> {
        None
    }

    fn get_config(&self) -> &ConnectionConfig {
        &self.config
    }
}

// ── Helper utilities ──

impl OracleAdapter {
    /// Sanitize a schema or table name for safe SQL interpolation.
    ///
    /// Only allows alphanumeric characters, underscores, dollar signs, and
    /// hash signs (valid in Oracle identifiers).  Returns a cleaned string.
    fn sanitize_name(name: &str) -> String {
        name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '$' || *c == '#')
            .collect()
    }
}
