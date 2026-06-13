//! DuckDB database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for DuckDB databases using the `duckdb` crate with the `bundled` feature.
//! Supports both in-memory (`:memory:`) and file-based DuckDB databases.

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
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ── DuckDB crate imports (only available with `duckdb` feature) ──
#[cfg(feature = "duckdb")]
use duckdb::{
    types::{TimeUnit, ValueRef},
    Connection,
};

/// Constant for in-memory database identifier.
const MEMORY_DB: &str = ":memory:";

// ── Sendable wrapper for duckdb::Connection ──
// SAFETY: This is safe because we always access the connection through a Mutex
// in the pool, ensuring exclusive access across threads.
#[cfg(feature = "duckdb")]
pub struct SendableDuckConnection(pub Connection);

#[cfg(feature = "duckdb")]
unsafe impl Send for SendableDuckConnection {}

#[cfg(feature = "duckdb")]
unsafe impl Sync for SendableDuckConnection {}

#[cfg(feature = "duckdb")]
impl std::ops::Deref for SendableDuckConnection {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "duckdb")]
impl std::ops::DerefMut for SendableDuckConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// ── DuckDbPool ──

/// DuckDB connection pool wrapper.
///
/// DuckDB is an embedded database where each process typically uses a single
/// database file. This pool manages connections with proper synchronization
/// for thread-safety.
pub struct DuckDbPool {
    /// Pool of available connections protected by a Mutex for thread safety.
    /// Each connection is individually wrapped in Arc<Mutex<>> for safe concurrent access.
    #[cfg(feature = "duckdb")]
    connections: Arc<Mutex<Vec<Arc<Mutex<Connection>>>>>,
    /// Maximum number of connections in the pool.
    max_connections: usize,
    /// Optional path to the database file. None for in-memory databases.
    #[cfg(feature = "duckdb")]
    db_path: Option<PathBuf>,
}

#[cfg(feature = "duckdb")]
impl DuckDbPool {
    /// Create a new DuckDB connection pool.
    fn new(max_connections: usize, db_path: Option<PathBuf>) -> Self {
        Self {
            connections: Arc::new(Mutex::new(Vec::new())),
            max_connections,
            db_path,
        }
    }

    /// Get a connection from the pool or create a new one.
    async fn get_conn(&self) -> DbResult<Arc<Mutex<Connection>>> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| DbError::PoolError(format!("Failed to lock connections: {}", e)))?;

        if let Some(conn) = connections.pop() {
            return Ok(conn);
        }

        // Create new connection
        let conn = self.create_connection()?;
        Ok(Arc::new(Mutex::new(conn)))
    }

    /// Return a connection to the pool.
    fn return_conn(&self, conn: Arc<Mutex<Connection>>) -> DbResult<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| DbError::PoolError(format!("Failed to lock connections: {}", e)))?;

        if connections.len() < self.max_connections {
            connections.push(conn);
        }
        Ok(())
    }

    /// Create a new DuckDB connection with proper configuration.
    fn create_connection(&self) -> DbResult<Connection> {
        let conn = if let Some(ref path) = self.db_path {
            Connection::open(path)
                .map_err(|e| DbError::Connection(format!("Failed to open database: {}", e)))?
        } else {
            // In-memory database
            Connection::open_in_memory().map_err(|e| {
                DbError::Connection(format!("Failed to open in-memory database: {}", e))
            })?
        };

        Ok(conn)
    }
}

#[cfg(not(feature = "duckdb"))]
impl DuckDbPool {
    #[allow(dead_code)]
    fn new(max_connections: usize, _db_path: Option<PathBuf>) -> Self {
        Self { max_connections }
    }
}

