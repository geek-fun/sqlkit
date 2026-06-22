//! RQLite HTTP database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for RQLite using the HTTP protocol via `reqwest`. RQLite is a distributed
//! SQLite database that exposes an HTTP API (default port 4001).
//!
//! # RQLite HTTP API
//!
//! - Query endpoint: `POST /db/query`
//! - Request body: JSON array of SQL strings: `["SELECT 1"]`
//! - Content-Type: `application/json`
//! - Auth: HTTP Basic Authentication (when username/password configured)
//!
//! Query response format:
//! ```json
//! {
//!   "results": [{
//!     "columns": ["col1", "col2"],
//!     "types": ["int", "text"],
//!     "values": [[1, "hello"], [2, "world"]]
//!   }]
//! }
//! ```
//!
//! Write response format:
//! ```json
//! {
//!   "results": [{
//!     "last_insert_id": 1,
//!     "rows_affected": 1
//!   }]
//! }
//! ```

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
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

// ---------------------------------------------------------------------------
// RQLite HTTP JSON response structures
// ---------------------------------------------------------------------------

/// A single result entry in the RQLite HTTP response.
///
/// Each element in the `results` array corresponds to one SQL statement
/// from the request batch.
#[derive(Debug, Deserialize)]
struct RqliteResult {
    /// Column names for a SELECT-like query.
    #[serde(default)]
    columns: Option<Vec<String>>,
    /// Column type annotations from RQLite.
    #[serde(default)]
    #[allow(dead_code)]
    types: Option<Vec<String>>,
    /// Positional row data (aligned with `columns`).
    #[serde(default)]
    values: Option<Vec<Vec<serde_json::Value>>>,

    /// Number of rows affected (for INSERT/UPDATE/DELETE).
    #[serde(default)]
    rows_affected: Option<i64>,
    /// Error message if the statement failed.
    #[serde(default)]
    error: Option<String>,
}

/// Top-level response from the RQLite HTTP API.
#[derive(Debug, Deserialize)]
struct RqliteResponse {
    results: Vec<RqliteResult>,
}

// ---------------------------------------------------------------------------
// RqlitePool — stateless HTTP "pool" wrapping a shared reqwest::Client
// ---------------------------------------------------------------------------

/// An RQLite connection pool.
///
/// Because the RQLite HTTP interface is stateless, the "pool" does not
/// maintain persistent connections. It holds a shared `reqwest::Client`
/// (which internally reuses HTTP connections via connection pooling) and the
/// base URL for the target server.
pub struct RqlitePool {
    client: reqwest::Client,
    #[allow(dead_code)]
    base_url: String,
}

#[async_trait]
impl ConnectionPool for RqlitePool {
    type Connection = reqwest::Client;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        Ok(Arc::new(self.client.clone()))
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
        0
    }

    async fn close(&self) -> DbResult<()> {
        Ok(())
    }

    async fn health_check(&self) -> DbResult<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RqliteAdapter
// ---------------------------------------------------------------------------

/// RQLite database adapter using the HTTP protocol.
///
/// Sends SQL queries to RQLite's HTTP endpoint and parses the JSON response.
/// Supports Basic authentication via the `Authorization` header.
///
/// # Example
///
/// ```ignore
/// use sqlkit::database::{DatabaseAdapter, ConnectionConfig, DatabaseType};
/// use sqlkit::database::rqlite::RqliteAdapter;
///
/// async fn example() {
///     let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "user")
///         .with_password("pass");
///     let mut adapter = RqliteAdapter::new(config);
///     adapter.connect().await.unwrap();
///     let result = adapter.execute_query("SELECT 1").await.unwrap();
///     adapter.disconnect().await.unwrap();
/// }
/// ```
pub struct RqliteAdapter {
    pub config: ConnectionConfig,
    client: Option<reqwest::Client>,
    pool: Option<Arc<RqlitePool>>,
}

