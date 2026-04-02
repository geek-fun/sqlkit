//! MySQL database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for MySQL databases using mysql_async with connection pooling support.

use crate::database::{
    adapter::DatabaseAdapter,
    config::{ConnectionConfig, SslMode},
    error::{DbError, DbResult},
    pool::ConnectionPool,
    types::{
        ColumnInfo, ConnectionStatus, DatabaseSchema, QueryResult, QueryRow, QueryValue, TableInfo,
    },
};
use async_trait::async_trait;
use mysql_async::{
    prelude::*,
    Conn, OptsBuilder, Pool, PoolConstraints, PoolOpts, Row, SslOpts, Value,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Convert MySQL error to DbError with detailed information for query errors.
fn mysql_error_to_db_error(error: mysql_async::Error) -> DbError {
    if let mysql_async::Error::Server(server_error) = &error {
        let mut details = vec![server_error.message.clone()];
        details.push(format!("[Error Code: {}]", server_error.code));
        details.push(format!("[SQL State: {}]", server_error.state));
        DbError::QueryExecution(details.join(" "))
    } else {
        DbError::QueryExecution(error.to_string())
    }
}

/// Convert MySQL connection error to DbError with detailed information.
/// Handles authentication failures and connection errors specifically.
fn mysql_connection_error_to_db_error(error: mysql_async::Error) -> DbError {
    if let mysql_async::Error::Server(server_error) = &error {
        // MySQL access denied errors: error code 1045 (28000)
        // MySQL unknown database errors: error code 1049 (42000)
        let is_auth_error = server_error.code == 1045 || server_error.state.starts_with("28");
        
        if is_auth_error {
            DbError::Authentication(format!("{} - {}", server_error.message, server_error.state))
        } else {
            DbError::Connection(format!("{} [Error Code: {}]", server_error.message, server_error.code))
        }
    } else {
        DbError::Connection(error.to_string())
    }
}

/// Configuration key for query timeout in milliseconds.
/// Use this key in ConnectionConfig options to set a timeout for query execution.
/// Example: `config.with_option(STATEMENT_TIMEOUT_KEY, "5000")` for 5 second timeout.
const STATEMENT_TIMEOUT_KEY: &str = "statement_timeout";

/// MySQL connection pool wrapper.
pub struct MySQLPool {
    pool: Pool,
}

#[async_trait]
impl ConnectionPool for MySQLPool {
    type Connection = Conn;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        // Note: Similar to PostgresPool, this method has limitations with mysql_async's pool design
        // The pool uses internal connection guards that are better accessed directly
        Err(DbError::UnsupportedOperation(
            "Direct connection access not supported with mysql_async pool - use pool.get_conn() directly"
                .to_string(),
        ))
    }

    async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
        // Connection is automatically returned when dropped
        Ok(())
    }

    fn active_connections(&self) -> usize {
        // Note: mysql_async doesn't expose detailed pool statistics
        // Returning 0 as a placeholder since actual metrics are not available
        0
    }

    fn idle_connections(&self) -> usize {
        // Note: mysql_async doesn't expose detailed pool statistics
        // Returning 0 as a placeholder since actual metrics are not available
        0
    }

    fn max_connections(&self) -> usize {
        // Note: mysql_async doesn't expose pool configuration after creation
        // Returning default value as actual configuration is not accessible
        10
    }

    async fn close(&self) -> DbResult<()> {
        // Disconnect all connections in the pool
        self.pool            .clone()            .disconnect()
            .await
            .map_err(|e| DbError::PoolError(format!("Failed to close pool: {}", e)))?;
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        let mut conn = self
            .pool
            .get_conn()
            .await
            .map_err(|e| DbError::PoolError(format!("Health check failed: {}", e)))?;

        conn.query_drop("SELECT 1")
            .await
            .map_err(|e| DbError::PoolError(format!("Health check query failed: {}", e)))?;

        Ok(())
    }
}

/// MySQL database adapter.
pub struct MySQLAdapter {
    pub(crate) config: ConnectionConfig,
    pool: Option<Arc<MySQLPool>>,
    // Store the raw pool for internal use
    raw_pool: Option<Pool>,
}