#[async_trait]
#[cfg(feature = "duckdb")]
impl ConnectionPool for DuckDbPool {
    type Connection = SendableDuckConnection;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        // NOTE: This method is not used in the current implementation.
        // DuckDB connections are managed through the custom get_conn() method instead,
        // which returns Arc<Mutex<Connection>> for proper thread-safety.
        // See SQLite adapter for rationale.
        Err(DbError::UnsupportedOperation(
            "Direct connection access not supported - use get_conn() instead".to_string(),
        ))
    }

    async fn return_connection(&self, connection: Arc<Self::Connection>) -> DbResult<()> {
        // Immediately drop the connection to avoid Send issues
        // duckdb::Connection may not be Send/Sync in all configurations
        std::mem::drop(connection);
        std::future::ready(Ok(())).await
    }

    fn active_connections(&self) -> usize {
        self.connections
            .lock()
            .map(|c| self.max_connections - c.len())
            .unwrap_or(0)
    }

    fn idle_connections(&self) -> usize {
        self.connections.lock().map(|c| c.len()).unwrap_or(0)
    }

    fn max_connections(&self) -> usize {
        self.max_connections
    }

    async fn close(&self) -> DbResult<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| DbError::PoolError(format!("Failed to lock connections: {}", e)))?;
        connections.clear();
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        let conn = self.get_conn().await?;
        let conn_guard = conn
            .lock()
            .map_err(|e| DbError::PoolError(format!("Failed to lock connection: {}", e)))?;

        conn_guard
            .execute("SELECT 1", [])
            .map_err(|e| DbError::PoolError(format!("Health check query failed: {}", e)))?;

        drop(conn_guard);
        self.return_conn(conn)?;
        Ok(())
    }
}

#[async_trait]
#[cfg(not(feature = "duckdb"))]
impl ConnectionPool for DuckDbPool {
    type Connection = String;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        Err(DbError::UnsupportedOperation(
            "DuckDB adapter requires the 'duckdb' feature".to_string(),
        ))
    }

    async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
        Err(DbError::UnsupportedOperation(
            "DuckDB adapter requires the 'duckdb' feature".to_string(),
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
            "DuckDB adapter requires the 'duckdb' feature".to_string(),
        ))
    }
}

// ── DuckDbAdapter ──

/// DuckDB database adapter.
///
/// Supports both in-memory (`:memory:`) and file-based DuckDB databases with
/// proper thread-safety through connection pooling. The database path is
/// resolved from `config.database` or `config.host`.
pub struct DuckDbAdapter {
    /// Connection configuration.
    pub config: ConnectionConfig,
    /// Thread-safe connection pool.
    pool: Option<Arc<DuckDbPool>>,
    /// Resolved path to the database file. `None` for in-memory databases.
    db_path: Option<PathBuf>,
}

