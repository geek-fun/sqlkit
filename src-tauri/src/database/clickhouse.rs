//! ClickHouse HTTP database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for ClickHouse using the HTTP protocol via `reqwest`. It communicates with
//! the ClickHouse server through its native HTTP interface (default port 8123),
//! using `default_format=JSON` for structured responses.

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
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

// ---------------------------------------------------------------------------
// ClickHouse HTTP JSON response structures
// ---------------------------------------------------------------------------

/// Metadata for a single column in a ClickHouse response.
#[derive(Debug, Deserialize)]
struct ClickHouseMetaColumn {
    name: String,
    #[serde(rename = "type")]
    col_type: String,
}

/// Statistics included in a ClickHouse response.
#[derive(Debug, Deserialize)]
struct ClickHouseStatistics {
    elapsed: Option<f64>,
    rows_read: Option<u64>,
    bytes_read: Option<u64>,
}

/// Top-level response from the ClickHouse HTTP API (`default_format=JSON`).
#[derive(Debug, Deserialize)]
struct ClickHouseResponse {
    meta: Vec<ClickHouseMetaColumn>,
    data: Vec<HashMap<String, serde_json::Value>>,
    rows: u64,
    statistics: Option<ClickHouseStatistics>,
}

// ---------------------------------------------------------------------------
// ClickHousePool — stateless HTTP "pool" wrapping a shared reqwest::Client
// ---------------------------------------------------------------------------

/// A ClickHouse connection pool.
///
/// Because the ClickHouse HTTP interface is stateless, the "pool" does not
/// maintain persistent connections.  It holds a shared `reqwest::Client`
/// (which internally reuses HTTP connections via connection pooling) and the
/// base URL for the target server.
pub struct ClickHousePool {
    client: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl ConnectionPool for ClickHousePool {
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
// ClickHouseAdapter
// ---------------------------------------------------------------------------

/// ClickHouse database adapter using the HTTP protocol.
///
/// Sends SQL queries to ClickHouse's built-in HTTP endpoint and parses the
/// `JSON` format response.  Supports Basic authentication via the
/// `Authorization` header.
pub struct ClickHouseAdapter {
    pub config: ConnectionConfig,
    client: Option<reqwest::Client>,
    pool: Option<Arc<ClickHousePool>>,
}

impl ClickHouseAdapter {
    /// Create a new `ClickHouseAdapter` from the supplied configuration.
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

    /// Build the base URL (`http://host:port`) from the configuration.
    fn build_base_url(&self) -> String {
        format!("http://{}:{}", self.config.host, self.config.port)
    }

    /// Create the `reqwest::Client` used for all HTTP calls.
    fn build_client() -> DbResult<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("sqlkit-clickhouse-adapter/0.1")
            .build()
            .map_err(|e| DbError::Connection(format!("Failed to create HTTP client: {}", e)))
    }

    /// Build the HTTP headers for a request.
    ///
    /// Adds `Content-Type: text/plain` and a `Basic` authorization header when
    /// a password is configured.
    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=UTF-8"),
        );

        if let Some(ref password) = self.config.password {
            let auth_data = format!("{}:{}", self.config.username, password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(auth_data.as_bytes());
            if let Ok(auth_value) = HeaderValue::from_str(&format!("Basic {}", encoded)) {
                headers.insert(AUTHORIZATION, auth_value);
            }
        }

        headers
    }

    /// Send a query to ClickHouse and return the parsed JSON response.
    async fn send_query(&self, query: &str) -> DbResult<ClickHouseResponse> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected to ClickHouse".to_string()))?;

        let url = format!("{}/?default_format=JSON", self.build_base_url());
        let headers = self.build_headers();