impl MySQLAdapter {
    /// Create a new MySQL adapter with the given configuration.
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            pool: None,
            raw_pool: None,
        }
    }

    /// Build MySQL connection options.
    fn build_connection_opts(&self) -> DbResult<OptsBuilder> {
        let mut opts = OptsBuilder::default()
            .ip_or_hostname(&self.config.host)
            .tcp_port(self.config.port)
            .user(Some(&self.config.username));

        if let Some(ref password) = self.config.password {
            opts = opts.pass(Some(password));
        }

        if let Some(ref database) = self.config.database {
            opts = opts.db_name(Some(database));
        }

        // Configure SSL/TLS
        match self.config.ssl_mode {
            SslMode::Disable => {
                opts = opts.ssl_opts(None);
            }
            SslMode::Prefer | SslMode::Require => {
                let ssl_opts = SslOpts::default();
                opts = opts.ssl_opts(Some(ssl_opts));
            }
            SslMode::VerifyCA | SslMode::VerifyFull => {
                let mut ssl_opts = SslOpts::default();

                if let Some(ref ca_cert) = self.config.ssl_ca_cert {
                    let ca_path: std::path::PathBuf = ca_cert.into();
                    ssl_opts = ssl_opts.with_root_certs(vec![ca_path.into()]);
                }

                opts = opts.ssl_opts(Some(ssl_opts));
            }
        }

        // Note: tcp_connect_timeout method removed in newer mysql_async versions
        // Connection timeout is handled at TCP level

        // Apply pool configuration
        let pool_opts = PoolOpts::default()
            .with_constraints(
                PoolConstraints::new(
                    self.config.pool_config.min_connections as usize,
                    self.config.pool_config.max_connections as usize,
                )
                .unwrap(),
            )
            .with_inactive_connection_ttl(self.config.pool_config.idle_timeout);
            // Note: with_ttl method removed in newer versions

        opts = opts.pool_opts(pool_opts);

        Ok(opts)
    }

    /// Convert a mysql_async Row to QueryRow.
    fn row_to_query_row(row: Row) -> DbResult<QueryRow> {
        let mut query_row = HashMap::new();
        let columns = row.columns_ref();

        for (idx, column) in columns.iter().enumerate() {
            let name = column.name_str().to_string();
            // Use safe conversion with fallback
            let value = Self::convert_value_safe(&row, idx);
            query_row.insert(name, value);
        }

        Ok(query_row)
    }

    /// Safely convert a MySQL value to QueryValue with fallback.
    fn convert_value_safe(row: &Row, idx: usize) -> QueryValue {
        match Self::convert_value(row, idx) {
            Ok(value) => value,
            Err(_) => {
                // Fallback to string representation
                match row.get::<Value, _>(idx) {
                    Some(val) => QueryValue::String(format!("{:?}", val)),
                    None => QueryValue::Null,
                }
            }
        }
    }

    /// Convert a MySQL value to QueryValue.
    fn convert_value(row: &Row, idx: usize) -> DbResult<QueryValue> {
        let value: Value = row
            .get(idx)
            .ok_or_else(|| DbError::TypeConversion(format!("Column {} not found", idx)))?;

        match value {
            Value::NULL => Ok(QueryValue::Null),
            Value::Bytes(bytes) => {
                // Try to convert to UTF-8 string first, fallback to bytes
                match String::from_utf8(bytes.clone()) {
                    Ok(s) => Ok(QueryValue::String(s)),
                    Err(_) => Ok(QueryValue::Bytes(bytes)),
                }
            }
            Value::Int(i) => Ok(QueryValue::Int(i)),
            Value::UInt(u) => Ok(QueryValue::Int(u as i64)),
            Value::Float(f) => Ok(QueryValue::Float(f as f64)),
            Value::Double(d) => Ok(QueryValue::Float(d)),
            Value::Date(year, month, day, hour, minute, second, micros) => {
                let datetime = format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                    year, month, day, hour, minute, second, micros
                );
                Ok(QueryValue::DateTime(datetime))
            }
            Value::Time(is_neg, days, hours, minutes, seconds, micros) => {
                let sign = if is_neg { "-" } else { "" };
                let time = format!(
                    "{}{}d {:02}:{:02}:{:02}.{:06}",
                    sign, days, hours, minutes, seconds, micros
                );
                Ok(QueryValue::String(time))
            }
        }
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> DbResult<Conn> {
        let pool = self
            .raw_pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        pool.get_conn()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to get connection: {}", e)))
    }
}