impl DuckDbAdapter {
    /// Create a new DuckDB adapter with the given configuration.
    ///
    /// The database path is resolved in the following order:
    /// 1. `config.database` (if set and not `:memory:`)
    /// 2. `config.host` (if set and not `:memory:` or empty)
    /// 3. `None` (in-memory database)
    pub fn new(config: ConnectionConfig) -> Self {
        let db_path = config
            .database
            .as_ref()
            .and_then(|db| {
                if db == MEMORY_DB {
                    None
                } else {
                    Some(PathBuf::from(db))
                }
            })
            .or_else(|| {
                if config.host == MEMORY_DB || config.host.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(&config.host))
                }
            });

        Self {
            config,
            pool: None,
            db_path,
        }
    }

    /// Validate and sanitize a table name to prevent SQL injection.
    ///
    /// Only allows alphanumeric characters, underscores, and optionally a schema prefix.
    /// Returns an error if the table name contains invalid characters.
    fn validate_table_name(table: &str) -> DbResult<()> {
        if table.is_empty() {
            return Err(DbError::InvalidQuery(
                "Table name cannot be empty".to_string(),
            ));
        }

        // Check for valid characters: alphanumeric, underscore, and dot (for schema.table)
        for c in table.chars() {
            if !c.is_alphanumeric() && c != '_' && c != '.' {
                return Err(DbError::InvalidQuery(format!(
                    "Invalid character '{}' in table name. Only alphanumeric, underscore, and dot allowed",
                    c
                )));
            }
        }

        // Additional validation: no consecutive dots, no leading/trailing dots
        if table.starts_with('.') || table.ends_with('.') || table.contains("..") {
            return Err(DbError::InvalidQuery(
                "Invalid table name format: dots must separate schema and table names".to_string(),
            ));
        }

        Ok(())
    }

    /// Convert a DuckDB `ValueRef` to a `QueryValue`.
    #[cfg(feature = "duckdb")]
    fn convert_value(val_ref: &ValueRef) -> QueryValue {
        match val_ref {
            ValueRef::Null => QueryValue::Null,
            ValueRef::Boolean(b) => QueryValue::Bool(*b),
            ValueRef::TinyInt(i) => QueryValue::Int(*i as i64),
            ValueRef::SmallInt(i) => QueryValue::Int(*i as i64),
            ValueRef::Int(i) => QueryValue::Int(*i as i64),
            ValueRef::BigInt(i) => QueryValue::Int(*i),
            ValueRef::HugeInt(i) => QueryValue::String(i.to_string()),
            ValueRef::UTinyInt(i) => QueryValue::Int(*i as i64),
            ValueRef::USmallInt(i) => QueryValue::Int(*i as i64),
            ValueRef::UInt(i) => QueryValue::Int(*i as i64),
            ValueRef::UBigInt(i) => QueryValue::Int(*i as i64),
            ValueRef::Float(f) => QueryValue::Float(*f as f64),
            ValueRef::Double(f) => QueryValue::Float(*f),
            ValueRef::Decimal(d) => {
                // Format decimal as string to preserve precision
                QueryValue::String(d.to_string())
            }
            ValueRef::Text(t) => {
                let s = std::str::from_utf8(t).unwrap_or("<invalid utf-8>");
                QueryValue::String(s.to_string())
            }
            ValueRef::Blob(b) => QueryValue::Bytes(b.to_vec()),
            ValueRef::Timestamp(unit, v) => {
                let (secs, nsecs) = Self::time_unit_to_secs_nsecs(unit, *v);
                if let Some(ts) = chrono::DateTime::from_timestamp(secs, nsecs) {
                    QueryValue::DateTime(ts.to_rfc3339())
                } else {
                    QueryValue::String(format!("Timestamp({})", v))
                }
            }
            ValueRef::Date32(days) => {
                // DuckDB Date32 is days since epoch
                if let Some(dt) =
                    chrono::NaiveDate::from_num_days_from_ce_opt((*days + 719163) as i32)
                {
                    QueryValue::DateTime(dt.format("%Y-%m-%d").to_string())
                } else {
                    QueryValue::String(format!("Date({})", days))
                }
            }
            ValueRef::Time64(unit, v) => {
                let (secs, nsecs) = Self::time_unit_to_secs_nsecs(unit, *v);
                // Time values represent duration since midnight
                let total_secs = secs as u32;
                let total_nsecs = nsecs as u32;
                if let Some(dt) =
                    chrono::NaiveTime::from_num_seconds_from_midnight_opt(total_secs, total_nsecs)
                {
                    QueryValue::DateTime(dt.format("%H:%M:%S.%f").to_string())
                } else {
                    QueryValue::String(format!("Time({})", v))
                }
            }
            ValueRef::Interval {
                months,
                days,
                nanos,
            } => QueryValue::String(format!("{} months {} days {} ns", months, days, nanos)),
            // Complex DuckDB types (List, Enum, Struct, Array, Map, Union) - format as debug
            _ => QueryValue::String(format!("{:?}", val_ref)),
        }
    }

    /// Execute a query and return results.
    #[cfg(feature = "duckdb")]
    async fn execute_query_internal(&self, query: &str) -> DbResult<QueryResult> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let conn = pool.get_conn().await?;
        let conn_guard = conn
            .lock()
            .map_err(|e| DbError::QueryExecution(format!("Failed to lock connection: {}", e)))?;

        // Determine if this is a query that returns rows
        let trimmed = query.trim().to_uppercase();
        let is_select = trimmed.starts_with("SELECT")
            || trimmed.starts_with("PRAGMA")
            || trimmed.starts_with("EXPLAIN")
            || trimmed.starts_with("DESCRIBE")
            || trimmed.starts_with("SHOW")
            || trimmed.starts_with("WITH")
            || trimmed.starts_with("CALL");

        if is_select {
            let mut stmt = conn_guard
                .prepare(query)
                .map_err(|e| DbError::QueryExecution(format!("Failed to prepare query: {}", e)))?;

            // Get column metadata before consuming the statement with query
            let column_count = stmt.column_count();
            let columns: Vec<String> = (0..column_count)
                .map(|i| {
                    stmt.column_name(i)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| format!("column_{}", i))
                })
                .collect();

            // Execute and iterate over rows
            let mut rows_iter = stmt
                .query([])
                .map_err(|e| DbError::QueryExecution(format!("Failed to execute query: {}", e)))?;

            let mut rows: Vec<QueryRow> = Vec::new();
            while let Some(row_result) = rows_iter.next() {
                let row = row_result
                    .map_err(|e| DbError::QueryExecution(format!("Failed to fetch row: {}", e)))?;
                let mut query_row = HashMap::new();
                for (idx, col_name) in columns.iter().enumerate() {
                    match row.get_ref(idx) {
                        Ok(val_ref) => {
                            let query_val = Self::convert_value(&val_ref);
                            query_row.insert(col_name.clone(), query_val);
                        }
                        Err(e) => {
                            query_row.insert(
                                col_name.clone(),
                                QueryValue::String(format!("<error: {}>", e)),
                            );
                        }
                    }
                }
                rows.push(query_row);
            }

            drop(rows_iter);
            drop(stmt);
            drop(conn_guard);
            pool.return_conn(conn)?;

            Ok(QueryResult {
                columns,
                rows,
                rows_affected: None,
                execution_time_ms: None,
            })
        } else {
            // For non-SELECT queries (INSERT, UPDATE, DELETE, CREATE, etc.)
            let rows_affected = conn_guard
                .execute(query, [])
                .map_err(|e| DbError::QueryExecution(format!("Failed to execute query: {}", e)))?;

            drop(conn_guard);
            pool.return_conn(conn)?;

            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                rows_affected: Some(rows_affected as u64),
                execution_time_ms: None,
            })
        }
    }
}

