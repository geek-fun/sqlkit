//! ODBC bridge adapter for enterprise databases.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! using ODBC (Open Database Connectivity) to support enterprise databases
//! such as Oracle, IBM DB2, Snowflake, DM8 (Oracle mode), XuguDB, and GBase 8a.
//!
//! # Thread Safety
//!
//! The `odbc` crate provides synchronous, non-thread-safe (`!Send`, `!Sync`) types.
//! All ODBC operations are wrapped in `tokio::task::spawn_blocking()` so the
//! async runtime is never blocked and ODBC objects never cross thread boundaries.
//!
//! # COMPATIBLE_MODE Auto-Detection
//!
//! For databases with multiple SQL dialects (DM8, OceanBase), the adapter probes
//! the server after connection to determine which dialect is active. The detected
//! mode influences schema query syntax (e.g., Oracle-style `user_tables` vs
//! MySQL-style `information_schema`).

use crate::database::{
    adapter::DatabaseAdapter,
    config::{ConnectionConfig, DatabaseType},
    error::{DbError, DbResult},
    pool::ConnectionPool,
    types::{
        ColumnInfo, ConnectionStatus, DatabaseSchema, QueryResult, QueryRow, QueryValue, TableInfo,
    },
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// The odbc crate (v0.18) re-exports odbc_safe as both `safe` module and via `pub extern crate`.
// AutocommitOn, StatementState, etc. come from odbc_safe.
use odbc::safe::AutocommitOn;
use odbc::{
    create_environment_v3, Connection, Cursor, Data, NoData, Statement,
};

// ============================================================================
// ODBC Connection Wrapper (Send-safe)
// ============================================================================

/// Dummy wrapper to satisfy the `ConnectionPool::Connection: Send` bound.
///
/// The actual ODBC connection is not held here; it is created and consumed
/// entirely inside `spawn_blocking` closures. This type exists only for the
/// trait contract.
pub struct OdbcConnection;

unsafe impl Send for OdbcConnection {}
unsafe impl Sync for OdbcConnection {}

// ============================================================================
// OdbcPool
// ============================================================================

/// ODBC connection pool.
///
/// Stores the ODBC connection string and re-creates ODBC connections on demand
/// within `tokio::task::spawn_blocking` closures.  Because `odbc::Connection` is
/// `!Send`, we cannot hold a persistent connection in the pool; instead every
/// operation opens a fresh connection, executes, and tears down.
pub struct OdbcPool {
    conn_str: String,
}

impl OdbcPool {
    /// Create a new pool from an ODBC connection string.
    pub fn new(conn_str: String) -> Self {
        Self { conn_str }
    }

    /// Build an ODBC connection string from `ConnectionConfig`.
    ///
    /// The resulting string follows the standard ODBC key=value format,
    /// e.g. `Driver={Oracle in instantclient};Server=localhost;Port=1521;...`
    pub fn build_connection_string(config: &ConnectionConfig) -> String {
        let driver = config
            .options
            .get("driver")
            .cloned()
            .unwrap_or_else(|| Self::driver_name(config.db_type).to_string());

        let mut parts = Vec::new();
        parts.push(format!("Driver={{{}}}", driver));
        parts.push(format!("Server={}", config.host));

        if config.port > 0 {
            parts.push(format!("Port={}", config.port));
        }

        if let Some(ref db) = config.database {
            parts.push(format!("Database={}", db));
        }

        parts.push(format!("UID={}", config.username));

        if let Some(ref pw) = config.password {
            parts.push(format!("PWD={}", pw));
        }

        // Append user-supplied extra options (may override any of the above)
        for (key, value) in &config.options {
            let k = key.to_lowercase();
            if k == "driver"
                || k == "server"
                || k == "port"
                || k == "database"
                || k == "uid"
                || k == "pwd"
            {
                continue; // already set above
            }
            parts.push(format!("{}={}", key, value));
        }

        parts.join(";")
    }

    /// Select a best-guess ODBC driver name for a given database type.
    fn driver_name(db_type: DatabaseType) -> &'static str {
        match db_type {
            DatabaseType::Oracle => "Oracle in instantclient",
            DatabaseType::DB2 => "IBM DB2 ODBC DRIVER",
            DatabaseType::H2 => "H2 ODBC Driver",
            DatabaseType::Snowflake => "SnowflakeDSIIDriver",
            DatabaseType::DM8Oracle => "DM8 ODBC DRIVER",
            DatabaseType::XuguDB => "XuguDB ODBC Driver",
            DatabaseType::GBase8a => "GBase 8a ODBC Driver",
            _ => "ODBC Driver",
        }
    }

    /// Execute a closure that receives a fresh ODBC connection.
    ///
    /// The environment and connection are created inside the closure and dropped
    /// when it returns. This ensures all `!Send` ODBC objects stay on a single
    /// thread.
    fn with_connection<F, T>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(&Connection<'_, AutocommitOn>) -> DbResult<T> + Send,
        T: Send,
    {
        let env = create_environment_v3().map_err(|e| {
            DbError::Connection(format!("Failed to create ODBC environment: {:?}", e))
        })?;
        let conn = env
            .connect_with_connection_string(&self.conn_str)
            .map_err(|e| DbError::Connection(format!("ODBC connection failed: {}", e)))?;
        f(&conn)
    }

    /// Execute a query and return the result set as `QueryResult`.
    fn exec_query(&self, query: &str) -> DbResult<QueryResult> {
        self.with_connection(|conn| exec_direct_and_collect(conn, query))
    }

    /// Execute a scalar query (single row, single column) and return the value.
    fn exec_scalar_string(&self, query: &str) -> DbResult<Option<String>> {
        self.with_connection(|conn| {
            let result = exec_direct_and_collect(conn, query)?;
            let val = result
                .rows
                .first()
                .and_then(|row| row.values().next())
                .and_then(|v| match v {
                    QueryValue::String(s) => Some(s.clone()),
                    QueryValue::Int(n) => Some(n.to_string()),
                    _ => None,
                });
            Ok(val)
        })
    }
}

#[async_trait]
impl ConnectionPool for OdbcPool {
    type Connection = OdbcConnection;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        // Individual connection access is not supported for ODBC because
        // odbc::Connection is !Send. All operations go through spawn_blocking
        // and create/destroy connections internally.
        Err(DbError::UnsupportedOperation(
            "ODBC connections are managed internally via spawn_blocking".to_string(),
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
        self.with_connection(|conn| {
            exec_direct_and_collect(conn, "SELECT 1")?;
            Ok(())
        })?;
        Ok(())
    }
}

// ============================================================================
// SQL Dialect Detection
// ============================================================================

/// Represents the SQL dialect to use for schema queries,
/// auto-detected from DM8 COMPATIBLE_MODE or OceanBase compatibility mode.
#[derive(Debug, Clone, PartialEq)]
enum SqlDialect {
    Oracle,
    MySql,
    PostgreSql,
    /// MSSQL / SQL Server
    SqlServer,
    /// Unknown or default — use generic ODBC catalog functions
    Generic,
}

impl SqlDialect {
    /// Map a raw compatibility mode string to a dialect.
    fn from_compatible_mode(mode: &str) -> Self {
        let m = mode.trim().to_uppercase();
        match m.as_str() {
            // DM8: 0=Oracle, 1=MySQL, 2=MSSQL, 3=PG
            "0" | "ORACLE" => SqlDialect::Oracle,
            "1" | "MYSQL" | "MARIADB" => SqlDialect::MySql,
            "2" | "MSSQL" | "SQLSERVER" | "SQL SERVER" => SqlDialect::SqlServer,
            "3" | "POSTGRESQL" | "POSTGRES" | "PG" => SqlDialect::PostgreSql,
            _ => SqlDialect::Generic,
        }
    }

    /// Schema query for listing tables (schema → name, table_type).
    fn tables_query(&self, schema: Option<&str>) -> String {
        match self {
            SqlDialect::Oracle => {
                let owner = schema
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| "USER".to_string());
                format!(
                    "SELECT table_name AS name, 'TABLE' AS table_type FROM all_tables WHERE owner = '{}' \
                     UNION ALL \
                     SELECT view_name AS name, 'VIEW' AS table_type FROM all_views WHERE owner = '{}'",
                    owner, owner
                )
            }
            SqlDialect::PostgreSql => {
                let schema_filter = schema
                    .map(|s| format!("AND schemaname = '{}'", s))
                    .unwrap_or_default();
                format!(
                    "SELECT tablename AS name, 'TABLE' AS table_type FROM pg_catalog.pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema') {} \
                     UNION ALL \
                     SELECT viewname AS name, 'VIEW' AS table_type FROM pg_catalog.pg_views WHERE schemaname NOT IN ('pg_catalog', 'information_schema') {}",
                    schema_filter, schema_filter
                )
            }
            SqlDialect::SqlServer | SqlDialect::MySql => {
                let schema_filter = schema
                    .map(|s| format!("AND table_schema = '{}'", s))
                    .unwrap_or_default();
                format!(
                    "SELECT table_name AS name, table_type FROM information_schema.tables WHERE table_schema NOT IN ('information_schema', 'sys', 'mysql', 'performance_schema') {}",
                    schema_filter
                )
            }
            SqlDialect::Generic => {
                let schema_filter = schema
                    .map(|s| format!("AND table_schema = '{}'", s))
                    .unwrap_or_default();
                format!(
                    "SELECT table_name AS name, table_type FROM information_schema.tables WHERE table_schema NOT IN ('information_schema', 'sys', 'mysql') {}",
                    schema_filter
                )
            }
        }
    }

    /// Schema query for listing columns of a given table.
    fn columns_query(&self, schema: Option<&str>, table: &str) -> String {
        match self {
            SqlDialect::Oracle => {
                let owner = schema
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| "USER".to_string());
                format!(
                    "SELECT column_name, data_type, nullable, data_default, \
                     CASE WHEN column_id IN (SELECT column_id FROM user_cons_columns WHERE constraint_name IN (SELECT constraint_name FROM user_constraints WHERE table_name = '{}' AND constraint_type = 'P')) THEN 1 ELSE 0 END AS is_pk \
                     FROM all_tab_columns WHERE owner = '{}' AND table_name = '{}' ORDER BY column_id",
                    table.to_uppercase(),
                    owner,
                    table.to_uppercase()
                )
            }
            SqlDialect::PostgreSql => {
                format!(
                    "SELECT c.column_name, c.data_type, c.is_nullable, c.column_default, \
                     CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END AS is_pk \
                     FROM information_schema.columns c \
                     LEFT JOIN (SELECT ku.column_name FROM information_schema.table_constraints tc \
                                JOIN information_schema.key_column_usage ku ON tc.constraint_name = ku.constraint_name \
                                WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_name = '{}') pk \
                     ON c.column_name = pk.column_name \
                     WHERE c.table_name = '{}' ORDER BY c.ordinal_position",
                    table, table
                )
            }
            SqlDialect::SqlServer | SqlDialect::MySql => {
                let schema_filter = schema
                    .map(|s| format!("AND c.table_schema = '{}'", s))
                    .unwrap_or_default();
                format!(
                    "SELECT c.column_name, c.data_type, c.is_nullable, c.column_default, \
                     CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END AS is_pk \
                     FROM information_schema.columns c \
                     LEFT JOIN (SELECT ku.column_name FROM information_schema.table_constraints tc \
                                JOIN information_schema.key_column_usage ku ON tc.constraint_name = ku.constraint_name \
                                WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_name = '{}') pk \
                     ON c.column_name = pk.column_name \
                     WHERE c.table_name = '{}' {} ORDER BY c.ordinal_position",
                    table, table, schema_filter
                )
            }
            SqlDialect::Generic => {
                format!(
                    "SELECT c.column_name, c.data_type, c.is_nullable, c.column_default \
                     FROM information_schema.columns c WHERE c.table_name = '{}' ORDER BY c.ordinal_position",
                    table
                )
            }
        }
    }
}