#[async_trait]
impl DatabaseAdapter for MySQLAdapter {
    type Pool = MySQLPool;

    async fn connect(&mut self) -> DbResult<()> {
        let opts = self.build_connection_opts()?;
        let pool = Pool::new(opts);

        let mut conn = pool
            .get_conn()
            .await
            .map_err(mysql_connection_error_to_db_error)?;

        conn.query_drop("SELECT 1")
            .await
            .map_err(mysql_connection_error_to_db_error)?;

        drop(conn);

        self.raw_pool = Some(pool.clone());
        self.pool = Some(Arc::new(MySQLPool { pool }));

        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        if let Some(pool) = self.pool.take() {
            pool.close().await?;
        }
        self.raw_pool = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let mut conn = self.get_conn().await?;

        // Get server version
        let version_row: Row = conn
            .query_first("SELECT VERSION() as version")
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?
            .ok_or_else(|| DbError::QueryExecution("Failed to get version".to_string()))?;
        let server_version: String = version_row.get("version").unwrap();

        // Get current database and user
        let db_row: Row = conn
            .query_first("SELECT DATABASE() as db, USER() as user")
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?
            .ok_or_else(|| DbError::QueryExecution("Failed to get database info".to_string()))?;
        let current_database: Option<String> = db_row.get("db");
        let current_user: String = db_row.get("user").unwrap();

        Ok(ConnectionStatus {
            is_connected: true,
            server_version: Some(server_version),
            current_database,
            current_user: Some(current_user),
            metadata: HashMap::new(),
        })
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let mut conn = self.get_conn().await?;
        let start = Instant::now();

        // Check if we have a query timeout configured
        let timeout = self
            .config
            .options
            .get(STATEMENT_TIMEOUT_KEY)
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_millis);

        // Determine if this is a query that returns rows
        let query_trimmed = query.trim().to_uppercase();
        let is_select = query_trimmed.starts_with("SELECT")
            || query_trimmed.starts_with("SHOW")
            || query_trimmed.starts_with("DESCRIBE")
            || query_trimmed.starts_with("EXPLAIN");

        let execution_time;

        if is_select {
            // Execute query and get results
            let result: Vec<Row> = if let Some(timeout_duration) = timeout {
                tokio::time::timeout(timeout_duration, conn.query(query))
                    .await
                    .map_err(|_| {
                        DbError::Timeout(format!("Query timed out after {:?}", timeout_duration))
                    })?
                    .map_err(mysql_error_to_db_error)?
            } else {
                conn.query(query)
                    .await
                    .map_err(mysql_error_to_db_error)?
            };

            execution_time = start.elapsed().as_millis() as u64;

            if result.is_empty() {
                Ok(QueryResult::new(Vec::new()).with_execution_time(execution_time))
            } else {
                let columns: Vec<String> = result[0]
                    .columns_ref()
                    .iter()
                    .map(|col| col.name_str().to_string())
                    .collect();

                let mut query_result = QueryResult::new(columns);
                for row in result {
                    let query_row = Self::row_to_query_row(row)?;
                    query_result.add_row(query_row);
                }

                Ok(query_result.with_execution_time(execution_time))
            }
        } else {
            // For INSERT, UPDATE, DELETE, etc.
            if let Some(timeout_duration) = timeout {
                tokio::time::timeout(timeout_duration, conn.query_drop(query))
                    .await
                    .map_err(|_| {
                        DbError::Timeout(format!("Query timed out after {:?}", timeout_duration))
                    })?
                    .map_err(mysql_error_to_db_error)?
            } else {
                conn.query_drop(query)
                    .await
                    .map_err(mysql_error_to_db_error)?
            };

            execution_time = start.elapsed().as_millis() as u64;

            // Get affected rows
            let affected = conn.affected_rows();

            Ok(QueryResult::affected(affected).with_execution_time(execution_time))
        }
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        let mut conn = self.get_conn().await?;