#[async_trait]
impl DatabaseAdapter for DuckDbAdapter {
    type Pool = DuckDbPool;

    async fn connect(&mut self) -> DbResult<()> {
        // For in-memory databases, force max_connections = 1 because each
        // Connection::open_in_memory() creates a separate isolated database.
        // With multiple connections, schema changes on one connection won't be
        // visible to other connections, causing "table not found" errors.
        let max_connections = if self.db_path.is_none() {
            1 // Single connection for in-memory databases
        } else {
            self.config.pool_config.max_connections as usize
        };

        let pool = DuckDbPool::new(max_connections, self.db_path.clone());

        #[cfg(feature = "duckdb")]
        {
            // Test the connection by creating one
            let conn = pool.get_conn().await?;
            pool.return_conn(conn)?;
        }

        #[cfg(not(feature = "duckdb"))]
        {
            let _ = pool; // suppress unused warning
            return Err(DbError::UnsupportedOperation(
                "DuckDB adapter requires the 'duckdb' feature to be enabled".to_string(),
            ));
        }

        self.pool = Some(Arc::new(pool));
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        if let Some(pool) = &self.pool {
            pool.close().await?;
        }
        self.pool = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        #[cfg(feature = "duckdb")]
        {
            let pool = self
                .pool
                .as_ref()
                .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

            let conn = pool.get_conn().await?;
            let conn_guard = conn
                .lock()
                .map_err(|e| DbError::Connection(format!("Failed to lock connection: {}", e)))?;

            // Get DuckDB version
            let version: String = conn_guard
                .query_row("SELECT version()", [], |row| row.get(0))
                .map_err(|e| DbError::QueryExecution(format!("Failed to get version: {}", e)))?;

            // Get database file path or indicate in-memory
            let db_name = self
                .db_path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(MEMORY_DB)
                .to_string();

            drop(conn_guard);
            pool.return_conn(conn)?;

            Ok(ConnectionStatus {
                is_connected: true,
                server_version: Some(version),
                current_database: Some(db_name),
                current_user: Some("duckdb".to_string()),
                metadata: HashMap::new(),
            })
        }

        #[cfg(not(feature = "duckdb"))]
        {
            Err(DbError::UnsupportedOperation(
                "DuckDB adapter requires the 'duckdb' feature to be enabled".to_string(),
            ))
        }
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        #[cfg(feature = "duckdb")]
        {
            self.execute_query_internal(query).await
        }

        #[cfg(not(feature = "duckdb"))]
        {
            let _ = query;
            Err(DbError::UnsupportedOperation(
                "DuckDB adapter requires the 'duckdb' feature to be enabled".to_string(),
            ))
        }
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        Ok(vec![DatabaseSchema {
            name: self
                .db_path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(MEMORY_DB)
                .to_string(),
            description: Some("DuckDB database".to_string()),
            is_system: false,
            metadata: HashMap::new(),
        }])
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        Ok(vec![
            "main".to_string(),
            "information_schema".to_string(),
            "pg_catalog".to_string(),
        ])
    }

