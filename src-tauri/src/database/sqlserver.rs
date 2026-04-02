//! SQL Server database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for Microsoft SQL Server databases using tiberius with connection pooling support.

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
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tiberius::{AuthMethod, Client, Config, EncryptionLevel, Row};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use uuid;

/// SQL Server connection wrapper.
pub struct SqlServerConnection {
    client: Client<Compat<TcpStream>>,
}

/// SQL Server connection pool wrapper.
///
/// Note: This is a simple implementation that maintains a vector of connections.
/// For production use, consider using a more sophisticated pooling strategy.
pub struct SqlServerPool {
    config: ConnectionConfig,
    connections: Arc<tokio::sync::Mutex<Vec<SqlServerConnection>>>,
    max_size: usize,
}

#[async_trait]
impl ConnectionPool for SqlServerPool {
    type Connection = Client<Compat<TcpStream>>;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        // Note: This is a simplified implementation
        // A production pool would implement proper connection lifecycle management
        Err(DbError::UnsupportedOperation(
            "Direct connection access not supported - use pool methods directly".to_string(),
        ))
    }

    async fn return_connection(&self, _connection: Arc<Self::Connection>) -> DbResult<()> {
        // Connection is automatically returned when dropped
        Ok(())
    }

    fn active_connections(&self) -> usize {
        // Note: This requires async context to lock the mutex
        // Returning 0 as placeholder
        0
    }

    fn idle_connections(&self) -> usize {
        // Note: This requires async context to lock the mutex
        // Returning 0 as placeholder
        0
    }

    fn max_connections(&self) -> usize {
        self.max_size
    }

    async fn close(&self) -> DbResult<()> {
        let mut connections = self.connections.lock().await;
        connections.clear();
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        let client = self.get_client().await?;
        let mut client = client.lock().await;

        client
            .simple_query("SELECT 1")
            .await
            .map_err(|e| DbError::PoolError(format!("Health check failed: {}", e)))?
            .into_results()
            .await
            .map_err(|e| DbError::PoolError(format!("Health check query failed: {}", e)))?;

        Ok(())
    }
}

impl SqlServerPool {
    /// Create a new SQL Server pool.
    pub fn new(config: ConnectionConfig, max_size: usize) -> Self {
        Self {
            config,
            connections: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            max_size,
        }
    }

    /// Get a client from the pool or create a new one.
    async fn get_client(&self) -> DbResult<Arc<tokio::sync::Mutex<Client<Compat<TcpStream>>>>> {
        let mut connections = self.connections.lock().await;

        if let Some(conn) = connections.pop() {
            Ok(Arc::new(tokio::sync::Mutex::new(conn.client)))
        } else {
            // Create a new connection
            let client = Self::create_connection(&self.config).await?;
            Ok(Arc::new(tokio::sync::Mutex::new(client)))
        }
    }

