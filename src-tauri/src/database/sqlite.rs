//! SQLite database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for SQLite databases using rusqlite with thread-safe connection management.

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
use rusqlite::{types::ValueRef, Connection, OpenFlags, Row};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Wrapper for rusqlite::Connection to implement Send + Sync
/// SAFETY: This is safe because we always access the connection through a Mutex
/// in the pool, ensuring exclusive access across threads.
pub struct SendableConnection(pub Connection);

unsafe impl Send for SendableConnection {}
unsafe impl Sync for SendableConnection {}

impl std::ops::Deref for SendableConnection {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SendableConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Constant for in-memory database identifier.
const MEMORY_DB: &str = ":memory:";

/// SQLite connection pool wrapper.
///
/// SQLite has limited concurrency support compared to client-server databases.
/// This pool manages connections with proper synchronization for thread-safety.
pub struct SQLitePool {
    connections: Arc<Mutex<Vec<Arc<Mutex<Connection>>>>>,
    max_connections: usize,
    db_path: Option<PathBuf>,
}

impl SQLitePool {
    /// Create a new SQLite connection pool.
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

    /// Create a new SQLite connection with proper configuration.
    fn create_connection(&self) -> DbResult<Connection> {
        let conn = if let Some(ref path) = self.db_path {
            Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_WRITE
                    | OpenFlags::SQLITE_OPEN_CREATE
                    | OpenFlags::SQLITE_OPEN_URI
                    | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(|e| DbError::Connection(format!("Failed to open database: {}", e)))?
        } else {
            // In-memory database
            Connection::open_in_memory().map_err(|e| {
                DbError::Connection(format!("Failed to open in-memory database: {}", e))
            })?
        };

        // Enable Write-Ahead Logging (WAL) mode for better concurrency
        // WAL mode allows multiple readers and one writer to proceed concurrently
        if self.db_path.is_some() {
            conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))
                .map_err(|e| DbError::Configuration(format!("Failed to enable WAL mode: {}", e)))?;
        }

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys=ON", [])
            .map_err(|e| DbError::Configuration(format!("Failed to enable foreign keys: {}", e)))?;

        Ok(conn)
    }
}

