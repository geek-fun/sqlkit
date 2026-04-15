//! PostgreSQL database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for PostgreSQL databases using tokio-postgres with connection pooling support.

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
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use deadpool_postgres::{
    Config as DeadpoolConfig, Pool, PoolConfig as DeadpoolPoolConfig, Runtime,
};
use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_postgres::{
    types::{FromSql, Kind, Type},
    Client, NoTls, Row,
};

/// A wrapper type that accepts any PostgreSQL type and reads raw bytes as UTF-8.
/// Used for custom types like ENUMs where the standard String deserialization
/// rejects non-text OIDs.
struct RawString(String);

impl<'a> FromSql<'a> for RawString {
    fn from_sql(
        _ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let s = std::str::from_utf8(raw)?;
        Ok(RawString(s.to_owned()))
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }
}

/// Convert PostgreSQL error to DbError with detailed information
fn postgres_error_to_db_error(error: tokio_postgres::Error) -> DbError {
    // For database errors, we want to preserve structured information
    // by creating a formatted string that our api_response parser can understand
    if let Some(db_error) = error.as_db_error() {
        let mut error_details = Vec::new();

        // Main error message
        error_details.push(db_error.message().to_string());

        // Add structured fields
        error_details.push(format!("[SQLSTATE: {}]", db_error.code().code()));

        if let Some(detail) = db_error.detail() {
            error_details.push(format!("[Detail] {}", detail));
        }

        if let Some(hint) = db_error.hint() {
            error_details.push(format!("[Hint] {}", hint));
        }

        if let Some(schema) = db_error.schema() {
            error_details.push(format!("[Schema: {}]", schema));
        }

        if let Some(table) = db_error.table() {
            error_details.push(format!("[Table: {}]", table));
        }

        if let Some(column) = db_error.column() {
            error_details.push(format!("[Column: {}]", column));
        }

        if let Some(constraint) = db_error.constraint() {
            error_details.push(format!("[Constraint: {}]", constraint));
        }

        if let Some(position) = db_error.position() {
            error_details.push(format!("[Position: {:?}]", position));
        }

        DbError::QueryExecution(error_details.join("\n"))
    } else {
        // For non-database errors, just use the error message
        DbError::QueryExecution(error.to_string())
    }
}

/// Convert deadpool pool error to DbError with detailed PostgreSQL error extraction.
///
/// Deadpool wraps the underlying tokio_postgres error with generic messages like
/// "Error occurred while creating a new object: db error" which hide authentication
/// failures. This function extracts the actual PostgreSQL error message.
fn deadpool_pool_error_to_db_error(error: deadpool_postgres::PoolError) -> DbError {
    use deadpool_postgres::PoolError;

    match error {
        PoolError::Timeout(timeout_type) => {
            DbError::Timeout(format!("Connection timeout: {:?}", timeout_type))
        }
        PoolError::Backend(pg_err) => {
            if let Some(db_error) = pg_err.as_db_error() {
                let code = db_error.code().code();
                let message = db_error.message();

                // SQLSTATE 28xxx = authentication errors, 3Dxxx = invalid catalog name
                if code.starts_with("28") || code.starts_with("3D") {
                    let mut details = vec![message.to_string()];
                    if let Some(detail) = db_error.detail() {
                        details.push(detail.to_string());
                    }
                    DbError::Authentication(details.join(" - "))
                } else {
                    DbError::Connection(format!("{} [SQLSTATE: {}]", message, code))
                }
            } else {
                DbError::Connection(pg_err.to_string())
            }
        }
        PoolError::Closed => DbError::Connection("Connection pool is closed".to_string()),
        PoolError::NoRuntimeSpecified => {
            DbError::Configuration("No runtime specified for pool".to_string())
        }
        PoolError::PostCreateHook(hook_err) => {
            DbError::Connection(format!("Post-create hook error: {:?}", hook_err))
        }
    }
}

/// PostgreSQL connection pool wrapper.
pub struct PostgresPool {
    pool: Pool,
}