// ============================================================================
// OdbcAdapter
// ============================================================================

/// ODBC database adapter.
///
/// Supports any database with an ODBC driver by building a connection string
/// from the `ConnectionConfig` and executing SQL through ODBC. The adapter
/// auto-detects DM8 COMPATIBLE_MODE and OceanBase tenant compatibility to
/// switch SQL dialect for schema queries.
pub struct OdbcAdapter {
    pub config: ConnectionConfig,
    pool: Option<Arc<OdbcPool>>,
    /// Detected SQL dialect, stored after `connect()` probes the server.
    dialect: Arc<Mutex<SqlDialect>>,
}

impl OdbcAdapter {
    /// Create a new ODBC adapter from configuration.
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            pool: None,
            dialect: Arc::new(Mutex::new(SqlDialect::Generic)),
        }
    }

    /// Build the connection string from current config.
    fn conn_str(&self) -> String {
        OdbcPool::build_connection_string(&self.config)
    }

    /// Detect DM8 COMPATIBLE_MODE or OceanBase tenant mode and update the dialect.
    async fn detect_compatible_mode(&self) {
        let pool = match self.pool.as_ref() {
            Some(p) => p.clone(),
            None => return,
        };

        // --- DM8 probe ---
        {
            let pool = pool.clone();
            let query =
                "SELECT para_value FROM v$dm_ini WHERE para_name='COMPATIBLE_MODE'".to_string();
            match tokio::task::spawn_blocking(move || pool.exec_scalar_string(&query)).await {
                Ok(Ok(Some(mode))) => {
                    let dialect = SqlDialect::from_compatible_mode(&mode);
                    if let Ok(mut d) = self.dialect.lock() {
                        *d = dialect;
                    }
                    return; // detected, done
                }
                _ => { /* v$dm_ini not available — not DM8, continue to OceanBase probe */ }
            }
        }

        // --- OceanBase probe ---
        {
            let pool = pool.clone();
            let query = "SELECT COMPATIBILITY_MODE FROM DBA_OB_TENANTS".to_string();
            match tokio::task::spawn_blocking(move || pool.exec_scalar_string(&query)).await {
                Ok(Ok(Some(mode))) => {
                    let dialect = SqlDialect::from_compatible_mode(&mode);
                    if let Ok(mut d) = self.dialect.lock() {
                        *d = dialect;
                    }
                }
                _ => { /* not OceanBase either — keep Generic */ }
            }
        }
    }

    /// Read the current dialect (non-blocking, copies from the mutex).
    fn current_dialect(&self) -> SqlDialect {
        self.dialect
            .lock()
            .map(|d| d.clone())
            .unwrap_or(SqlDialect::Generic)
    }

    /// Execute a query via spawn_blocking.
    async fn exec_query_async(&self, query: &str) -> DbResult<QueryResult> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?
            .clone();
        let sql = query.to_string();
        tokio::task::spawn_blocking(move || pool.exec_query(&sql))
            .await
            .map_err(|e| DbError::Connection(format!("spawn_blocking join error: {}", e)))?
    }

    /// Execute a scalar query via spawn_blocking.
    async fn exec_scalar_async(&self, query: &str) -> DbResult<Option<String>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?
            .clone();
        let sql = query.to_string();
        tokio::task::spawn_blocking(move || pool.exec_scalar_string(&sql))
            .await
            .map_err(|e| DbError::Connection(format!("spawn_blocking join error: {}", e)))?
    }
}