        let response = client
            .post(&url)
            .headers(headers)
            .body(query.to_owned())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DbError::Timeout(format!("ClickHouse query timed out: {}", e))
                } else if e.is_connect() {
                    DbError::Connection(format!(
                        "Cannot connect to ClickHouse at {}: {}",
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
                "ClickHouse error (HTTP {}): {}",
                status.as_u16(),
                body.trim()
            )));
        }

        serde_json::from_str::<ClickHouseResponse>(&body).map_err(|e| {
            // ClickHouse sometimes returns plain-text errors even with HTTP 200
            if body.trim().starts_with("Code:") || body.contains("DB::Exception") {
                DbError::QueryExecution(format!("ClickHouse error: {}", body.trim()))
            } else {
                DbError::Serialization(format!(
                    "Failed to parse ClickHouse JSON response: {} (body preview: {})",
                    e,
                    body.chars().take(200).collect::<String>()
                ))
            }
        })
    }

    /// Convert a `serde_json::Value` into a `QueryValue`.
    fn json_to_query_value(value: serde_json::Value) -> QueryValue {
        match value {
            serde_json::Value::Null => QueryValue::Null,
            serde_json::Value::Bool(b) => QueryValue::Bool(b),
            serde_json::Value::Number(n) => {
                // Try i64 first, then u64 (for large UInt64 values), then f64
                if let Some(i) = n.as_i64() {
                    QueryValue::Int(i)
                } else if let Some(u) = n.as_u64() {
                    // UInt64 too large for i64 — fall back to string representation
                    QueryValue::String(u.to_string())
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
}

// ---------------------------------------------------------------------------
// DatabaseAdapter trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl DatabaseAdapter for ClickHouseAdapter {
    type Pool = ClickHousePool;

    async fn connect(&mut self) -> DbResult<()> {
        let client = Self::build_client()?;

        // Verify connectivity by sending a simple query
        let url = format!("{}/?default_format=JSON", self.build_base_url());
        let headers = self.build_headers();

        let response = client
            .post(&url)
            .headers(headers)
            .body("SELECT 1".to_owned())
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    DbError::Connection(format!(
                        "Cannot connect to ClickHouse at {}: {}",
                        self.build_base_url(),
                        e
                    ))
                } else {
                    DbError::Connection(format!("ClickHouse connection test failed: {}", e))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(DbError::Connection(format!(
                "ClickHouse rejected connection (HTTP {}): {}",
                status.as_u16(),
                body.trim()
            )));
        }

        let base_url = self.build_base_url();
        self.client = Some(client.clone());
        self.pool = Some(Arc::new(ClickHousePool { client, base_url }));

        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.client = None;
        self.pool = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let version_resp = self.send_query("SELECT version() AS version").await?;
        let server_version = version_resp
            .data
            .first()
            .and_then(|row| row.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let info_resp = self
            .send_query("SELECT currentDatabase() AS db, currentUser() AS user")
            .await?;
        let current_database = info_resp
            .data
            .first()
            .and_then(|row| row.get("db"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let current_user = info_resp
            .data
            .first()
            .and_then(|row| row.get("user"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ConnectionStatus {
            is_connected: true,
            server_version,
            current_database,
            current_user,
            metadata: HashMap::new(),
        })
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let start = Instant::now();
        let response = self.send_query(query).await?;
        let execution_time = start.elapsed().as_millis() as u64;

        let columns: Vec<String> = response.meta.into_iter().map(|col| col.name).collect();

        let rows: Vec<QueryRow> = response
            .data
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(k, v)| (k, Self::json_to_query_value(v)))
                    .collect()
            })
            .collect();

        let mut result = if !columns.is_empty() {
            let mut r = QueryResult::new(columns);
            for row in rows {
                r.add_row(row);
            }
            r
        } else if response.rows > 0 {
            // DML like INSERT — use rows as affected count
            QueryResult::affected(response.rows)
        } else {
            QueryResult::affected(0)
        };

        result.execution_time_ms = Some(execution_time);
        Ok(result)
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        let response = self.send_query("SHOW DATABASES").await?;

        let databases = response
            .data
            .into_iter()
            .filter_map(|row| {
                row.get("name").and_then(|v| v.as_str()).map(|name| {
                    let is_system =
                        matches!(name, "system" | "INFORMATION_SCHEMA" | "information_schema");
                    DatabaseSchema {
                        name: name.to_string(),
                        description: None,
                        is_system,
                        metadata: HashMap::new(),
                    }
                })
            })
            .collect();

        Ok(databases)
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        // Like MySQL, ClickHouse uses databases as the top-level namespace
        let databases = self.list_databases().await?;
        Ok(databases.into_iter().map(|db| db.name).collect())
    }

    async fn list_tables(
        &self,
        database: Option<&str>,
        _schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let db = database
            .or(self.config.database.as_deref())
            .unwrap_or("default");

        let query = format!(
            "SELECT name, engine, total_rows, total_bytes, comment \
             FROM system.tables \
             WHERE database = '{}' \
             ORDER BY name",
            db.replace('\'', "\\'")
        );

        let response = self.send_query(&query).await?;

        let tables = response
            .data
            .into_iter()
            .filter_map(|row| {
                let name = row.get("name")?.as_str()?.to_string();
                let engine = row
                    .get("engine")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let table_type = if engine.to_uppercase().contains("VIEW") {
                    "VIEW".to_string()
                } else {
                    "TABLE".to_string()
                };
                let row_count = row.get("total_rows").and_then(|v| v.as_u64());
                let size_bytes = row.get("total_bytes").and_then(|v| v.as_u64());
                let description = row
                    .get("comment")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());

                let mut metadata = HashMap::new();
                metadata.insert("engine".to_string(), engine);

                Some(TableInfo {
                    schema: Some(db.to_string()),
                    name,
                    table_type,
                    row_count,
                    size_bytes,
                    description,
                    metadata,
                })
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
        let db = database
            .or(self.config.database.as_deref())
            .unwrap_or("default");

        let query = format!(
            "SELECT name, type, position, default_kind, default_expression, \
                    comment, is_in_primary_key \
             FROM system.columns \
             WHERE database = '{}' AND table = '{}' \
             ORDER BY position",
            db.replace('\'', "\\'"),
            table.replace('\'', "\\'")
        );

        let response = self.send_query(&query).await?;

        let columns = response
            .data
            .into_iter()
            .filter_map(|row| {
                let name = row.get("name")?.as_str()?.to_string();
                let raw_type = row.get("type")?.as_str()?.to_string();
                let raw_type_lower = raw_type.to_lowercase();

                // Determine nullability from the Nullable(...) wrapper
                let is_nullable = raw_type_lower.starts_with("nullable(");
                let data_type = if is_nullable {
                    raw_type
                        .strip_prefix("Nullable(")
                        .and_then(|s| s.strip_suffix(')'))
                        .unwrap_or(&raw_type)
                        .to_string()
                } else {
                    raw_type.clone()
                };

                // Parse default value expression if a meaningful default exists
                let default_value = match row.get("default_kind").and_then(|v| v.as_str()) {
                    Some("DEFAULT") | Some("MATERIALIZED") | Some("ALIAS") => row
                        .get("default_expression")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                    _ => None,
                };

                let description = row
                    .get("comment")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());

                let is_primary_key = row
                    .get("is_in_primary_key")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    != 0;

                Some(ColumnInfo {
                    name,
                    data_type,
                    nullable: is_nullable,
                    default_value,
                    is_primary_key,
                    is_auto_increment: false, // ClickHouse has no auto_increment
                    max_length: None,
                    precision: None,
                    scale: None,
                    description,
                    metadata: HashMap::new(),
                })
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
        let db = database
            .or(self.config.database.as_deref())
            .unwrap_or("default");

        let query = format!(
            "SELECT name, engine, total_rows, total_bytes, comment \
             FROM system.tables \
             WHERE database = '{}' AND name = '{}'",
            db.replace('\'', "\\'"),
            table.replace('\'', "\\'")
        );

        let response = self.send_query(&query).await?;

        let row =
            response.data.into_iter().next().ok_or_else(|| {
                DbError::TableNotFound(format!("Table {}.{} not found", db, table))
            })?;

        let name = row
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(table)
            .to_string();
        let engine = row
            .get("engine")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let table_type = if engine.to_uppercase().contains("VIEW") {
            "VIEW".to_string()
        } else {
            "TABLE".to_string()
        };
        let row_count = row.get("total_rows").and_then(|v| v.as_u64());
        let size_bytes = row.get("total_bytes").and_then(|v| v.as_u64());
        let description = row
            .get("comment")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let mut metadata = HashMap::new();
        metadata.insert("engine".to_string(), engine);

        Ok(TableInfo {
            schema: Some(db.to_string()),
            name,
            table_type,
            row_count,
            size_bytes,
            description,
            metadata,
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
        let config = ConnectionConfig::new(DatabaseType::ClickHouse, "localhost", 8123, "default");
        let adapter = ClickHouseAdapter::new(config);
        assert!(adapter.client.is_none());
        assert!(adapter.pool.is_none());
    }

    #[test]
    fn test_get_config() {
        let config =
            ConnectionConfig::new(DatabaseType::ClickHouse, "ch.example.com", 8443, "admin")
                .with_database("analytics");
        let adapter = ClickHouseAdapter::new(config.clone());
        let cfg = adapter.get_config();
        assert_eq!(cfg.host, "ch.example.com");
        assert_eq!(cfg.port, 8443);
        assert_eq!(cfg.username, "admin");
        assert_eq!(cfg.database.as_deref(), Some("analytics"));
    }

    #[test]
    fn test_get_pool_initially_none() {
        let config = ConnectionConfig::new(DatabaseType::ClickHouse, "localhost", 8123, "default");
        let adapter = ClickHouseAdapter::new(config);
        assert!(adapter.get_pool().is_none());
    }

    // ---- URL building ----

    #[test]
    fn test_build_base_url() {
        let config =
            ConnectionConfig::new(DatabaseType::ClickHouse, "ch.example.com", 8123, "default");
        let adapter = ClickHouseAdapter::new(config);
        assert_eq!(adapter.build_base_url(), "http://ch.example.com:8123");
    }

    #[test]
    fn test_build_base_url_non_default_port() {
        let config = ConnectionConfig::new(DatabaseType::ClickHouse, "localhost", 8443, "default");
        let adapter = ClickHouseAdapter::new(config);
        assert_eq!(adapter.build_base_url(), "http://localhost:8443");
    }

    // ---- Headers ----

    #[test]
    fn test_build_headers_with_password() {
        let config = ConnectionConfig::new(DatabaseType::ClickHouse, "localhost", 8123, "default")
            .with_password("s3cret");
        let adapter = ClickHouseAdapter::new(config);
        let headers = adapter.build_headers();
        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
    }

    #[test]
    fn test_build_headers_without_password() {
        let config = ConnectionConfig::new(DatabaseType::ClickHouse, "localhost", 8123, "default");
        let adapter = ClickHouseAdapter::new(config);
        let headers = adapter.build_headers();
        assert!(!headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
    }

    // ---- QueryValue conversion ----

    #[test]
    fn test_json_null_to_query_value() {
        assert_eq!(
            ClickHouseAdapter::json_to_query_value(serde_json::Value::Null),
            QueryValue::Null,
        );
    }

    #[test]
    fn test_json_bool_to_query_value() {
        assert_eq!(
            ClickHouseAdapter::json_to_query_value(serde_json::Value::Bool(true)),
            QueryValue::Bool(true),
        );
    }

    #[test]
    fn test_json_int_to_query_value() {
        assert_eq!(
            ClickHouseAdapter::json_to_query_value(serde_json::json!(42)),
            QueryValue::Int(42),
        );
    }

    #[test]
    fn test_json_negative_int_to_query_value() {
        assert_eq!(
            ClickHouseAdapter::json_to_query_value(serde_json::json!(-7)),
            QueryValue::Int(-7),
        );
    }

    #[test]
    fn test_json_large_uint64_to_query_value() {
        // 2^63 = 9_223_372_036_854_775_808 — exceeds i64::MAX
        let large = serde_json::json!(9_223_372_036_854_775_808u64);
        let result = ClickHouseAdapter::json_to_query_value(large);
        assert_eq!(
            result,
            QueryValue::String("9223372036854775808".to_string())
        );
    }

    #[test]
    fn test_json_float_to_query_value() {
        assert_eq!(
            ClickHouseAdapter::json_to_query_value(serde_json::json!(3.14)),
            QueryValue::Float(3.14),
        );
    }

    #[test]
    fn test_json_string_to_query_value() {
        assert_eq!(
            ClickHouseAdapter::json_to_query_value(serde_json::Value::String("hello".to_string())),
            QueryValue::String("hello".to_string()),
        );
    }

    #[test]
    fn test_json_array_to_query_value() {
        assert_eq!(
            ClickHouseAdapter::json_to_query_value(serde_json::json!([1, "a", true])),
            QueryValue::String("[1,\"a\",true]".to_string()),
        );
    }

    #[test]
    fn test_json_object_to_query_value() {
        let obj = serde_json::json!({"key": "value"});
        let result = ClickHouseAdapter::json_to_query_value(obj);
        assert_eq!(
            result,
            QueryValue::String("{\"key\":\"value\"}".to_string())
        );
    }

    // ---- Disconnect ----

    #[test]
    fn test_disconnect_clears_state() {
        let config = ConnectionConfig::new(DatabaseType::ClickHouse, "localhost", 8123, "default");
        let mut adapter = ClickHouseAdapter::new(config);
        // Simulate connected state
        adapter.client = Some(reqwest::Client::new());
        adapter.pool = Some(Arc::new(ClickHousePool {
            client: reqwest::Client::new(),
            base_url: "http://localhost:8123".to_string(),
        }));

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            adapter.disconnect().await.unwrap();
        });

        assert!(adapter.client.is_none());
        assert!(adapter.pool.is_none());
    }
}