#[async_trait]
impl ConnectionPool for PostgresPool {
    type Connection = Client;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        // NOTE: This method is not used in the current implementation because deadpool-postgres
        // provides its own connection management through the pool.get() method.
        // The PostgresAdapter methods directly call pool.get() instead of using this trait method.
        // This is a known limitation of the current ConnectionPool trait design when used with
        // deadpool, which wraps connections in its own guard type that cannot be easily converted
        // to Arc<Client>. Future versions could either:
        // 1. Redesign the ConnectionPool trait to be more generic
        // 2. Use a different pooling strategy
        // 3. Keep the current approach where this method is intentionally not used
        Err(DbError::UnsupportedOperation(
            "Direct connection access not supported with deadpool - use pool.get() directly"
                .to_string(),
        ))
    }

    async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
        // Connection is automatically returned when dropped
        Ok(())
    }

    fn active_connections(&self) -> usize {
        self.pool.status().size
    }

    fn idle_connections(&self) -> usize {
        self.pool.status().available
    }

    fn max_connections(&self) -> usize {
        self.pool.status().max_size
    }

    async fn close(&self) -> DbResult<()> {
        // Pool connections will be closed when the pool is dropped
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| DbError::PoolError(format!("Health check failed: {}", e)))?;

        client
            .query_one("SELECT 1", &[])
            .await
            .map_err(|e| DbError::PoolError(format!("Health check query failed: {}", e)))?;

        Ok(())
    }
}

/// PostgreSQL database adapter.
pub struct PostgresAdapter {
    pub(crate) config: ConnectionConfig,
    pool: Option<Arc<PostgresPool>>,
}

impl PostgresAdapter {
    /// Create a new PostgreSQL adapter with the given configuration.
    pub fn new(config: ConnectionConfig) -> Self {
        Self { config, pool: None }
    }

    /// Build the PostgreSQL connection string.
    fn build_connection_string(&self) -> String {
        let mut parts = Vec::new();

        parts.push(format!("host={}", self.config.host));
        parts.push(format!("port={}", self.config.port));
        parts.push(format!("user={}", self.config.username));

        if let Some(ref password) = self.config.password {
            parts.push(format!("password={}", password));
        }

        if let Some(ref database) = self.config.database {
            parts.push(format!("dbname={}", database));
        }

        // Add SSL mode
        let ssl_mode = match self.config.ssl_mode {
            SslMode::Disable => "disable",
            SslMode::Prefer => "prefer",
            SslMode::Require => "require",
            SslMode::VerifyCA => "verify-ca",
            SslMode::VerifyFull => "verify-full",
        };
        parts.push(format!("sslmode={}", ssl_mode));

        // Add additional options
        for (key, value) in &self.config.options {
            parts.push(format!("{}={}", key, value));
        }

        parts.join(" ")
    }

    fn build_tls_connector(&self, skip_verification: bool) -> DbResult<TlsConnector> {
        let mut builder = TlsConnector::builder();

        if skip_verification {
            builder.danger_accept_invalid_certs(true);
            builder.danger_accept_invalid_hostnames(true);
        }

        if let Some(ref ca_cert_path) = self.config.ssl_ca_cert {
            let cert_data = fs::read(ca_cert_path).map_err(|e| {
                DbError::Connection(format!("Failed to read CA certificate: {}", e))
            })?;
            let cert = Certificate::from_pem(&cert_data).map_err(|e| {
                DbError::Connection(format!("Failed to parse CA certificate: {}", e))
            })?;
            builder.add_root_certificate(cert);
        }

        builder
            .build()
            .map_err(|e| DbError::Connection(format!("Failed to build TLS: {}", e)))
    }

    /// Convert a tokio_postgres Row to QueryRow.
    fn row_to_query_row(row: &Row) -> DbResult<QueryRow> {
        let mut query_row = HashMap::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let name = column.name().to_string();
            // Use a resilient conversion that falls back to string representation
            let value = Self::convert_value_safe(row, idx, column.type_());
            query_row.insert(name, value);
        }