// ============================================================================
// DatabaseAdapter impl
// ============================================================================

#[async_trait]
impl DatabaseAdapter for OdbcAdapter {
    type Pool = OdbcPool;

    async fn connect(&mut self) -> DbResult<()> {
        let conn_str = self.conn_str();
        let pool = OdbcPool::new(conn_str);

        // Verify connectivity before accepting
        pool.health_check().await?;

        self.pool = Some(Arc::new(pool));

        // Auto-detect DM8 COMPATIBLE_MODE / OceanBase tenant mode
        self.detect_compatible_mode().await;

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
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?
            .clone();

        tokio::task::spawn_blocking(move || -> DbResult<ConnectionStatus> {
            pool.with_connection(|_conn| {
                // ODBC does not provide a standard way to query server version
                // without driver-specific SQL. We just report "connected".
                Ok(ConnectionStatus {
                    is_connected: true,
                    server_version: Some("ODBC".to_string()),
                    current_database: None,
                    current_user: None,
                    metadata: HashMap::new(),
                })
            })
        })
        .await
        .map_err(|e| DbError::Connection(format!("spawn_blocking join error: {}", e)))?
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        self.exec_query_async(query).await
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        // List databases is not universally supported via ODBC without
        // driver-specific queries. Return the configured database name as a
        // single entry when available.
        let databases = if let Some(ref db) = self.config.database {
            vec![DatabaseSchema {
                name: db.clone(),
                description: Some("ODBC connection".to_string()),
                is_system: false,
                metadata: HashMap::new(),
            }]
        } else {
            Vec::new()
        };
        Ok(databases)
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        let query = match self.current_dialect() {
            SqlDialect::Oracle => {
                "SELECT username FROM all_users ORDER BY username".to_string()
            }
            SqlDialect::PostgreSql => {
                "SELECT schema_name FROM information_schema.schemata WHERE schema_name NOT IN ('pg_catalog', 'information_schema') ORDER BY schema_name".to_string()
            }
            SqlDialect::MySql | SqlDialect::SqlServer | SqlDialect::Generic => {
                "SELECT schema_name FROM information_schema.schemata ORDER BY schema_name"
                    .to_string()
            }
        };

        let result = self.exec_query_async(&query).await?;
        let schemas: Vec<String> = result
            .rows
            .iter()
            .filter_map(|row| {
                row.values().next().and_then(|v| match v {
                    QueryValue::String(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .collect();
        Ok(schemas)
    }

    async fn list_tables(
        &self,
        _database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let dialect = self.current_dialect();
        let query = dialect.tables_query(schema);
        let result = self.exec_query_async(&query).await?;

        let tables: Vec<TableInfo> = result
            .rows
            .iter()
            .filter_map(|row| {
                let name = row.get("name").and_then(|v| match v {
                    QueryValue::String(s) => Some(s.clone()),
                    _ => None,
                })?;
                let table_type = row
                    .get("table_type")
                    .and_then(|v| match v {
                        QueryValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "TABLE".to_string());

                Some(TableInfo {
                    schema: schema.map(|s| s.to_string()),
                    name,
                    table_type,
                    row_count: None,
                    size_bytes: None,
                    description: None,
                    metadata: HashMap::new(),
                })
            })
            .collect();

        Ok(tables)
    }

    async fn list_columns(
        &self,
        _database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        let dialect = self.current_dialect();
        let query = dialect.columns_query(schema, table);
        let result = self.exec_query_async(&query).await?;

        let columns: Vec<ColumnInfo> = result
            .rows
            .iter()
            .map(|row| {
                let name = row
                    .get("column_name")
                    .and_then(|v| match v {
                        QueryValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let data_type = row
                    .get("data_type")
                    .and_then(|v| match v {
                        QueryValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                let nullable = row
                    .get("nullable")
                    .and_then(|v| match v {
                        QueryValue::String(s) => Some(s == "YES"),
                        _ => None,
                    })
                    .unwrap_or(true);
                let default_value = row.get("column_default").and_then(|v| match v {
                    QueryValue::String(s) => Some(s.clone()),
                    QueryValue::Null => None,
                    _ => None,
                });
                let is_pk = row
                    .get("is_pk")
                    .and_then(|v| match v {
                        QueryValue::Int(n) => Some(*n > 0),
                        QueryValue::String(s) => Some(s == "true" || s == "1" || s == "YES"),
                        _ => None,
                    })
                    .unwrap_or(false);

                ColumnInfo {
                    name,
                    data_type,
                    nullable,
                    default_value,
                    is_primary_key: is_pk,
                    is_auto_increment: false,
                    max_length: None,
                    precision: None,
                    scale: None,
                    description: None,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(columns)
    }

    async fn get_table_info(
        &self,
        _database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        let dialect = self.current_dialect();
        let row_count_q = match dialect {
            SqlDialect::Oracle => {
                let owner = schema
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| "USER".to_string());
                format!(
                    "SELECT num_rows FROM all_tables WHERE owner = '{}' AND table_name = '{}'",
                    owner,
                    table.to_uppercase()
                )
            }
            SqlDialect::PostgreSql => {
                format!(
                    "SELECT n_live_tup FROM pg_stat_user_tables WHERE relname = '{}'",
                    table
                )
            }
            SqlDialect::MySql | SqlDialect::SqlServer | SqlDialect::Generic => {
                format!(
                    "SELECT table_rows FROM information_schema.tables WHERE table_name = '{}'",
                    table
                )
            }
        };

        let row_count = self
            .exec_scalar_async(&row_count_q)
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse::<u64>().ok());

        Ok(TableInfo {
            schema: schema.map(|s| s.to_string()),
            name: table.to_string(),
            table_type: "TABLE".to_string(),
            row_count,
            size_bytes: None,
            description: None,
            metadata: HashMap::new(),
        })
    }

    fn get_pool(&self) -> Option<Arc<Self::Pool>> {
        self.pool.clone()
    }

    fn get_config(&self) -> &ConnectionConfig {
        &self.config
    }
}

// ============================================================================
// ODBC Helper Functions (synchronous — called inside spawn_blocking)
// ============================================================================

/// Execute a SQL query directly on an ODBC connection and collect results into
/// a `QueryResult`.
fn exec_direct_and_collect(
    conn: &Connection<'_, AutocommitOn>,
    query: &str,
) -> DbResult<QueryResult> {
    let stmt =
        Statement::with_parent(conn).map_err(|e| DbError::QueryExecution(format!("{}", e)))?;

    match stmt
        .exec_direct(query)
        .map_err(|e| DbError::QueryExecution(format!("{}", e)))?
    {
        Data(mut stmt) => {
            // SELECT-like query — fetch rows
            let num_cols = stmt
                .num_result_cols()
                .map_err(|e| DbError::QueryExecution(format!("{}", e)))?;

            // Collect column names (ODBC columns are 1-indexed)
            let mut columns = Vec::new();
            for idx in 1..=num_cols {
                let desc = stmt
                    .describe_col(idx as u16)
                    .map_err(|e| DbError::QueryExecution(format!("{}", e)))?;
                columns.push(desc.name.to_string());
            }

            let mut rows = Vec::new();
            loop {
                let cursor = stmt
                    .fetch()
                    .map_err(|e| DbError::QueryExecution(format!("{}", e)))?;
                match cursor {
                    Some(mut cursor) => {
                        let mut row: HashMap<String, QueryValue> = HashMap::new();
                        for (col_idx, col_name) in columns.iter().enumerate() {
                            let value = get_cell_value(&mut cursor, (col_idx + 1) as u16);
                            row.insert(col_name.clone(), value);
                        }
                        rows.push(row);
                    }
                    None => break,
                }
            }

            Ok(QueryResult {
                columns,
                rows,
                rows_affected: None,
                execution_time_ms: None,
            })
        }
        NoData(stmt) => {
            // INSERT / UPDATE / DELETE — return affected row count
            let rows_affected = stmt
                .affected_row_count()
                .map_err(|e| DbError::QueryExecution(format!("{}", e)))?;
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                rows_affected: Some(rows_affected as u64),
                execution_time_ms: None,
            })
        }
    }
}

/// Extract a cell value from an ODBC cursor at the given column index (1-based).
///
/// Tries String first (covers most data types via ODBC conversion), then binary
/// for BLOBs. If both fail or the column is NULL, returns `QueryValue::Null`.
/// Does NOT return an error so that a single bad cell does not break the entire
/// result set.
fn get_cell_value<S>(
    cursor: &mut Cursor<'_, '_, '_, S, AutocommitOn>,
    col: u16,
) -> QueryValue {
    // String conversion is the most universal — ODBC drivers can convert
    // numeric, date, and text columns to strings.
    match cursor.get_data::<String>(col) {
        Ok(Some(s)) => return QueryValue::String(s),
        Ok(None) => return QueryValue::Null,
        Err(_) => { /* try next format */ }
    }

    // Binary fallback for BLOB / VARBINARY columns
    match cursor.get_data::<Vec<u8>>(col) {
        Ok(Some(b)) => return QueryValue::Bytes(b),
        Ok(None) => return QueryValue::Null,
        Err(_) => { /* give up */ }
    }

    QueryValue::Null
}