#[async_trait]
impl ConnectionPool for SQLitePool {
    type Connection = SendableConnection;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        // NOTE: This method is not used in the current implementation.
        // SQLite connections are managed through the custom get_conn() method instead,
        // which returns Arc<Mutex<Connection>> for proper thread-safety.
        // The ConnectionPool trait was designed for client-server databases (Postgres, MySQL)
        // where connections can be safely shared as Arc<Connection>. SQLite requires
        // Mutex wrapping for thread-safety, which doesn't fit cleanly into this trait design.
        // This is a known architectural limitation. Future improvements could:
        // 1. Make ConnectionPool trait generic over the wrapper type
        // 2. Create a separate trait for embedded databases
        // 3. Continue with current approach where this method is intentionally unused
        Err(DbError::UnsupportedOperation(
            "Direct connection access not supported - use get_conn() instead".to_string(),
        ))
    }

    async fn return_connection(&self, connection: Arc<Self::Connection>) -> DbResult<()> {
        // Immediately drop the connection to avoid Send issues
        // rusqlite::Connection is not Send because it contains RefCell
        std::mem::drop(connection);

        // NOTE: This method is not actually used in SQLite's implementation.
        // Connection return is handled by the custom return_conn() method which
        // expects Arc<Mutex<Connection>> to match our thread-safety requirements.
        // The parameter is dropped immediately to satisfy the trait requirement
        // while avoiding Send trait bounds issues with rusqlite::Connection.

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

/// SQLite database adapter.
///
/// Supports both file-based and in-memory SQLite databases with proper
/// thread-safety through connection pooling and Write-Ahead Logging (WAL) mode.
pub struct SQLiteAdapter {
    config: ConnectionConfig,
    pool: Option<Arc<SQLitePool>>,
    db_path: Option<PathBuf>,
}

impl SQLiteAdapter {
    /// Create a new SQLite adapter with the given configuration.
    ///
    /// The database path can be specified in the config.database field:
    /// - For file-based databases: Use a file path (e.g., "/path/to/db.sqlite")
    /// - For in-memory databases: Use ":memory:" or leave database as None
    pub fn new(config: ConnectionConfig) -> Self {
        let db_path = config.database.as_ref().and_then(|db| {
            if db == MEMORY_DB {
                None
            } else {
                Some(PathBuf::from(db))
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

    /// Convert a rusqlite Row to QueryRow.
    fn row_to_query_row(row: &Row) -> DbResult<QueryRow> {
        let mut query_row = HashMap::new();
        let column_count = row.as_ref().column_count();

        for idx in 0..column_count {
            let name = row
                .as_ref()
                .column_name(idx)
                .map_err(|e| DbError::QueryExecution(format!("Failed to get column name: {}", e)))?
                .to_string();
            let value = Self::convert_value(row, idx)?;
            query_row.insert(name, value);
        }

        Ok(query_row)
    }

    /// Convert a SQLite value to QueryValue.
    fn convert_value(row: &Row, idx: usize) -> DbResult<QueryValue> {
        let value_ref = row
            .get_ref(idx)
            .map_err(|e| DbError::TypeConversion(format!("Failed to get value: {}", e)))?;

        match value_ref {
            ValueRef::Null => Ok(QueryValue::Null),
            ValueRef::Integer(i) => Ok(QueryValue::Int(i)),
            ValueRef::Real(f) => Ok(QueryValue::Float(f)),
            ValueRef::Text(t) => {
                let s = std::str::from_utf8(t)
                    .map_err(|e| DbError::TypeConversion(format!("Invalid UTF-8: {}", e)))?;
                Ok(QueryValue::String(s.to_string()))
            }
            ValueRef::Blob(b) => Ok(QueryValue::Bytes(b.to_vec())),
        }
    }

    /// Execute a query and return results.
    async fn execute_query_internal(&self, query: &str) -> DbResult<QueryResult> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let conn = pool.get_conn().await?;
        let conn_guard = conn
            .lock()
            .map_err(|e| DbError::QueryExecution(format!("Failed to lock connection: {}", e)))?;

        // Check if this is a SELECT query
        let trimmed = query.trim().to_uppercase();
        let is_select = trimmed.starts_with("SELECT")
            || trimmed.starts_with("PRAGMA")
            || trimmed.starts_with("EXPLAIN");

        if is_select {
            let mut stmt = conn_guard
                .prepare(query)
                .map_err(|e| DbError::QueryExecution(format!("Failed to prepare query: {}", e)))?;

            let column_count = stmt.column_count();
            let columns: Vec<String> = (0..column_count)
                .map(|i| stmt.column_name(i).unwrap_or("unknown").to_string())
                .collect();

            let rows_iter = stmt
                .query_map([], |row| Ok(Self::row_to_query_row(row)))
                .map_err(|e| DbError::QueryExecution(format!("Failed to execute query: {}", e)))?;

            let mut rows = Vec::new();
            for row_result in rows_iter {
                let row = row_result
                    .map_err(|e| DbError::QueryExecution(format!("Failed to fetch row: {}", e)))?
                    .map_err(|e| {
                        DbError::QueryExecution(format!("Failed to convert row: {}", e))
                    })?;
                rows.push(row);
            }

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
            // For non-SELECT queries (INSERT, UPDATE, DELETE, etc.)
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
impl DatabaseAdapter for SQLiteAdapter {
    type Pool = SQLitePool;

    async fn connect(&mut self) -> DbResult<()> {
        // For in-memory databases, force max_connections = 1 because each
        // Connection::open_in_memory() creates a separate isolated database.
        // With multiple connections, CREATE TABLE on one connection won't be
        // visible to other connections, causing "no such table" errors.
        let max_connections = if self.db_path.is_none() {
            1 // Single connection for in-memory databases
        } else {
            self.config.pool_config.max_connections as usize
        };

        let pool = SQLitePool::new(max_connections, self.db_path.clone());

        // Test the connection by creating one
        let conn = pool.get_conn().await?;
        pool.return_conn(conn)?;

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
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let conn = pool.get_conn().await?;
        let conn_guard = conn
            .lock()
            .map_err(|e| DbError::Connection(format!("Failed to lock connection: {}", e)))?;

        // Get SQLite version
        let version: String = conn_guard
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
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
            current_user: Some("local".to_string()),
            metadata: HashMap::new(),
        })
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        self.execute_query_internal(query).await
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        Ok(vec![DatabaseSchema {
            name: self
                .db_path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(MEMORY_DB)
                .to_string(),
            description: Some("SQLite database".to_string()),
            is_system: false,
            metadata: HashMap::new(),
        }])
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        // SQLite doesn't support schemas in the traditional sense
        // Return a single default schema
        Ok(vec!["main".to_string()])
    }

    async fn list_tables(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let query = r#"
            SELECT 
                name,
                type
            FROM sqlite_master
            WHERE type IN ('table', 'view')
            AND name NOT LIKE 'sqlite_%'
            ORDER BY name
        "#;

        let result = self.execute_query_internal(query).await?;

        let mut tables = Vec::new();
        for row in result.rows {
            let name = match row.get("name") {
                Some(QueryValue::String(s)) => s.clone(),
                _ => continue,
            };

            let table_type = match row.get("type") {
                Some(QueryValue::String(s)) => s.to_uppercase(),
                _ => "TABLE".to_string(),
            };

            tables.push(TableInfo {
                schema: Some("main".to_string()),
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
        _schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        // Validate table name to prevent SQL injection
        // This uses a strict whitelist approach (alphanumeric, underscore, dot only)
        Self::validate_table_name(table)?;

        // SAFETY: After validation, table name is guaranteed to only contain safe characters.
        // Using double quotes for SQL standard identifier quoting.
        // Note: PRAGMA statements in SQLite don't support parameter binding,
        // so we use string formatting with validated input as a defense-in-depth approach.
        let query = format!("PRAGMA table_info(\"{}\")", table);
        let result = self.execute_query_internal(&query).await?;

        if result.rows.is_empty() {
            return Err(DbError::TableNotFound(table.to_string()));
        }

        let mut columns = Vec::new();
        for row in result.rows {
            let name = match row.get("name") {
                Some(QueryValue::String(s)) => s.clone(),
                _ => continue,
            };

            let data_type = match row.get("type") {
                Some(QueryValue::String(s)) => s.clone(),
                _ => "".to_string(),
            };

            let notnull = match row.get("notnull") {
                Some(QueryValue::Int(i)) => *i != 0,
                _ => false,
            };

            let default_value = match row.get("dflt_value") {
                Some(QueryValue::String(s)) => Some(s.clone()),
                Some(QueryValue::Null) => None,
                _ => None,
            };

            let is_primary_key = match row.get("pk") {
                Some(QueryValue::Int(i)) => *i > 0,
                _ => false,
            };

            let nullable = !notnull && !is_primary_key;

            columns.push(ColumnInfo {
                name,
                data_type,
                nullable,
                default_value,
                is_primary_key,
                is_auto_increment: false, // SQLite doesn't expose this easily
                max_length: None,
                precision: None,
                scale: None,
                description: None,
                metadata: HashMap::new(),
            });
        }

        Ok(columns)
    }

    async fn get_table_info(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        // Get basic table info
        let tables = self.list_tables(database, schema).await?;
        let table_info = tables
            .into_iter()
            .find(|t| t.name == table)
            .ok_or_else(|| DbError::TableNotFound(table.to_string()))?;

        // Try to get row count
        // Validate table name to prevent SQL injection
        // This uses a strict whitelist approach (alphanumeric, underscore, dot only)
        Self::validate_table_name(table)?;

        // SAFETY: After validation, table name is guaranteed to only contain safe characters.
        // Using double quotes for SQL standard identifier quoting.
        let count_query = format!("SELECT COUNT(*) as count FROM \"{}\"", table);
        let row_count = match self.execute_query_internal(&count_query).await {
            Ok(result) => result
                .rows
                .first()
                .and_then(|row| row.get("count"))
                .and_then(|v| match v {
                    QueryValue::Int(i) => Some(*i as u64),
                    _ => None,
                }),
            Err(_) => None,
        };

        Ok(TableInfo {
            row_count,
            ..table_info
        })
    }

    fn get_pool(&self) -> Option<Arc<Self::Pool>> {
        self.pool.clone()
    }

    fn get_config(&self) -> &ConnectionConfig {
        &self.config
    }
}