        Ok(query_row)
    }

    /// Safely convert a PostgreSQL value to QueryValue with fallback to string.
    fn convert_value_safe(row: &Row, idx: usize, col_type: &Type) -> QueryValue {
        match Self::convert_value(row, idx, col_type) {
            Ok(value) => value,
            Err(_) => {
                match row.try_get::<_, Option<String>>(idx) {
                    Ok(Some(s)) => QueryValue::String(s),
                    Ok(None) => QueryValue::Null,
                    Err(_) => {
                        match row.try_get::<_, Option<RawString>>(idx) {
                            Ok(Some(s)) => QueryValue::String(s.0),
                            Ok(None) => QueryValue::Null,
                            Err(_) => {
                                QueryValue::String(format!("<{}>", col_type.name()))
                            }
                        }
                    }
                }
            }
        }
    }

    /// Convert a PostgreSQL value to QueryValue.
    fn convert_value(row: &Row, idx: usize, col_type: &Type) -> DbResult<QueryValue> {
        match *col_type {
            Type::BOOL => {
                let val: Option<bool> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::Bool(v)),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::INT2 => {
                let val: Option<i16> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::Int(v as i64)),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::INT4 => {
                let val: Option<i32> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::Int(v as i64)),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::INT8 => {
                let val: Option<i64> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::Int(v)),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::FLOAT4 => {
                let val: Option<f32> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::Float(v as f64)),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::FLOAT8 => {
                let val: Option<f64> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::Float(v)),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::NUMERIC => {
                use rust_decimal::Decimal;
                let val: Option<Decimal> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::String(v.to_string())),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::VARCHAR | Type::TEXT | Type::BPCHAR | Type::NAME => {
                let val: Option<String> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::String(v)),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::BYTEA => {
                let val: Option<Vec<u8>> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::Bytes(v)),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::UUID => {
                use uuid::Uuid;
                let val: Option<Uuid> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::String(v.to_string())),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::JSON | Type::JSONB => {
                let val: Option<serde_json::Value> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::String(v.to_string())),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::TIMESTAMP => {
                let val: Option<NaiveDateTime> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::DateTime(
                        v.format("%Y-%m-%d %H:%M:%S%.f").to_string(),
                    )),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::TIMESTAMPTZ => {
                let val: Option<DateTime<FixedOffset>> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::DateTime(
                        v.format("%Y-%m-%d %H:%M:%S %z").to_string(),
                    )),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::DATE => {
                let val: Option<NaiveDate> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::DateTime(v.to_string())),
                    None => Ok(QueryValue::Null),
                }
            }
            Type::TIME | Type::TIMETZ => {
                let val: Option<NaiveTime> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::DateTime(v.to_string())),
                    None => Ok(QueryValue::Null),
                }
            }
            // Handle array types (kind = Array) — decode element-by-element
            _ if matches!(col_type.kind(), Kind::Array(_)) => {
                let inner = if let Kind::Array(t) = col_type.kind() {
                    t
                } else {
                    unreachable!()
                };
                let formatted = match *inner {
                    // text-like arrays
                    Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::CHAR => {
                        let v: Option<Vec<Option<String>>> = row
                            .try_get(idx)
                            .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                        match v {
                            None => return Ok(QueryValue::Null),
                            Some(items) => items
                                .iter()
                                .map(|x| x.as_deref().unwrap_or("NULL").to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                        }
                    }
                    // integer arrays
                    Type::INT2 | Type::INT4 => {
                        let v: Option<Vec<Option<i32>>> = row
                            .try_get(idx)
                            .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                        match v {
                            None => return Ok(QueryValue::Null),
                            Some(items) => items
                                .iter()
                                .map(|x| {
                                    x.map(|n| n.to_string())
                                        .unwrap_or_else(|| "NULL".to_string())
                                })
                                .collect::<Vec<_>>()
                                .join(", "),
                        }
                    }
                    Type::INT8 => {
                        let v: Option<Vec<Option<i64>>> = row
                            .try_get(idx)
                            .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                        match v {
                            None => return Ok(QueryValue::Null),
                            Some(items) => items
                                .iter()
                                .map(|x| {
                                    x.map(|n| n.to_string())
                                        .unwrap_or_else(|| "NULL".to_string())
                                })
                                .collect::<Vec<_>>()
                                .join(", "),
                        }
                    }
                    // float arrays
                    Type::FLOAT4 | Type::FLOAT8 => {
                        let v: Option<Vec<Option<f64>>> = row
                            .try_get(idx)
                            .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                        match v {
                            None => return Ok(QueryValue::Null),
                            Some(items) => items
                                .iter()
                                .map(|x| {
                                    x.map(|n| n.to_string())
                                        .unwrap_or_else(|| "NULL".to_string())
                                })
                                .collect::<Vec<_>>()
                                .join(", "),
                        }
                    }
                    // bool arrays
                    Type::BOOL => {
                        let v: Option<Vec<Option<bool>>> = row
                            .try_get(idx)
                            .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                        match v {
                            None => return Ok(QueryValue::Null),
                            Some(items) => items
                                .iter()
                                .map(|x| {
                                    x.map(|b| b.to_string())
                                        .unwrap_or_else(|| "NULL".to_string())
                                })
                                .collect::<Vec<_>>()
                                .join(", "),
                        }
                    }
                    // fallback: try text array
                    _ => {
                        let v: Option<Vec<Option<String>>> = row.try_get(idx).unwrap_or(None);
                        match v {
                            None => return Ok(QueryValue::Null),
                            Some(items) => items
                                .iter()
                                .map(|x| x.as_deref().unwrap_or("NULL").to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                        }
                    }
                };
                Ok(QueryValue::String(format!("[{}]", formatted)))
            }
            _ if matches!(col_type.kind(), Kind::Enum(_)) => {
                match row.try_get::<_, Option<RawString>>(idx) {
                    Ok(Some(v)) => Ok(QueryValue::String(v.0)),
                    Ok(None) => Ok(QueryValue::Null),
                    Err(e) => Err(DbError::TypeConversion(e.to_string())),
                }
            }
            _ => {
                let val: Option<String> = row
                    .try_get(idx)
                    .map_err(|e| DbError::TypeConversion(e.to_string()))?;
                match val {
                    Some(v) => Ok(QueryValue::String(v)),
                    None => Ok(QueryValue::Null),
                }
            }
        }
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    type Pool = PostgresPool;

    async fn connect(&mut self) -> DbResult<()> {
        let connection_string = self.build_connection_string();

        // Create deadpool configuration
        let mut pg_config = DeadpoolConfig::new();
        pg_config.url = Some(connection_string);
        pg_config.pool = Some(DeadpoolPoolConfig::new(
            self.config.pool_config.max_connections as usize,
        ));

        // Determine TLS configuration
        let pool = match self.config.ssl_mode {
            SslMode::Disable => pg_config
                .create_pool(Some(Runtime::Tokio1), NoTls)
                .map_err(|e| DbError::Connection(format!("Failed to create pool: {}", e)))?,
            SslMode::Prefer | SslMode::Require => {
                let tls_connector = self.build_tls_connector(true)?;
                let tls = MakeTlsConnector::new(tls_connector);
                pg_config
                    .create_pool(Some(Runtime::Tokio1), tls)
                    .map_err(|e| DbError::Connection(format!("Failed to create pool: {}", e)))?
            }
            SslMode::VerifyCA | SslMode::VerifyFull => {
                let tls_connector = self.build_tls_connector(false)?;
                let tls = MakeTlsConnector::new(tls_connector);
                pg_config
                    .create_pool(Some(Runtime::Tokio1), tls)
                    .map_err(|e| DbError::Connection(format!("Failed to create pool: {}", e)))?
            }
        };

        let _client = pool.get().await.map_err(deadpool_pool_error_to_db_error)?;

        self.pool = Some(Arc::new(PostgresPool { pool }));

        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.pool = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let client = pool
            .pool
            .get()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to get connection: {}", e)))?;

        let version_row = client
            .query_one("SELECT version()", &[])
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;
        let server_version: String = version_row.get(0);

        let db_row = client
            .query_one("SELECT current_database(), current_user", &[])
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;
        let current_database: String = db_row.get(0);
        let current_user: String = db_row.get(1);

        Ok(ConnectionStatus {
            is_connected: true,
            server_version: Some(server_version),
            current_database: Some(current_database),
            current_user: Some(current_user),
            metadata: HashMap::new(),
        })
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let client = pool
            .pool
            .get()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to get connection: {}", e)))?;

        let start = Instant::now();

        // Handle queries with timeout if configured
        let timeout = self
            .config
            .options
            .get("statement_timeout")
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_millis);

        // Determine if this is a query that returns rows or a statement
        let query_trimmed = query.trim().to_uppercase();
        let is_select = query_trimmed.starts_with("SELECT")
            || query_trimmed.starts_with("WITH")
            || query_trimmed.starts_with("SHOW")
            || query_trimmed.starts_with("EXPLAIN");

        let execution_time;

        if is_select {
            let result = if let Some(timeout_duration) = timeout {
                tokio::time::timeout(timeout_duration, client.query(query, &[]))
                    .await
                    .map_err(|_| {
                        DbError::Timeout(format!("Query timed out after {:?}", timeout_duration))
                    })?
                    .map_err(postgres_error_to_db_error)?
            } else {
                client
                    .query(query, &[])
                    .await
                    .map_err(postgres_error_to_db_error)?
            };

            execution_time = start.elapsed().as_millis() as u64;

            if result.is_empty() {
                Ok(QueryResult::new(Vec::new()).with_execution_time(execution_time))
            } else {
                let columns: Vec<String> = result[0]
                    .columns()
                    .iter()
                    .map(|col| col.name().to_string())
                    .collect();

                let mut query_result = QueryResult::new(columns);
                for row in &result {
                    let query_row = Self::row_to_query_row(row)?;
                    query_result.add_row(query_row);
                }

                Ok(query_result.with_execution_time(execution_time))
            }
        } else {
            // For INSERT, UPDATE, DELETE, etc.
            let affected = if let Some(timeout_duration) = timeout {
                tokio::time::timeout(timeout_duration, client.execute(query, &[]))
                    .await
                    .map_err(|_| {
                        DbError::Timeout(format!("Query timed out after {:?}", timeout_duration))
                    })?
                    .map_err(postgres_error_to_db_error)?
            } else {
                client
                    .execute(query, &[])
                    .await
                    .map_err(postgres_error_to_db_error)?
            };

            execution_time = start.elapsed().as_millis() as u64;

            Ok(QueryResult::affected(affected).with_execution_time(execution_time))
        }
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        let query = r#"
            SELECT 
                datname as name,
                pg_catalog.shobj_description(oid, 'pg_database') as description,
                CASE WHEN datname IN ('postgres', 'template0', 'template1') THEN true ELSE false END as is_system
            FROM pg_catalog.pg_database
            WHERE datistemplate = false
            ORDER BY is_system, datname
        "#;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let client = pool
            .pool
            .get()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to get connection: {}", e)))?;

        let rows = client
            .query(query, &[])
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let databases = rows
            .iter()
            .map(|row| {
                let name: String = row.get(0);
                let description: Option<String> = row.get(1);
                let is_system: bool = row.get(2);

                DatabaseSchema {
                    name,
                    description,
                    is_system,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(databases)
    }

    async fn list_schemas(&self, database: Option<&str>) -> DbResult<Vec<String>> {
        // If a different database is requested, create a temporary connection to it.
        // PostgreSQL connections are per-database, so we cannot query another database's
        // schemas through the current connection.
        if let Some(db) = database {
            if Some(db) != self.config.database.as_deref() {
                let mut temp_config = self.config.clone();
                temp_config.database = Some(db.to_string());
                let mut temp_adapter = PostgresAdapter::new(temp_config);
                temp_adapter.connect().await?;
                return temp_adapter.list_schemas(None).await;
            }
        }

        let query = r#"
            SELECT schema_name
            FROM information_schema.schemata
            WHERE schema_name NOT IN ('pg_catalog', 'information_schema')
                AND schema_name NOT LIKE 'pg_toast%'
                AND schema_name NOT LIKE 'pg_temp%'
            ORDER BY schema_name
        "#;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let client = pool
            .pool
            .get()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to get connection: {}", e)))?;

        let rows = client
            .query(query, &[])
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let schemas = rows.iter().map(|row| row.get(0)).collect();

        Ok(schemas)
    }

    async fn list_tables(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        // If a different database is requested, create a temporary connection to it.
        // PostgreSQL connections are per-database, so we cannot query another database's
        // tables through the current connection.
        if let Some(db) = database {
            if Some(db) != self.config.database.as_deref() {
                let mut temp_config = self.config.clone();
                temp_config.database = Some(db.to_string());
                let mut temp_adapter = PostgresAdapter::new(temp_config);
                temp_adapter.connect().await?;
                return temp_adapter.list_tables(None, schema).await;
            }
        }

        let schema_filter = schema.unwrap_or("public");

        let query = r#"
            SELECT 
                schemaname as schema,
                tablename as name,
                'TABLE' as table_type
            FROM pg_catalog.pg_tables
            WHERE schemaname = $1
            UNION ALL
            SELECT 
                schemaname as schema,
                viewname as name,
                'VIEW' as table_type
            FROM pg_catalog.pg_views
            WHERE schemaname = $1
            ORDER BY name
        "#;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let client = pool
            .pool
            .get()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to get connection: {}", e)))?;

        let rows = client
            .query(query, &[&schema_filter])
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let tables = rows
            .iter()
            .map(|row| {
                let schema: String = row.get(0);
                let name: String = row.get(1);
                let table_type: String = row.get(2);

                TableInfo {
                    schema: Some(schema),
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
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        if database.is_some() && database != self.config.database.as_deref() {
            return Err(DbError::UnsupportedOperation(
                "Cannot list columns from a different database without reconnecting".to_string(),
            ));
        }

        let schema_filter = schema.unwrap_or("public");

        let query = r#"
            SELECT 
                c.column_name,
                c.data_type,
                c.udt_name,
                c.is_nullable,
                c.column_default,
                c.character_maximum_length,
                c.numeric_precision,
                c.numeric_scale,
                pg_catalog.col_description(
                    (c.table_schema || '.' || c.table_name)::regclass::oid,
                    c.ordinal_position
                ) as description,
                CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END as is_primary_key,
                CASE WHEN c.data_type = 'USER-DEFINED' AND EXISTS (
                    SELECT 1 FROM pg_catalog.pg_type t
                    JOIN pg_catalog.pg_namespace n ON t.typnamespace = n.oid
                    WHERE t.typname = c.udt_name AND n.nspname = c.udt_schema AND t.typtype = 'e'
                ) THEN true ELSE false END as is_enum
            FROM information_schema.columns c
            LEFT JOIN (
                SELECT ku.table_schema, ku.table_name, ku.column_name
                FROM information_schema.table_constraints tc
                JOIN information_schema.key_column_usage ku
                    ON tc.constraint_name = ku.constraint_name
                    AND tc.table_schema = ku.table_schema
                WHERE tc.constraint_type = 'PRIMARY KEY'
            ) pk ON c.table_schema = pk.table_schema 
                AND c.table_name = pk.table_name 
                AND c.column_name = pk.column_name
            WHERE c.table_schema = $1 AND c.table_name = $2
            ORDER BY c.ordinal_position
        "#;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let client = pool
            .pool
            .get()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to get connection: {}", e)))?;

        let rows = client
            .query(query, &[&schema_filter, &table])
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let columns = rows
            .iter()
            .map(|row| {
                let name: String = row.get(0);
                let data_type: String = row.get(1);
                let udt_name: String = row.get(2);
                let is_nullable: String = row.get(3);
                let default_value: Option<String> = row.get(4);
                let max_length: Option<i32> = row.get(5);
                let precision: Option<i32> = row.get(6);
                let scale: Option<i32> = row.get(7);
                let description: Option<String> = row.get(8);
                let is_primary_key: bool = row.get(9);
                let is_enum: bool = row.get(10);

                let is_auto_increment = default_value
                    .as_ref()
                    .map(|d| d.contains("nextval"))
                    .unwrap_or(false);

                let actual_type = if data_type == "USER-DEFINED" {
                    udt_name
                } else {
                    data_type
                };

                let mut metadata = HashMap::new();
                if is_enum {
                    metadata.insert("is_enum".to_string(), "true".to_string());
                }

                ColumnInfo {
                    name,
                    data_type: actual_type,
                    nullable: is_nullable == "YES",
                    default_value,
                    is_primary_key,
                    is_auto_increment,
                    max_length: max_length.map(|v| v as u32),
                    precision: precision.map(|v| v as u32),
                    scale: scale.map(|v| v as u32),
                    description,
                    metadata,
                }
            })
            .collect();

        Ok(columns)
    }

    async fn get_table_info(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        if database.is_some() && database != self.config.database.as_deref() {
            return Err(DbError::UnsupportedOperation(
                "Cannot get table info from a different database without reconnecting".to_string(),
            ));
        }

        let schema_filter = schema.unwrap_or("public");

        let query = r#"
            SELECT 
                schemaname,
                tablename,
                'TABLE' as table_type,
                pg_catalog.obj_description((schemaname || '.' || tablename)::regclass::oid, 'pg_class') as description
            FROM pg_catalog.pg_tables
            WHERE schemaname = $1 AND tablename = $2
            UNION ALL
            SELECT 
                schemaname,
                viewname,
                'VIEW' as table_type,
                pg_catalog.obj_description((schemaname || '.' || viewname)::regclass::oid, 'pg_class') as description
            FROM pg_catalog.pg_views
            WHERE schemaname = $1 AND viewname = $2
        "#;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))?;

        let client = pool
            .pool
            .get()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to get connection: {}", e)))?;

        let rows = client
            .query(query, &[&schema_filter, &table])
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        if rows.is_empty() {
            return Err(DbError::TableNotFound(format!(
                "Table {}.{} not found",
                schema_filter, table
            )));
        }

        let row = &rows[0];
        let schema: String = row.get(0);
        let name: String = row.get(1);
        let table_type: String = row.get(2);
        let description: Option<String> = row.get(3);

        // Get row count and size for tables (not views)
        let (row_count, size_bytes) = if table_type == "TABLE" {
            // Use a safer approach by constructing the qualified table name from validated schema and table
            let stats_query = r#"
                SELECT 
                    c.reltuples::bigint as row_count,
                    pg_total_relation_size(c.oid) as size_bytes
                FROM pg_class c
                JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE n.nspname = $1 AND c.relname = $2
            "#;

            let stats_rows = client
                .query(stats_query, &[&schema_filter, &table])
                .await
                .map_err(|e| DbError::QueryExecution(e.to_string()))?;

            if !stats_rows.is_empty() {
                let row_count: i64 = stats_rows[0].get(0);
                let size_bytes: i64 = stats_rows[0].get(1);
                (Some(row_count as u64), Some(size_bytes as u64))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        Ok(TableInfo {
            schema: Some(schema),
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
    fn test_postgres_adapter_creation() {
        let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "postgres")
            .with_database("testdb")
            .with_password("password");

        let adapter = PostgresAdapter::new(config);
        assert!(adapter.pool.is_none());
        assert_eq!(adapter.config.host, "localhost");
    }

    #[test]
    fn test_connection_string_building() {
        let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "postgres")
            .with_database("testdb")
            .with_password("password")
            .with_ssl_mode(SslMode::Require)
            .with_option("application_name", "sqlkit");

        let adapter = PostgresAdapter::new(config);
        let conn_str = adapter.build_connection_string();

        assert!(conn_str.contains("host=localhost"));
        assert!(conn_str.contains("port=5432"));
        assert!(conn_str.contains("user=postgres"));
        assert!(conn_str.contains("password=password"));
        assert!(conn_str.contains("dbname=testdb"));
        assert!(conn_str.contains("sslmode=require"));
        assert!(conn_str.contains("application_name=sqlkit"));
    }

    #[test]
    fn test_ssl_mode_mapping() {
        let test_cases = vec![
            (SslMode::Disable, "sslmode=disable"),
            (SslMode::Prefer, "sslmode=prefer"),
            (SslMode::Require, "sslmode=require"),
            (SslMode::VerifyCA, "sslmode=verify-ca"),
            (SslMode::VerifyFull, "sslmode=verify-full"),
        ];

        for (ssl_mode, expected) in test_cases {
            let config =
                ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "postgres")
                    .with_ssl_mode(ssl_mode);

            let adapter = PostgresAdapter::new(config);
            let conn_str = adapter.build_connection_string();

            assert!(
                conn_str.contains(expected),
                "Expected {} in connection string",
                expected
            );
        }
    }

    #[test]
    fn test_connection_string_with_multiple_options() {
        let config =
            ConnectionConfig::new(DatabaseType::PostgreSQL, "db.example.com", 5433, "admin")
                .with_database("production")
                .with_password("secure_pass")
                .with_ssl_mode(SslMode::VerifyFull)
                .with_option("application_name", "sqlkit")
                .with_option("connect_timeout", "10");

        let adapter = PostgresAdapter::new(config);
        let conn_str = adapter.build_connection_string();

        assert!(conn_str.contains("host=db.example.com"));
        assert!(conn_str.contains("port=5433"));
        assert!(conn_str.contains("user=admin"));
        assert!(conn_str.contains("password=secure_pass"));
        assert!(conn_str.contains("dbname=production"));
        assert!(conn_str.contains("sslmode=verify-full"));
        assert!(conn_str.contains("application_name=sqlkit"));
        assert!(conn_str.contains("connect_timeout=10"));
    }

    #[test]
    fn test_connection_string_without_database() {
        let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "postgres")
            .with_password("password");

        let adapter = PostgresAdapter::new(config);
        let conn_str = adapter.build_connection_string();

        assert!(conn_str.contains("host=localhost"));
        assert!(conn_str.contains("port=5432"));
        assert!(conn_str.contains("user=postgres"));
        assert!(conn_str.contains("password=password"));
        assert!(!conn_str.contains("dbname="));
    }

    #[test]
    fn test_connection_string_without_password() {
        let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "postgres")
            .with_database("testdb");

        let adapter = PostgresAdapter::new(config);
        let conn_str = adapter.build_connection_string();

        assert!(conn_str.contains("host=localhost"));
        assert!(conn_str.contains("port=5432"));
        assert!(conn_str.contains("user=postgres"));
        assert!(conn_str.contains("dbname=testdb"));
        assert!(!conn_str.contains("password="));
    }

    #[test]
    fn test_get_config() {
        let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "postgres")
            .with_database("testdb");

        let adapter = PostgresAdapter::new(config.clone());
        let retrieved_config = adapter.get_config();

        assert_eq!(retrieved_config.host, config.host);
        assert_eq!(retrieved_config.port, config.port);
        assert_eq!(retrieved_config.username, config.username);
        assert_eq!(retrieved_config.database, config.database);
    }

    #[test]
    fn test_pool_initially_none() {
        let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "postgres");
        let adapter = PostgresAdapter::new(config);

        assert!(adapter.get_pool().is_none());
    }
}