impl RqliteAdapter {
    /// Create a new `RqliteAdapter` from the supplied configuration.
    ///
    /// The adapter starts in a disconnected state; call [`connect`] before
    /// issuing any queries.
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            client: None,
            pool: None,
        }
    }

    /// Build the base URL from the configuration.
    fn build_base_url(&self) -> String {
        let scheme = if self.config.ssl_mode == SslMode::Disable {
            "http"
        } else {
            "https"
        };
        format!("{}://{}:{}", scheme, self.config.host, self.config.port)
    }

    /// Create the `reqwest::Client` used for all HTTP calls.
    fn build_client(&self) -> DbResult<reqwest::Client> {
        let timeout = std::time::Duration::from_secs(self.config.connect_timeout_secs);
        let mut builder = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent("sqlkit-rqlite-adapter/0.1");

        builder = self.apply_ssl_to_builder(builder)?;

        builder
            .build()
            .map_err(|e| DbError::Connection(format!("Failed to create HTTP client: {}", e)))
    }

    fn apply_ssl_to_builder(
        &self,
        mut builder: reqwest::ClientBuilder,
    ) -> DbResult<reqwest::ClientBuilder> {
        match self.config.ssl_mode {
            SslMode::Disable => {}
            SslMode::Prefer | SslMode::Require => {
                builder = builder.danger_accept_invalid_certs(true);
            }
            SslMode::VerifyCA | SslMode::VerifyFull => {
                if let Some(ref ca_cert) = self.config.ssl_ca_cert {
                    let pem = std::fs::read(ca_cert).map_err(|e| {
                        DbError::Connection(format!("Failed to read CA certificate: {}", e))
                    })?;
                    let cert = reqwest::Certificate::from_pem(&pem).map_err(|e| {
                        DbError::Connection(format!("Failed to parse CA certificate: {}", e))
                    })?;
                    builder = builder.add_root_certificate(cert);
                }
            }
        }
        if let (Some(ref cert_path), Some(ref key_path)) =
            (&self.config.ssl_client_cert, &self.config.ssl_client_key)
        {
            let cert_pem = std::fs::read(cert_path).map_err(|e| {
                DbError::Connection(format!("Failed to read client certificate: {}", e))
            })?;
            let key_pem = std::fs::read(key_path)
                .map_err(|e| DbError::Connection(format!("Failed to read client key: {}", e)))?;
            let mut combined = cert_pem;
            combined.extend_from_slice(&key_pem);
            let identity = reqwest::Identity::from_pem(&combined).map_err(|e| {
                DbError::Connection(format!("Failed to parse client identity: {}", e))
            })?;
            builder = builder.identity(identity);
        }
        Ok(builder)
    }

    /// Build the HTTP headers for a request.
    ///
    /// Adds `Content-Type: application/json` and a `Basic` authorization header when
    /// a password is configured.
    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(ref password) = self.config.password {
            let auth_data = format!("{}:{}", self.config.username, password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(auth_data.as_bytes());
            if let Ok(auth_value) = HeaderValue::from_str(&format!("Basic {}", encoded)) {
                headers.insert(AUTHORIZATION, auth_value);
            }
        }

        headers
    }

    /// Serialize a SQL string into the RQLite JSON request body.
    ///
    /// RQLite expects a JSON array of SQL strings: `["query1", "query2"]`.
    fn build_request_body(query: &str) -> DbResult<String> {
        serde_json::to_string(&vec![query]).map_err(|e| {
            DbError::Serialization(format!("Failed to serialize RQLite request body: {}", e))
        })
    }

    /// Send a query to RQLite and return the parsed JSON response.
    async fn send_query(&self, query: &str) -> DbResult<RqliteResponse> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected to RQLite".to_string()))?;

        let url = format!("{}/db/query", self.build_base_url());
        let headers = self.build_headers();
        let body = Self::build_request_body(query)?;

        let response = client
            .post(&url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DbError::Timeout(format!("RQLite query timed out: {}", e))
                } else if e.is_connect() {
                    DbError::Connection(format!(
                        "Cannot connect to RQLite at {}: {}",
                        self.build_base_url(),
                        e
                    ))
                } else {
                    DbError::QueryExecution(format!("HTTP request failed: {}", e))
                }
            })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| DbError::QueryExecution(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            return Err(DbError::QueryExecution(format!(
                "RQLite error (HTTP {}): {}",
                status.as_u16(),
                body.trim()
            )));
        }

        let resp: RqliteResponse = serde_json::from_str(&body).map_err(|e| {
            DbError::Serialization(format!(
                "Failed to parse RQLite JSON response: {} (body preview: {})",
                e,
                body.chars().take(200).collect::<String>()
            ))
        })?;

        // Check for SQL-level errors in any result
        for result in &resp.results {
            if let Some(ref err) = result.error {
                return Err(DbError::QueryExecution(format!(
                    "RQLite query error: {}",
                    err
                )));
            }
        }

        Ok(resp)
    }

    /// Convert a `serde_json::Value` into a `QueryValue`.
    fn json_to_query_value(value: serde_json::Value) -> QueryValue {
        match value {
            serde_json::Value::Null => QueryValue::Null,
            serde_json::Value::Bool(b) => QueryValue::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    QueryValue::Int(i)
                } else if let Some(f) = n.as_f64() {
                    QueryValue::Float(f)
                } else {
                    QueryValue::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => QueryValue::String(s),
            serde_json::Value::Array(arr) => {
                QueryValue::String(serde_json::Value::Array(arr).to_string())
            }
            serde_json::Value::Object(obj) => {
                QueryValue::String(serde_json::Value::Object(obj).to_string())
            }
        }
    }

    /// Build a positional row map from the RQLite columns + values arrays.
    ///
    /// RQLite returns data positionally: `columns` and `values` arrays are aligned
    /// by index. This helper zips them into a `HashMap<&str, &Value>` for ergonomic
    /// access by column name.
    fn zip_row<'a>(
        columns: &'a [String],
        values: &'a [serde_json::Value],
    ) -> HashMap<&'a str, &'a serde_json::Value> {
        columns
            .iter()
            .zip(values.iter())
            .map(|(col, val)| (col.as_str(), val))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// DatabaseAdapter trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl DatabaseAdapter for RqliteAdapter {
    type Pool = RqlitePool;

    async fn connect(&mut self) -> DbResult<()> {
        let client = self.build_client()?;

        // Verify connectivity by sending SELECT 1
        let url = format!("{}/db/query", self.build_base_url());
        let headers = self.build_headers();
        let body = Self::build_request_body("SELECT 1")?;

        let response = client
            .post(&url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    DbError::Connection(format!(
                        "Cannot connect to RQLite at {}: {}",
                        self.build_base_url(),
                        e
                    ))
                } else {
                    DbError::Connection(format!("RQLite connection test failed: {}", e))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let resp_body = response.text().await.unwrap_or_default();
            return Err(DbError::Connection(format!(
                "RQLite rejected connection (HTTP {}): {}",
                status.as_u16(),
                resp_body.trim()
            )));
        }

        let base_url = self.build_base_url();
        self.client = Some(client.clone());
        self.pool = Some(Arc::new(RqlitePool { client, base_url }));

        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.client = None;
        self.pool = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let _resp = self.send_query("SELECT 1").await?;

        Ok(ConnectionStatus {
            is_connected: true,
            server_version: None, // RQLite does not expose version via SQL
            current_database: Some("rqlite".to_string()),
            current_user: self.config.username.clone().into(),
            metadata: HashMap::new(),
        })
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let start = Instant::now();
        let response = self.send_query(query).await?;
        let execution_time = start.elapsed().as_millis() as u64;

        // Process the first result (we only send single-statement queries)
        let result = match response.results.into_iter().next() {
            Some(r) => r,
            None => {
                return Ok(QueryResult::affected(0).with_execution_time(execution_time));
            }
        };

        // If columns are present, this is a SELECT-like query
        if let Some(columns) = result.columns {
            let values = result.values.unwrap_or_default();

            let rows: Vec<QueryRow> = values
                .into_iter()
                .map(|row| {
                    let mut map = HashMap::new();
                    for (i, val) in row.into_iter().enumerate() {
                        let col_name = columns
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| format!("col_{}", i));
                        map.insert(col_name, Self::json_to_query_value(val));
                    }
                    map
                })
                .collect();

            let mut qr = QueryResult::new(columns);
            for row in rows {
                qr.add_row(row);
            }
            qr.execution_time_ms = Some(execution_time);
            Ok(qr)
        } else if let Some(rows_affected) = result.rows_affected {
            // Write operation (INSERT/UPDATE/DELETE)
            let mut qr = QueryResult::affected(rows_affected as u64);
            qr.execution_time_ms = Some(execution_time);
            Ok(qr)
        } else {
            // DDL or other statement with no result set
            let mut qr = QueryResult::affected(0);
            qr.execution_time_ms = Some(execution_time);
            Ok(qr)
        }
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        // RQLite is a single-database system
        Ok(vec![DatabaseSchema {
            name: "rqlite".to_string(),
            description: Some("RQLite distributed database".to_string()),
            is_system: false,
            metadata: HashMap::new(),
        }])
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        // SQLite/RQLite does not have named schemas; everything is in "main"
        Ok(vec!["main".to_string()])
    }

    async fn list_tables(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let resp = self
            .send_query(
                "SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') ORDER BY name",
            )
            .await?;

        let result = match resp.results.into_iter().next() {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        let columns = match result.columns {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };
        let values = result.values.unwrap_or_default();

        let tables = values
            .into_iter()
            .filter_map(|row| {
                let row_map = Self::zip_row(&columns, &row);

                let name = row_map.get("name")?.as_str()?.to_string();
                let obj_type = row_map
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("table");
                let table_type = if obj_type.eq_ignore_ascii_case("view") {
                    "VIEW".to_string()
                } else {
                    "TABLE".to_string()
                };

                Some(TableInfo {
                    schema: Some("main".to_string()),
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
        _schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        let escaped_table = table.replace('\'', "''");
        let query = format!("PRAGMA table_info('{}')", escaped_table);
        let resp = self.send_query(&query).await?;

        let result = match resp.results.into_iter().next() {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        let columns = match result.columns {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };
        let values = result.values.unwrap_or_default();

        // PRAGMA table_info returns: cid, name, type, notnull, dflt_value, pk
        let col_info = values
            .into_iter()
            .filter_map(|row| {
                let row_map = Self::zip_row(&columns, &row);

                let name = row_map.get("name")?.as_str()?.to_string();
                let data_type = row_map
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("TEXT")
                    .to_string();
                let not_null = row_map.get("notnull").and_then(|v| v.as_i64()).unwrap_or(0);
                let default_value = row_map
                    .get("dflt_value")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let is_pk = row_map.get("pk").and_then(|v| v.as_i64()).unwrap_or(0) != 0;

                Some(ColumnInfo {
                    name,
                    data_type,
                    nullable: not_null == 0,
                    default_value,
                    is_primary_key: is_pk,
                    is_auto_increment: false,
                    max_length: None,
                    precision: None,
                    scale: None,
                    description: None,
                    metadata: HashMap::new(),
                })
            })
            .collect();

        Ok(col_info)
    }

    async fn get_table_info(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        // First check the table exists and get its type
        let escaped_table = table.replace('\'', "''");
        let check_query = format!(
            "SELECT name, type FROM sqlite_master WHERE name = '{}'",
            escaped_table
        );
        let check_resp = self.send_query(&check_query).await?;

        let check_result = check_resp
            .results
            .into_iter()
            .next()
            .ok_or_else(|| DbError::TableNotFound(format!("Table not found: {}", table)))?;

        let check_columns = check_result
            .columns
            .ok_or_else(|| DbError::TableNotFound(format!("Table not found: {}", table)))?;
        let check_values = check_result
            .values
            .ok_or_else(|| DbError::TableNotFound(format!("Table not found: {}", table)))?;

        let check_row = check_values
            .into_iter()
            .next()
            .ok_or_else(|| DbError::TableNotFound(format!("Table not found: {}", table)))?;

        let row_map = Self::zip_row(&check_columns, &check_row);

        let name = row_map
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(table)
            .to_string();
        let obj_type = row_map
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("table");
        let table_type = if obj_type.eq_ignore_ascii_case("view") {
            "VIEW".to_string()
        } else {
            "TABLE".to_string()
        };

        // Get row count via SELECT COUNT(*)
        let escaped_table_dq = table.replace('"', "\"\"");
        let count_query = format!("SELECT COUNT(*) AS count FROM \"{}\"", escaped_table_dq);
        let row_count = {
            let count_resp = self.send_query(&count_query).await?;
            count_resp
                .results
                .into_iter()
                .next()
                .and_then(|r| r.values)
                .and_then(|v| v.into_iter().next())
                .and_then(|row| row.into_iter().next())
                .and_then(|val| val.as_u64())
        };

        Ok(TableInfo {
            schema: Some("main".to_string()),
            name,
            table_type,
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseType;

    // ---- Construction ----

    #[test]
    fn test_new_adapter_is_disconnected() {
        let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "default");
        let adapter = RqliteAdapter::new(config);
        assert!(adapter.client.is_none());
        assert!(adapter.pool.is_none());
    }

    #[test]
    fn test_get_config() {
        let config =
            ConnectionConfig::new(DatabaseType::RQLite, "rqlite.example.com", 4001, "admin")
                .with_database("default");
        let adapter = RqliteAdapter::new(config.clone());
        let cfg = adapter.get_config();
        assert_eq!(cfg.host, "rqlite.example.com");
        assert_eq!(cfg.port, 4001);
        assert_eq!(cfg.username, "admin");
        assert_eq!(cfg.database.as_deref(), Some("default"));
    }

    #[test]
    fn test_get_pool_initially_none() {
        let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "default");
        let adapter = RqliteAdapter::new(config);
        assert!(adapter.get_pool().is_none());
    }

    // ---- URL building ----

    #[test]
    fn test_build_base_url() {
        let config =
            ConnectionConfig::new(DatabaseType::RQLite, "rqlite.example.com", 4001, "default")
                .with_ssl_mode(SslMode::Disable);
        let adapter = RqliteAdapter::new(config);
        assert_eq!(adapter.build_base_url(), "http://rqlite.example.com:4001");
    }

    #[test]
    fn test_build_base_url_non_default_port() {
        let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "default")
            .with_ssl_mode(SslMode::Disable);
        let adapter = RqliteAdapter::new(config);
        assert_eq!(adapter.build_base_url(), "http://localhost:4001");
    }

    #[test]
    fn test_build_base_url_https() {
        let config =
            ConnectionConfig::new(DatabaseType::RQLite, "rqlite.example.com", 4001, "default")
                .with_ssl_mode(SslMode::Prefer);
        let adapter = RqliteAdapter::new(config);
        assert_eq!(adapter.build_base_url(), "https://rqlite.example.com:4001");
    }

    #[test]
    fn test_build_base_url_https_require() {
        let config =
            ConnectionConfig::new(DatabaseType::RQLite, "rqlite.example.com", 4001, "default")
                .with_ssl_mode(SslMode::Require);
        let adapter = RqliteAdapter::new(config);
        assert_eq!(adapter.build_base_url(), "https://rqlite.example.com:4001");
    }

    // ---- Headers ----

    #[test]
    fn test_build_headers_with_password() {
        let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "default")
            .with_password("s3cret");
        let adapter = RqliteAdapter::new(config);
        let headers = adapter.build_headers();
        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
    }

    #[test]
    fn test_build_headers_without_password() {
        let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "default");
        let adapter = RqliteAdapter::new(config);
        let headers = adapter.build_headers();
        assert!(!headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
    }

    // ---- Request body ----

    #[test]
    fn test_build_request_body() {
        let body = RqliteAdapter::build_request_body("SELECT 1").unwrap();
        assert_eq!(body, "[\"SELECT 1\"]");
    }

    #[test]
    fn test_build_request_body_with_special_chars() {
        let body = RqliteAdapter::build_request_body("INSERT INTO t VALUES ('a', 'b')").unwrap();
        assert_eq!(body, "[\"INSERT INTO t VALUES ('a', 'b')\"]");
    }

    // ---- QueryValue conversion ----

    #[test]
    fn test_json_null_to_query_value() {
        assert_eq!(
            RqliteAdapter::json_to_query_value(serde_json::Value::Null),
            QueryValue::Null,
        );
    }

    #[test]
    fn test_json_bool_to_query_value() {
        assert_eq!(
            RqliteAdapter::json_to_query_value(serde_json::Value::Bool(true)),
            QueryValue::Bool(true),
        );
    }

    #[test]
    fn test_json_int_to_query_value() {
        assert_eq!(
            RqliteAdapter::json_to_query_value(serde_json::json!(42)),
            QueryValue::Int(42),
        );
    }

    #[test]
    fn test_json_negative_int_to_query_value() {
        assert_eq!(
            RqliteAdapter::json_to_query_value(serde_json::json!(-7)),
            QueryValue::Int(-7),
        );
    }

    #[test]
    fn test_json_float_to_query_value() {
        assert_eq!(
            RqliteAdapter::json_to_query_value(serde_json::json!(3.14)),
            QueryValue::Float(3.14),
        );
    }

    #[test]
    fn test_json_string_to_query_value() {
        assert_eq!(
            RqliteAdapter::json_to_query_value(serde_json::Value::String("hello".to_string())),
            QueryValue::String("hello".to_string()),
        );
    }

    #[test]
    fn test_json_array_to_query_value() {
        assert_eq!(
            RqliteAdapter::json_to_query_value(serde_json::json!([1, "a", true])),
            QueryValue::String("[1,\"a\",true]".to_string()),
        );
    }

    #[test]
    fn test_json_object_to_query_value() {
        let obj = serde_json::json!({"key": "value"});
        let result = RqliteAdapter::json_to_query_value(obj);
        assert_eq!(
            result,
            QueryValue::String("{\"key\":\"value\"}".to_string())
        );
    }

    // ---- Disconnect ----

    #[test]
    fn test_disconnect_clears_state() {
        let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "default");
        let mut adapter = RqliteAdapter::new(config);
        // Simulate connected state
        adapter.client = Some(reqwest::Client::new());
        adapter.pool = Some(Arc::new(RqlitePool {
            client: reqwest::Client::new(),
            base_url: "http://localhost:4001".to_string(),
        }));

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            adapter.disconnect().await.unwrap();
        });

        assert!(adapter.client.is_none());
        assert!(adapter.pool.is_none());
    }

    // ---- RQLite response parsing ----

    #[test]
    fn test_parse_query_response() {
        let json = r#"{
            "results": [{
                "columns": ["id", "name"],
                "types": ["integer", "text"],
                "values": [[1, "hello"], [2, "world"]]
            }]
        }"#;
        let resp: RqliteResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.results.len(), 1);
        let result = &resp.results[0];
        assert_eq!(
            result.columns.as_ref().unwrap(),
            &vec!["id".to_string(), "name".to_string()]
        );
        assert!(result.error.is_none());
    }

    #[test]
    fn test_parse_write_response() {
        let json = r#"{
            "results": [{
                "rows_affected": 1
            }]
        }"#;
        let resp: RqliteResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.results.len(), 1);
        let result = &resp.results[0];
        assert_eq!(result.rows_affected, Some(1));
    }

    #[test]
    fn test_parse_error_response() {
        let json = r#"{
            "results": [{
                "error": "no such table: nonexistent"
            }]
        }"#;
        let resp: RqliteResponse = serde_json::from_str(json).unwrap();
        let result = &resp.results[0];
        assert_eq!(result.error.as_ref().unwrap(), "no such table: nonexistent");
    }

    #[test]
    fn test_zip_row() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let values = vec![serde_json::json!(1), serde_json::json!("alice")];
        let map = RqliteAdapter::zip_row(&columns, &values);
        assert_eq!(map.get("id").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(map.get("name").and_then(|v| v.as_str()), Some("alice"));
    }

    #[test]
    fn test_list_databases_returns_rqlite() {
        let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "default");
        let adapter = RqliteAdapter::new(config);
        let rt = tokio::runtime::Runtime::new().unwrap();
        // This only tests the logic since connect is not called
        // (list_databases doesn't need connection for this adapter)
        let dbs = rt.block_on(async { adapter.list_databases().await.unwrap() });
        assert_eq!(dbs.len(), 1);
        assert_eq!(dbs[0].name, "rqlite");
    }

    #[test]
    fn test_list_schemas_returns_main() {
        let config = ConnectionConfig::new(DatabaseType::RQLite, "localhost", 4001, "default");
        let adapter = RqliteAdapter::new(config);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let schemas = rt.block_on(async { adapter.list_schemas(None).await.unwrap() });
        assert_eq!(schemas, vec!["main"]);
    }
}