    /// Create a new connection to SQL Server.
    async fn create_connection(config: &ConnectionConfig) -> DbResult<Client<Compat<TcpStream>>> {
        let mut tiberius_config = Config::new();

        tiberius_config.host(&config.host);
        tiberius_config.port(config.port);

        if let Some(ref database) = config.database {
            tiberius_config.database(database);
        }

        // Configure authentication
        // Check if Windows Authentication is requested via options
        let use_windows_auth = config
            .options
            .get("use_windows_auth")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        if use_windows_auth {
            // Windows Authentication (Integrated Security)
            // Note: In tiberius 0.12, Windows auth requires domain, username, and password
            // Windows Authentication
            // Note: tiberius 0.12 doesn't have native SSPI/Integrated auth
            // Fall back to SQL Server authentication with Windows credentials
            tiberius_config.authentication(AuthMethod::sql_server(
                &config.username,
                config.password.as_deref().unwrap_or(""),
            ));
        } else {
            // SQL Server Authentication (username/password)
            tiberius_config.authentication(AuthMethod::sql_server(
                &config.username,
                config.password.as_deref().unwrap_or(""),
            ));
        }

        // Configure TLS/SSL
        match config.ssl_mode {
            SslMode::Disable => {
                tiberius_config.encryption(EncryptionLevel::NotSupported);
            }
            SslMode::Prefer => {
                tiberius_config.encryption(EncryptionLevel::Off);
            }
            SslMode::Require => {
                tiberius_config.encryption(EncryptionLevel::Required);
                tiberius_config.trust_cert();
            }
            SslMode::VerifyCA | SslMode::VerifyFull => {
                tiberius_config.encryption(EncryptionLevel::Required);
            }
        }

        // Trust server certificate if specified
        if config.trust_server_certificate {
            tiberius_config.trust_cert();
        }

        // Establish TCP connection with optional timeout
        let connect_timeout = config
            .options
            .get("connect_timeout")
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30)); // Default 30 second timeout

        let tcp = tokio::time::timeout(
            connect_timeout,
            TcpStream::connect(tiberius_config.get_addr()),
        )
        .await
        .map_err(|_| {
            DbError::Connection(format!("Connection timeout after {:?}", connect_timeout))
        })?
        .map_err(|e| DbError::Connection(format!("Failed to connect to SQL Server: {}", e)))?;

        tcp.set_nodelay(true)
            .map_err(|e| DbError::Connection(format!("Failed to set TCP_NODELAY: {}", e)))?;

        // Connect using tiberius
        let client = Client::connect(tiberius_config, tcp.compat_write())
            .await
            .map_err(|e| DbError::Connection(format!("Failed to authenticate: {}", e)))?;

        Ok(client)
    }
}

/// SQL Server database adapter.
pub struct SqlServerAdapter {
    pub(crate) config: ConnectionConfig,
    pool: Option<Arc<SqlServerPool>>,
    client: Option<Arc<tokio::sync::Mutex<Client<Compat<TcpStream>>>>>,
}

impl SqlServerAdapter {
    /// Create a new SQL Server adapter with the given configuration.
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            pool: None,
            client: None,
        }
    }

    /// Get the current client connection.
    async fn get_client(&self) -> DbResult<Arc<tokio::sync::Mutex<Client<Compat<TcpStream>>>>> {
        if let Some(ref pool) = self.pool {
            pool.get_client().await
        } else if let Some(ref client) = self.client {
            Ok(client.clone())
        } else {
            Err(DbError::Connection("Not connected".to_string()))
        }
    }

    /// Convert a tiberius Row to QueryRow.
    fn row_to_query_row(row: &Row) -> DbResult<QueryRow> {
        let mut query_row = HashMap::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let name = column.name().to_string();
            let value = Self::convert_value(row, idx)?;
            query_row.insert(name, value);
        }

        Ok(query_row)
    }

    /// Convert a SQL Server value to QueryValue.
    fn convert_value(row: &Row, idx: usize) -> DbResult<QueryValue> {
        // Try different types using try_get
        // In tiberius 0.12, ColumnData fields are Option-wrapped

        // Try boolean
        if let Ok(Some(v)) = row.try_get::<bool, _>(idx) {
            return Ok(QueryValue::Bool(v));
        }

        // Try integers
        if let Ok(Some(v)) = row.try_get::<i16, _>(idx) {
            return Ok(QueryValue::Int(v as i64));
        }
        if let Ok(Some(v)) = row.try_get::<i32, _>(idx) {
            return Ok(QueryValue::Int(v as i64));
        }
        if let Ok(Some(v)) = row.try_get::<i64, _>(idx) {
            return Ok(QueryValue::Int(v));
        }

        // Try floats
        if let Ok(Some(v)) = row.try_get::<f32, _>(idx) {
            return Ok(QueryValue::Float(v as f64));
        }
        if let Ok(Some(v)) = row.try_get::<f64, _>(idx) {
            return Ok(QueryValue::Float(v));
        }

        // Try string
        if let Ok(Some(v)) = row.try_get::<&str, _>(idx) {
            return Ok(QueryValue::String(v.to_string()));
        }

        // Try binary
        if let Ok(Some(v)) = row.try_get::<&[u8], _>(idx) {
            return Ok(QueryValue::Bytes(v.to_vec()));
        }

        // Try UUID
        if let Ok(Some(v)) = row.try_get::<uuid::Uuid, _>(idx) {
            return Ok(QueryValue::String(v.to_string()));
        }

        // Default to null if nothing matches or value is NULL
        Ok(QueryValue::Null)
    }
}