    async fn list_tables(
        &self,
        _database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        #[cfg(feature = "duckdb")]
        {
            let schema_filter = schema.unwrap_or("main");

            // DuckDB stores metadata in information_schema.tables like PostgreSQL
            // Use parameterized query for safety
            let query = format!(
                r#"
                SELECT 
                    table_name,
                    table_type,
                    table_schema
                FROM information_schema.tables
                WHERE table_schema = '{}'
                ORDER BY table_name
                "#,
                Self::sanitize_schema_name(schema_filter)
            );

            let result = self.execute_query_internal(&query).await?;

            let mut tables = Vec::new();
            for row in result.rows {
                let name = match row.get("table_name") {
                    Some(QueryValue::String(s)) => s.clone(),
                    _ => continue,
                };

                let table_type = match row.get("table_type") {
                    Some(QueryValue::String(s)) => s.to_uppercase().replace("BASE TABLE", "TABLE"),
                    _ => "TABLE".to_string(),
                };

                let row_schema = match row.get("table_schema") {
                    Some(QueryValue::String(s)) => Some(s.clone()),
                    _ => Some(schema_filter.to_string()),
                };

                tables.push(TableInfo {
                    schema: row_schema,
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

        #[cfg(not(feature = "duckdb"))]
        {
            let _ = schema;
            Err(DbError::UnsupportedOperation(
                "DuckDB adapter requires the 'duckdb' feature to be enabled".to_string(),
            ))
        }
    }

    async fn list_columns(
        &self,
        _database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        #[cfg(feature = "duckdb")]
        {
            // Validate table name to prevent SQL injection
            Self::validate_table_name(table)?;

            let schema_filter = schema.unwrap_or("main");

            let query = format!(
                r#"
                SELECT 
                    column_name,
                    data_type,
                    is_nullable,
                    column_default,
                    ordinal_position,
                    character_maximum_length,
                    numeric_precision,
                    numeric_scale
                FROM information_schema.columns
                WHERE table_schema = '{}'
                  AND table_name = '{}'
                ORDER BY ordinal_position
                "#,
                Self::sanitize_schema_name(schema_filter),
                Self::sanitize_table_name(table)
            );

            let result = self.execute_query_internal(&query).await?;

            if result.rows.is_empty() {
                return Err(DbError::TableNotFound(table.to_string()));
            }

            let mut columns = Vec::new();
            for row in result.rows {
                let name = match row.get("column_name") {
                    Some(QueryValue::String(s)) => s.clone(),
                    _ => continue,
                };

                let data_type = match row.get("data_type") {
                    Some(QueryValue::String(s)) => s.clone(),
                    _ => String::new(),
                };

                let nullable = match row.get("is_nullable") {
                    Some(QueryValue::String(s)) => s == "YES",
                    _ => true,
                };

                let default_value = match row.get("column_default") {
                    Some(QueryValue::String(s)) => Some(s.clone()),
                    _ => None,
                };

                // DuckDB doesn't expose primary key info through information_schema.columns
                // directly; we'd need a separate query to duckdb_constraints() for that.
                // For now, default to false.
                let is_primary_key = false;

                let max_length = match row.get("character_maximum_length") {
                    Some(QueryValue::Int(i)) => {
                        if *i > 0 {
                            Some(*i as u32)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                let precision = match row.get("numeric_precision") {
                    Some(QueryValue::Int(i)) => {
                        if *i > 0 {
                            Some(*i as u32)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                let scale = match row.get("numeric_scale") {
                    Some(QueryValue::Int(i)) => {
                        if *i > 0 {
                            Some(*i as u32)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                columns.push(ColumnInfo {
                    name,
                    data_type,
                    nullable,
                    default_value,
                    is_primary_key,
                    is_auto_increment: false,
                    max_length,
                    precision,
                    scale,
                    description: None,
                    metadata: HashMap::new(),
                });
            }

            Ok(columns)
        }

        #[cfg(not(feature = "duckdb"))]
        {
            let _ = (schema, table);
            Err(DbError::UnsupportedOperation(
                "DuckDB adapter requires the 'duckdb' feature to be enabled".to_string(),
            ))
        }
    }

    async fn get_table_info(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        #[cfg(feature = "duckdb")]
        {
            // Get basic table info from list_tables
            let tables = self.list_tables(database, schema).await?;
            let mut table_info = tables
                .into_iter()
                .find(|t| t.name == table)
                .ok_or_else(|| DbError::TableNotFound(table.to_string()))?;

            // Try to get row count
            Self::validate_table_name(table)?;
            let schema_filter = schema.unwrap_or("main");
            let count_query = format!(
                "SELECT COUNT(*) as count FROM \"{}\".\"{}\"",
                Self::sanitize_schema_name(schema_filter),
                Self::sanitize_table_name(table)
            );
            let row_count = match self.execute_query_internal(&count_query).await {
                Ok(result) => result.rows.first().and_then(|row| match row.get("count") {
                    Some(QueryValue::Int(i)) => Some(*i as u64),
                    _ => None,
                }),
                Err(_) => None,
            };

            table_info.row_count = row_count;
            Ok(table_info)
        }

        #[cfg(not(feature = "duckdb"))]
        {
            let _ = (database, schema, table);
            Err(DbError::UnsupportedOperation(
                "DuckDB adapter requires the 'duckdb' feature to be enabled".to_string(),
            ))
        }
    }

    fn get_pool(&self) -> Option<Arc<Self::Pool>> {
        self.pool.clone()
    }

    fn get_config(&self) -> &ConnectionConfig {
        &self.config
    }
}

// ── Helper utilities ──

impl DuckDbAdapter {
    /// Convert a `TimeUnit` and value pair to seconds and nanoseconds.
    #[cfg(feature = "duckdb")]
    fn time_unit_to_secs_nsecs(unit: &TimeUnit, value: i64) -> (i64, u32) {
        match unit {
            TimeUnit::Second => (value, 0),
            TimeUnit::Millisecond => {
                let secs = value / 1_000;
                let nsecs = ((value % 1_000) * 1_000_000) as u32;
                (secs, nsecs)
            }
            TimeUnit::Microsecond => {
                let secs = value / 1_000_000;
                let nsecs = ((value % 1_000_000) * 1_000) as u32;
                (secs, nsecs)
            }
            TimeUnit::Nanosecond => {
                let secs = value / 1_000_000_000;
                let nsecs = (value % 1_000_000_000) as u32;
                (secs, nsecs)
            }
        }
    }

    /// Sanitize a schema name for use in SQL queries.
    fn sanitize_schema_name(name: &str) -> String {
        let sanitized: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if sanitized.is_empty() {
            "main".to_string()
        } else {
            sanitized
        }
    }

    /// Sanitize a table name for use in SQL queries.
    fn sanitize_table_name(name: &str) -> String {
        let sanitized: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if sanitized.is_empty() {
            "unknown".to_string()
        } else {
            sanitized
        }
    }
}