        let query = "SHOW DATABASES";
        let rows: Vec<Row> = conn
            .query(query)
            .await
            .map_err(mysql_error_to_db_error)?;

        let databases = rows
            .into_iter()
            .map(|row| {
                let name: String = row.get(0).unwrap();
                DatabaseSchema {
                    name,
                    description: None,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(databases)
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        // MySQL doesn't have separate schemas like PostgreSQL
        // In MySQL, databases are the top-level namespace
        // Return the list of databases instead
        let databases = self.list_databases().await?;
        Ok(databases.into_iter().map(|db| db.name).collect())
    }

    async fn list_tables(
        &self,
        database: Option<&str>,
        _schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let mut conn = self.get_conn().await?;

        // If a specific database is requested, use it
        let db_name = database.or(self.config.database.as_deref());

        let query = if let Some(db) = db_name {
            format!("SHOW FULL TABLES FROM `{}`", db)
        } else {
            "SHOW FULL TABLES".to_string()
        };

        let rows: Vec<Row> = conn
            .query(&query)
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let tables = rows
            .into_iter()
            .map(|row| {
                let name: String = row.get(0).unwrap();
                let table_type_raw: String = row.get(1).unwrap();
                let table_type = if table_type_raw.to_uppercase() == "BASE TABLE" {
                    "TABLE".to_string()
                } else {
                    table_type_raw.to_uppercase()
                };

                TableInfo {
                    schema: db_name.map(|s| s.to_string()),
                    name,
                    table_type,
                    row_count: None,
                    size_bytes: None,
                    description: None,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(tables)
    }

    async fn list_columns(
        &self,
        database: Option<&str>,
        _schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        let mut conn = self.get_conn().await?;

        // If a specific database is requested, use it
        let db_name = database
            .or(self.config.database.as_deref())
            .ok_or_else(|| DbError::Configuration("No database specified".to_string()))?;

        let query = format!(
            "SELECT 
                COLUMN_NAME,
                DATA_TYPE,
                IS_NULLABLE,
                COLUMN_DEFAULT,
                CHARACTER_MAXIMUM_LENGTH,
                NUMERIC_PRECISION,
                NUMERIC_SCALE,
                COLUMN_KEY,
                EXTRA,
                COLUMN_COMMENT
            FROM INFORMATION_SCHEMA.COLUMNS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY ORDINAL_POSITION"
        );

        let rows: Vec<Row> = conn
            .exec(&query, (db_name, table))
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let columns = rows
            .into_iter()
            .map(|row| {
                let name: String = row.get(0).unwrap();
                let data_type: String = row.get(1).unwrap();
                let is_nullable: String = row.get(2).unwrap();
                let default_value: Option<String> = row.get(3);
                let max_length: Option<u32> = row.get(4);
                let precision: Option<u32> = row.get(5);
                let scale: Option<u32> = row.get(6);
                let column_key: String = row.get(7).unwrap_or_default();
                let extra: String = row.get(8).unwrap_or_default();
                let description: Option<String> = row.get(9);

                let is_primary_key = column_key.to_uppercase().contains("PRI");
                let is_auto_increment = extra.to_uppercase().contains("AUTO_INCREMENT");

                ColumnInfo {
                    name,
                    data_type,
                    nullable: is_nullable.to_uppercase() == "YES",
                    default_value,
                    is_primary_key,
                    is_auto_increment,
                    max_length,
                    precision,
                    scale,
                    description,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(columns)
    }

    async fn get_table_info(
        &self,
        database: Option<&str>,
        _schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        let mut conn = self.get_conn().await?;

        // If a specific database is requested, use it
        let db_name = database
            .or(self.config.database.as_deref())
            .ok_or_else(|| DbError::Configuration("No database specified".to_string()))?;

        // Get basic table information
        let query = format!(
            "SELECT 
                TABLE_NAME,
                TABLE_TYPE,
                TABLE_ROWS,
                DATA_LENGTH + INDEX_LENGTH as size_bytes,
                TABLE_COMMENT
            FROM INFORMATION_SCHEMA.TABLES
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?"
        );

        let row: Option<Row> = conn
            .exec_first(&query, (db_name, table))
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let row = row.ok_or_else(|| {
            DbError::TableNotFound(format!("Table {}.{} not found", db_name, table))
        })?;

        let name: String = row.get(0).unwrap();
        let table_type_raw: String = row.get(1).unwrap();
        let table_type = if table_type_raw.to_uppercase() == "BASE TABLE" {
            "TABLE".to_string()
        } else {
            table_type_raw.to_uppercase()
        };
        let row_count: Option<u64> = row.get(2);
        let size_bytes: Option<u64> = row.get(3);
        let description: Option<String> = row.get(4);

        Ok(TableInfo {
            schema: Some(db_name.to_string()),
            name,
            table_type,
            row_count,
            size_bytes,
            description,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseType;

    #[test]
    fn test_mysql_adapter_creation() {
        let config = ConnectionConfig::new(DatabaseType::MySQL, "localhost", 3306, "root")
            .with_database("testdb")
            .with_password("password");

        let adapter = MySQLAdapter::new(config);
        assert!(adapter.pool.is_none());
        assert_eq!(adapter.config.host, "localhost");
    }

    #[test]
    fn test_connection_opts_building() {
        let config = ConnectionConfig::new(DatabaseType::MySQL, "localhost", 3306, "root")
            .with_database("testdb")
            .with_password("password")
            .with_ssl_mode(SslMode::Require)
            .with_option("connect_timeout", "10");

        let adapter = MySQLAdapter::new(config);
        let _opts = adapter.build_connection_opts().expect("Failed to build opts");

        // Verify the options were set correctly - just check that opts built successfully
    }

    #[test]
    fn test_ssl_mode_configuration() {
        let test_cases = vec![
            SslMode::Disable,
            SslMode::Prefer,
            SslMode::Require,
            SslMode::VerifyCA,
            SslMode::VerifyFull,
        ];

        for ssl_mode in test_cases {
            let config = ConnectionConfig::new(DatabaseType::MySQL, "localhost", 3306, "root")
                .with_ssl_mode(ssl_mode);

            let adapter = MySQLAdapter::new(config);
            // Should not panic when building opts with different SSL modes
            let _ = adapter.build_connection_opts();
        }
    }

    #[test]
    fn test_connection_opts_with_multiple_options() {
        let config = ConnectionConfig::new(DatabaseType::MySQL, "db.example.com", 3307, "admin")
            .with_database("production")
            .with_password("secure_pass")
            .with_ssl_mode(SslMode::VerifyFull)
            .with_option("connect_timeout", "20");

        let adapter = MySQLAdapter::new(config);
        let _opts = adapter.build_connection_opts().expect("Failed to build opts");

        // Verify the options were set correctly - just check that opts built successfully
    }

    #[test]
    fn test_connection_opts_without_database() {
        let config = ConnectionConfig::new(DatabaseType::MySQL, "localhost", 3306, "root")
            .with_password("password");

        let adapter = MySQLAdapter::new(config);
        let _opts = adapter.build_connection_opts().expect("Failed to build opts");

        // Verify opts build successfully even without database
    }

    #[test]
    fn test_connection_opts_without_password() {
        let config =
            ConnectionConfig::new(DatabaseType::MySQL, "localhost", 3306, "root").with_database("testdb");

        let adapter = MySQLAdapter::new(config);
        let _opts = adapter.build_connection_opts().expect("Failed to build opts");

        // Verify opts build successfully even without password
    }

    #[test]
    fn test_get_config() {
        let config = ConnectionConfig::new(DatabaseType::MySQL, "localhost", 3306, "root")
            .with_database("testdb");

        let adapter = MySQLAdapter::new(config.clone());
        let retrieved_config = adapter.get_config();

        assert_eq!(retrieved_config.host, config.host);
        assert_eq!(retrieved_config.port, config.port);
        assert_eq!(retrieved_config.username, config.username);
        assert_eq!(retrieved_config.database, config.database);
    }

    #[test]
    fn test_pool_initially_none() {
        let config = ConnectionConfig::new(DatabaseType::MySQL, "localhost", 3306, "root");
        let adapter = MySQLAdapter::new(config);

        assert!(adapter.get_pool().is_none());
    }
}