#[async_trait]
impl DatabaseAdapter for SqlServerAdapter {
    type Pool = SqlServerPool;

    async fn connect(&mut self) -> DbResult<()> {
        // Create connection pool
        let pool = SqlServerPool::new(
            self.config.clone(),
            self.config.pool_config.max_connections as usize,
        );

        // Test the connection by getting a client
        let client = pool.get_client().await?;

        // Verify connection works with a simple query
        {
            let mut client_guard = client.lock().await;
            client_guard
                .simple_query("SELECT 1")
                .await
                .map_err(|e| DbError::Connection(format!("Connection test failed: {}", e)))?
                .into_results()
                .await
                .map_err(|e| DbError::Connection(format!("Connection test failed: {}", e)))?;
        }

        self.client = Some(client);
        self.pool = Some(Arc::new(pool));

        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        if let Some(pool) = self.pool.take() {
            pool.close().await?;
        }
        self.client = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let client = self.get_client().await?;
        let mut client = client.lock().await;

        // Get server version
        let version_stream = client
            .simple_query("SELECT @@VERSION as version")
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let version_result = version_stream
            .into_first_result()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let server_version = if let Some(row) = version_result.first() {
            match Self::convert_value(row, 0)? {
                QueryValue::String(s) => Some(s),
                _ => None,
            }
        } else {
            None
        };

        // Get current database and user
        let db_stream = client
            .simple_query("SELECT DB_NAME() as db, SUSER_SNAME() as username")
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let db_result = db_stream
            .into_first_result()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let (current_database, current_user) = if let Some(row) = db_result.first() {
            let db = match Self::convert_value(row, 0)? {
                QueryValue::String(s) => Some(s),
                _ => None,
            };
            let username = match Self::convert_value(row, 1)? {
                QueryValue::String(s) => Some(s),
                _ => None,
            };
            (db, username)
        } else {
            (None, None)
        };

        Ok(ConnectionStatus {
            is_connected: true,
            server_version,
            current_database,
            current_user,
            metadata: HashMap::new(),
        })
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let client = self.get_client().await?;
        let mut client = client.lock().await;
        let start = Instant::now();

        // Check if we have a query timeout configured
        let timeout = self
            .config
            .options
            .get("statement_timeout")
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_millis);

        // Execute query with optional timeout
        let stream = if let Some(timeout_duration) = timeout {
            tokio::time::timeout(timeout_duration, client.simple_query(query))
                .await
                .map_err(|_| {
                    DbError::Timeout(format!("Query timed out after {:?}", timeout_duration))
                })?
                .map_err(|e| DbError::QueryExecution(e.to_string()))?
        } else {
            client
                .simple_query(query)
                .await
                .map_err(|e| DbError::QueryExecution(e.to_string()))?
        };

        // Collect results
        let results = stream
            .into_results()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let execution_time = start.elapsed().as_millis() as u64;

        // Process results
        if results.is_empty() {
            // No result set (e.g., INSERT, UPDATE, DELETE)
            // Get rows affected from the last operation
            let affected = results.len() as u64;
            Ok(QueryResult::affected(affected).with_execution_time(execution_time))
        } else {
            let result_set = &results[0];

            if result_set.is_empty() {
                Ok(QueryResult::new(Vec::new()).with_execution_time(execution_time))
            } else {
                let columns: Vec<String> = result_set[0]
                    .columns()
                    .iter()
                    .map(|col| col.name().to_string())
                    .collect();

                let mut query_result = QueryResult::new(columns);
                for row in result_set {
                    let query_row = Self::row_to_query_row(row)?;
                    query_result.add_row(query_row);
                }

                Ok(query_result.with_execution_time(execution_time))
            }
        }
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        let query = r#"
            SELECT 
                name,
                NULL as description
            FROM sys.databases
            WHERE name NOT IN ('master', 'tempdb', 'model', 'msdb')
            ORDER BY name
        "#;

        let client = self.get_client().await?;
        let mut client = client.lock().await;

        let stream = client
            .simple_query(query)
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let rows = stream
            .into_first_result()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let databases = rows
            .iter()
            .map(|row| {
                let name = match Self::convert_value(row, 0) {
                    Ok(QueryValue::String(s)) => s,
                    _ => String::new(),
                };
                let description = match Self::convert_value(row, 1) {
                    Ok(QueryValue::String(s)) => Some(s),
                    _ => None,
                };

                DatabaseSchema {
                    name,
                    description,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(databases)
    }

    async fn list_schemas(&self, database: Option<&str>) -> DbResult<Vec<String>> {
        // If a different database is requested, create a temporary connection to it.
        // SQL Server connections are per-database context, so we reconnect to query another DB.
        if let Some(db) = database {
            if Some(db) != self.config.database.as_deref() {
                let mut temp_config = self.config.clone();
                temp_config.database = Some(db.to_string());
                let mut temp_adapter = SqlServerAdapter::new(temp_config);
                temp_adapter.connect().await?;
                return temp_adapter.list_schemas(None).await;
            }
        }

        // SQL Server supports schemas within databases
        let query = r#"
            SELECT name
            FROM sys.schemas
            WHERE name NOT IN ('db_owner', 'db_accessadmin', 'db_securityadmin', 
                              'db_ddladmin', 'db_backupoperator', 'db_datareader', 
                              'db_datawriter', 'db_denydatareader', 'db_denydatawriter',
                              'sys', 'INFORMATION_SCHEMA')
            ORDER BY name
        "#;

        let client = self.get_client().await?;
        let mut client = client.lock().await;

        let stream = client
            .simple_query(query)
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let rows = stream
            .into_first_result()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let schemas = rows
            .iter()
            .map(|row| match Self::convert_value(row, 0) {
                Ok(QueryValue::String(s)) => s,
                _ => String::new(),
            })
            .collect();

        Ok(schemas)
    }

    async fn list_tables(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        // If a different database is requested, create a temporary connection to it.
        // SQL Server connections are per-database context, so we reconnect to query another DB.
        if let Some(db) = database {
            if Some(db) != self.config.database.as_deref() {
                let mut temp_config = self.config.clone();
                temp_config.database = Some(db.to_string());
                let mut temp_adapter = SqlServerAdapter::new(temp_config);
                temp_adapter.connect().await?;
                return temp_adapter.list_tables(None, schema).await;
            }
        }

        let schema_filter = schema.unwrap_or("dbo");

        let query = format!(
            r#"
            SELECT 
                s.name as schema_name,
                t.name as table_name,
                t.type_desc as table_type
            FROM sys.tables t
            INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
            WHERE s.name = '{}'
            UNION ALL
            SELECT 
                s.name as schema_name,
                v.name as table_name,
                'VIEW' as table_type
            FROM sys.views v
            INNER JOIN sys.schemas s ON v.schema_id = s.schema_id
            WHERE s.name = '{}'
            ORDER BY table_name
            "#,
            schema_filter, schema_filter
        );

        let client = self.get_client().await?;
        let mut client = client.lock().await;

        let stream = client
            .simple_query(&query)
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let rows = stream
            .into_first_result()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let tables = rows
            .iter()
            .map(|row| {
                let schema = match Self::convert_value(row, 0) {
                    Ok(QueryValue::String(s)) => Some(s),
                    _ => None,
                };
                let name = match Self::convert_value(row, 1) {
                    Ok(QueryValue::String(s)) => s,
                    _ => String::new(),
                };
                let table_type = match Self::convert_value(row, 2) {
                    Ok(QueryValue::String(s)) => s,
                    _ => String::new(),
                };

                TableInfo {
                    schema,
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

        let schema_filter = schema.unwrap_or("dbo");

        let query = format!(
            r#"
            SELECT 
                c.name as column_name,
                t.name as data_type,
                c.is_nullable,
                c.max_length,
                c.precision,
                c.scale,
                CASE WHEN pk.column_id IS NOT NULL THEN 1 ELSE 0 END as is_primary_key,
                c.is_identity as is_auto_increment,
                dc.definition as default_value
            FROM sys.columns c
            INNER JOIN sys.tables tbl ON c.object_id = tbl.object_id
            INNER JOIN sys.schemas s ON tbl.schema_id = s.schema_id
            INNER JOIN sys.types t ON c.user_type_id = t.user_type_id
            LEFT JOIN sys.index_columns ic ON ic.object_id = c.object_id AND ic.column_id = c.column_id
            LEFT JOIN sys.indexes i ON ic.object_id = i.object_id AND ic.index_id = i.index_id AND i.is_primary_key = 1
            LEFT JOIN (
                SELECT ic.object_id, ic.column_id
                FROM sys.index_columns ic
                INNER JOIN sys.indexes i ON ic.object_id = i.object_id AND ic.index_id = i.index_id
                WHERE i.is_primary_key = 1
            ) pk ON pk.object_id = c.object_id AND pk.column_id = c.column_id
            LEFT JOIN sys.default_constraints dc ON dc.parent_object_id = c.object_id AND dc.parent_column_id = c.column_id
            WHERE s.name = '{}' AND tbl.name = '{}'
            ORDER BY c.column_id
            "#,
            schema_filter, table
        );

        let client = self.get_client().await?;
        let mut client = client.lock().await;

        let stream = client
            .simple_query(&query)
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let rows = stream
            .into_first_result()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let columns = rows
            .iter()
            .map(|row| {
                let name = match Self::convert_value(row, 0) {
                    Ok(QueryValue::String(s)) => s,
                    _ => String::new(),
                };
                let data_type = match Self::convert_value(row, 1) {
                    Ok(QueryValue::String(s)) => s,
                    _ => String::new(),
                };
                let nullable = match Self::convert_value(row, 2) {
                    Ok(QueryValue::Bool(b)) => b,
                    _ => false,
                };
                let max_length = match Self::convert_value(row, 3) {
                    Ok(QueryValue::Int(i)) => Some(i as u32),
                    _ => None,
                };
                let precision = match Self::convert_value(row, 4) {
                    Ok(QueryValue::Int(i)) => Some(i as u32),
                    _ => None,
                };
                let scale = match Self::convert_value(row, 5) {
                    Ok(QueryValue::Int(i)) => Some(i as u32),
                    _ => None,
                };
                let is_primary_key = match Self::convert_value(row, 6) {
                    Ok(QueryValue::Int(i)) => i == 1,
                    _ => false,
                };
                let is_auto_increment = match Self::convert_value(row, 7) {
                    Ok(QueryValue::Bool(b)) => b,
                    _ => false,
                };
                let default_value = match Self::convert_value(row, 8) {
                    Ok(QueryValue::String(s)) => Some(s),
                    _ => None,
                };

                ColumnInfo {
                    name,
                    data_type,
                    nullable,
                    default_value,
                    is_primary_key,
                    is_auto_increment,
                    max_length,
                    precision,
                    scale,
                    description: None,
                    metadata: HashMap::new(),
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

        let schema_filter = schema.unwrap_or("dbo");

        let query = format!(
            r#"
            SELECT 
                s.name as schema_name,
                t.name as table_name,
                t.type_desc as table_type,
                p.rows as row_count,
                SUM(a.total_pages) * 8 * 1024 as size_bytes
            FROM sys.tables t
            INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            WHERE s.name = '{}' AND t.name = '{}' AND p.index_id IN (0,1)
            GROUP BY s.name, t.name, t.type_desc, p.rows
            UNION ALL
            SELECT 
                s.name as schema_name,
                v.name as table_name,
                'VIEW' as table_type,
                NULL as row_count,
                NULL as size_bytes
            FROM sys.views v
            INNER JOIN sys.schemas s ON v.schema_id = s.schema_id
            WHERE s.name = '{}' AND v.name = '{}'
            "#,
            schema_filter, table, schema_filter, table
        );

        let client = self.get_client().await?;
        let mut client = client.lock().await;

        let stream = client
            .simple_query(&query)
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let rows = stream
            .into_first_result()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        if rows.is_empty() {
            return Err(DbError::TableNotFound(format!(
                "Table {}.{} not found",
                schema_filter, table
            )));
        }

        let row = &rows[0];
        let schema = match Self::convert_value(row, 0) {
            Ok(QueryValue::String(s)) => Some(s),
            _ => None,
        };
        let name = match Self::convert_value(row, 1) {
            Ok(QueryValue::String(s)) => s,
            _ => String::new(),
        };
        let table_type = match Self::convert_value(row, 2) {
            Ok(QueryValue::String(s)) => s,
            _ => String::new(),
        };
        let row_count = match Self::convert_value(row, 3) {
            Ok(QueryValue::Int(i)) => Some(i as u64),
            _ => None,
        };
        let size_bytes = match Self::convert_value(row, 4) {
            Ok(QueryValue::Int(i)) => Some(i as u64),
            _ => None,
        };

        Ok(TableInfo {
            schema,
            name,
            table_type,
            row_count,
            size_bytes,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseType;

    #[test]
    fn test_sqlserver_adapter_creation() {
        let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa")
            .with_database("testdb")
            .with_password("Password123!");

        let adapter = SqlServerAdapter::new(config);
        assert!(adapter.pool.is_none());
        assert_eq!(adapter.config.host, "localhost");
    }

    #[test]
    fn test_sqlserver_config_with_windows_auth() {
        let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "")
            .with_database("testdb")
            .with_option("use_windows_auth", "true");

        let adapter = SqlServerAdapter::new(config);
        assert_eq!(
            adapter.config.options.get("use_windows_auth"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_sqlserver_config_with_ssl() {
        let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa")
            .with_database("testdb")
            .with_password("Password123!")
            .with_ssl_mode(SslMode::Require);

        let adapter = SqlServerAdapter::new(config);
        assert_eq!(adapter.config.ssl_mode, SslMode::Require);
    }

    #[test]
    fn test_sqlserver_config_with_trust_cert() {
        let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa")
            .with_database("testdb")
            .with_password("Password123!")
            .with_option("trust_server_certificate", "true");

        let adapter = SqlServerAdapter::new(config);
        assert_eq!(
            adapter.config.options.get("trust_server_certificate"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_get_config() {
        let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa")
            .with_database("testdb");

        let adapter = SqlServerAdapter::new(config.clone());
        let retrieved_config = adapter.get_config();

        assert_eq!(retrieved_config.host, config.host);
        assert_eq!(retrieved_config.port, config.port);
        assert_eq!(retrieved_config.username, config.username);
        assert_eq!(retrieved_config.database, config.database);
    }

    #[test]
    fn test_pool_initially_none() {
        let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa");
        let adapter = SqlServerAdapter::new(config);

        assert!(adapter.get_pool().is_none());
    }
}
