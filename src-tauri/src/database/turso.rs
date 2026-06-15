//! Turso (libsql) HTTP database adapter implementation.
//!
//! This module provides a concrete implementation of the `DatabaseAdapter` trait
//! for Turso/libsql using the HTTP protocol via `reqwest`. It communicates with
//! the Turso HTTP API via the `/v2/pipeline` endpoint, using Bearer token
//! authentication.
//!
//! Turso uses libsql, which is SQLite-compatible. The pipeline API is stateless
//! and batched — each request can contain multiple statements.

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
// Turso pipeline API response structures
// ---------------------------------------------------------------------------

/// A single value cell in a Turso response row.
#[derive(Debug, Deserialize)]
struct TursoValue {
    /// The type discriminator: "null", "integer", "float", "text", "blob".
    #[serde(rename = "type")]
    value_type: String,
    /// The string-encoded value (absent for nulls).
    #[serde(default)]
    value: Option<String>,
}

/// Column metadata from a Turso response.
#[derive(Debug, Deserialize)]
struct TursoColumn {
    name: String,
}

/// Result set (columns + rows) returned by a successful execute response.
#[derive(Debug, Deserialize)]
struct TursoResults {
    cols: Vec<TursoColumn>,
    rows: Vec<Vec<TursoValue>>,
}

/// Error detail returned by Turso.
#[derive(Debug, Deserialize)]
struct TursoError {
    message: String,
}

/// The `response` field inside each pipeline result entry.
#[derive(Debug, Deserialize)]
struct TursoResponse {
    /// "results" for queries, "error" for failures.
    #[serde(rename = "type")]
    response_type: String,
    #[serde(default)]
    results: Option<TursoResults>,
    #[serde(default)]
    error: Option<TursoError>,
    /// Present for DML statements (INSERT/UPDATE/DELETE).
    #[serde(default)]
    affected_row_count: Option<u64>,
}

/// A single entry in the pipeline `results` array.
#[derive(Debug, Deserialize)]
struct TursoResultEntry {
    /// Always "execute" for statement results.
    #[serde(rename = "type")]
    entry_type: String,
    response: TursoResponse,
}

/// Top-level response from the Turso `/v2/pipeline` endpoint.
#[derive(Debug, Deserialize)]
struct TursoPipelineResponse {
    results: Vec<TursoResultEntry>,
}

// ---------------------------------------------------------------------------
// TursoPool — stateless HTTP "pool" wrapping a shared reqwest::Client
// ---------------------------------------------------------------------------

/// A Turso connection pool.
///
/// Because the Turso HTTP API is stateless, the "pool" does not maintain
/// persistent connections. It holds a shared `reqwest::Client` (which internally
/// reuses HTTP connections via connection pooling) and the base URL for the
/// target server.
pub struct TursoPool {
    client: reqwest::Client,
    #[allow(dead_code)]
    base_url: String,
}

#[async_trait]
impl ConnectionPool for TursoPool {
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
// TursoAdapter
// ---------------------------------------------------------------------------

/// Turso/libsql database adapter using the HTTP protocol.
///
/// Sends SQL queries to Turso's `/v2/pipeline` HTTP endpoint and parses the
/// structured JSON response. Supports Bearer token authentication via the
/// `Authorization` header.
pub struct TursoAdapter {
    pub config: ConnectionConfig,
    client: Option<reqwest::Client>,
    pool: Option<Arc<TursoPool>>,
}

impl TursoAdapter {
    /// Create a new `TursoAdapter` from the supplied configuration.
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

    /// Build the base URL (`https://host[:port]`) from the configuration.
    ///
    /// The `host` field in the config should be the full Turso hostname
    /// (e.g., `my-db-org.turso.io`). The default port is 443 and is omitted
    /// unless a non-default port is explicitly set.
    fn build_base_url(&self) -> String {
        if self.config.port != 0 && self.config.port != 443 {
            format!("https://{}:{}", self.config.host, self.config.port)
        } else {
            format!("https://{}", self.config.host)
        }
    }

    /// Create the `reqwest::Client` used for all HTTP calls.
    fn build_client() -> DbResult<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("sqlkit-turso-adapter/0.1")
            .build()
            .map_err(|e| DbError::Connection(format!("Failed to create HTTP client: {}", e)))
    }

    /// Build the HTTP headers for a request.
    ///
    /// Adds `Content-Type: application/json` and, when a password (token) is
    /// configured, a `Bearer` authorization header.
    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(ref token) = self.config.password {
            if let Ok(auth_value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                headers.insert(AUTHORIZATION, auth_value);
            }
        }

        headers
    }

    /// Build the JSON body for a pipeline request.
    fn build_pipeline_body(sql: &str) -> serde_json::Value {
        serde_json::json!({
            "requests": [
                {"type": "execute", "stmt": {"sql": sql}},
                {"type": "close"}
            ]
        })
    }

    /// Send a SQL query through the Turso pipeline API and return the parsed
    /// pipeline response.
    async fn send_pipeline(&self, sql: &str) -> DbResult<TursoPipelineResponse> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected to Turso".to_string()))?;

        let url = format!("{}/v2/pipeline", self.build_base_url());
        let headers = self.build_headers();
        let body = Self::build_pipeline_body(sql);

        let response = client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DbError::Timeout(format!("Turso query timed out: {}", e))
                } else if e.is_connect() {
                    DbError::Connection(format!(
                        "Cannot connect to Turso at {}: {}",
                        self.build_base_url(),
                        e
                    ))
                } else {
                    DbError::QueryExecution(format!("HTTP request failed: {}", e))
                }
            })?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| DbError::QueryExecution(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            return Err(DbError::QueryExecution(format!(
                "Turso error (HTTP {}): {}",
                status.as_u16(),
                body_text.trim()
            )));
        }

        serde_json::from_str::<TursoPipelineResponse>(&body_text).map_err(|e| {
            DbError::Serialization(format!(
                "Failed to parse Turso JSON response: {} (body preview: {})",
                e,
                body_text.chars().take(200).collect::<String>()
            ))
        })
    }

    /// Convert a `TursoValue` (type + optional value string) into a `QueryValue`.
    fn turso_value_to_query_value(value: TursoValue) -> QueryValue {
        match value.value_type.as_str() {
            "null" => QueryValue::Null,
            "integer" => {
                if let Some(v) = value.value.as_deref() {
                    v.parse::<i64>()
                        .map(QueryValue::Int)
                        .unwrap_or_else(|_| QueryValue::String(v.to_string()))
                } else {
                    QueryValue::Null
                }
            }
            "float" => {
                if let Some(v) = value.value.as_deref() {
                    v.parse::<f64>()
                        .map(QueryValue::Float)
                        .unwrap_or_else(|_| QueryValue::String(v.to_string()))
                } else {
                    QueryValue::Null
                }
            }
            "text" => QueryValue::String(value.value.unwrap_or_default()),
            "blob" => {
                if let Some(v) = value.value.as_deref() {
                    let engine = base64::engine::general_purpose::STANDARD;
                    engine.decode(v).map(QueryValue::Bytes).unwrap_or_else(|_| {
                        QueryValue::String(format!("<blob: base64 length {}>", v.len()))
                    })
                } else {
                    QueryValue::Bytes(Vec::new())
                }
            }
            // Fallback for unknown types
            _ => QueryValue::String(value.value.unwrap_or_default()),
        }
    }

    /// Extract the first `TursoResults` from a pipeline response.
    ///
    /// Returns `None` if the response indicates an error or has no result set.
    fn extract_results(response: &TursoPipelineResponse) -> DbResult<Option<&TursoResults>> {
        for entry in &response.results {
            if entry.response.response_type == "error" {
                let msg = entry
                    .response
                    .error
                    .as_ref()
                    .map(|e| e.message.as_str())
                    .unwrap_or("unknown error");
                return Err(DbError::QueryExecution(format!(
                    "Turso query error: {}",
                    msg
                )));
            }
            if entry.response.response_type == "results" {
                return Ok(entry.response.results.as_ref());
            }
        }
        Ok(None)
    }

    /// Extract the affected row count from a pipeline response (for DML).
    fn extract_affected_row_count(response: &TursoPipelineResponse) -> Option<u64> {
        for entry in &response.results {
            if let Some(count) = entry.response.affected_row_count {
                if count > 0 {
                    return Some(count);
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// DatabaseAdapter trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl DatabaseAdapter for TursoAdapter {
    type Pool = TursoPool;

    async fn connect(&mut self) -> DbResult<()> {
        let client = Self::build_client()?;

        // Verify connectivity by sending a simple "SELECT 1" pipeline
        let url = format!("{}/v2/pipeline", self.build_base_url());
        let headers = self.build_headers();
        let body = Self::build_pipeline_body("SELECT 1");

        let response = client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    DbError::Connection(format!(
                        "Cannot connect to Turso at {}: {}",
                        self.build_base_url(),
                        e
                    ))
                } else {
                    DbError::Connection(format!("Turso connection test failed: {}", e))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(DbError::Connection(format!(
                "Turso rejected connection (HTTP {}): {}",
                status.as_u16(),
                body.trim()
            )));
        }

        let base_url = self.build_base_url();
        self.client = Some(client.clone());
        self.pool = Some(Arc::new(TursoPool { client, base_url }));

        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.client = None;
        self.pool = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let pipeline_resp = self.send_pipeline("SELECT 1 AS one").await?;

        // Verify we got a valid response
        Self::extract_results(&pipeline_resp)?;

        let current_database = self.config.database.clone();

        Ok(ConnectionStatus {
            is_connected: true,
            server_version: None, // Turso HTTP API does not expose version easily
            current_database,
            current_user: Some(self.config.username.clone()),
            metadata: HashMap::new(),
        })
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let start = Instant::now();
        let pipeline_resp = self.send_pipeline(query).await?;
        let execution_time = start.elapsed().as_millis() as u64;

        // Check for errors first
        let results_opt = Self::extract_results(&pipeline_resp)?;

        let mut result = if let Some(results) = results_opt {
            let columns: Vec<String> = results.cols.iter().map(|col| col.name.clone()).collect();

            let rows: Vec<QueryRow> = results
                .rows
                .iter()
                .map(|row| {
                    let mut map = HashMap::new();
                    for (i, val) in row.iter().enumerate() {
                        let col_name = columns
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| format!("col_{}", i));
                        map.insert(col_name, Self::turso_value_to_query_value(val.clone()));
                    }
                    map
                })
                .collect();

            let mut r = QueryResult::new(columns);
            for row in rows {
                r.add_row(row);
            }
            r
        } else {
            // DML statement — check for affected row count
            let affected = Self::extract_affected_row_count(&pipeline_resp).unwrap_or(0);
            QueryResult::affected(affected)
        };

        result.execution_time_ms = Some(execution_time);
        Ok(result)
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        // Turso is a single-database service; return the configured database
        let name = self
            .config
            .database
            .clone()
            .unwrap_or_else(|| "main".to_string());

        Ok(vec![DatabaseSchema {
            name,
            description: None,
            is_system: false,
            metadata: HashMap::new(),
        }])
    }

    async fn list_schemas(&self, _database: Option<&str>) -> DbResult<Vec<String>> {
        // SQLite/Turso has no schema namespace; return "main" as the default
        Ok(vec!["main".to_string()])
    }

    async fn list_tables(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let response = self
            .send_pipeline("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .await?;
        let results = Self::extract_results(&response)?
            .ok_or_else(|| DbError::QueryExecution("No results from table list".to_string()))?;

        let tables = results
            .rows
            .iter()
            .filter_map(|row| {
                row.first().map(|v| {
                    let name = match &v.value {
                        Some(n) => n.clone(),
                        None => return None,
                    };

                    Some(TableInfo {
                        schema: Some("main".to_string()),
                        name,
                        table_type: "TABLE".to_string(),
                        row_count: None,
                        size_bytes: None,
                        description: None,
                        metadata: HashMap::new(),
                    })
                })?
            })
            .collect::<Vec<_>>();

        Ok(tables)
    }

    async fn list_columns(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        let safe_table = table.replace('\'', "''");
        let query = format!("PRAGMA table_info('{}')", safe_table);
        let response = self.send_pipeline(&query).await?;
        let results = Self::extract_results(&response)?
            .ok_or_else(|| DbError::TableNotFound(format!("Table '{}' not found", table)))?;

        let columns = results
            .rows
            .iter()
            .filter_map(|row| {
                // PRAGMA table_info returns: cid, name, type, notnull, dflt_value, pk
                if row.len() < 6 {
                    return None;
                }

                let name = row[1].value.clone().unwrap_or_default();
                let data_type = row[2].value.clone().unwrap_or_else(|| "TEXT".to_string());
                let notnull = row[3]
                    .value
                    .as_deref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                let default_value = row[4].value.clone();
                let is_pk = row[5]
                    .value
                    .as_deref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
                    != 0;

                Some(ColumnInfo {
                    name,
                    data_type,
                    nullable: notnull == 0,
                    default_value,
                    is_primary_key: is_pk,
                    is_auto_increment: false, // SQLite rowid is implicit
                    max_length: None,
                    precision: None,
                    scale: None,
                    description: None,
                    metadata: HashMap::new(),
                })
            })
            .collect();

        Ok(columns)
    }

    async fn get_table_info(
        &self,
        _database: Option<&str>,
        _schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        let escaped_table_dq = table.replace('"', "\"\"");
        let row_count_query = format!("SELECT COUNT(*) AS cnt FROM \"{}\"", escaped_table_dq);
        let row_count_resp = self.send_pipeline(&row_count_query).await?;

        let row_count = Self::extract_results(&row_count_resp)?.and_then(|r| {
            r.rows.first().and_then(|row| {
                row.first()
                    .and_then(|v| v.value.as_deref().and_then(|s| s.parse::<u64>().ok()))
            })
        });

        Ok(TableInfo {
            schema: Some("main".to_string()),
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
        let config = ConnectionConfig::new(DatabaseType::Turso, "my-db-org.turso.io", 443, "token");
        let adapter = TursoAdapter::new(config);
        assert!(adapter.client.is_none());
        assert!(adapter.pool.is_none());
    }

    #[test]
    fn test_get_config() {
        let config = ConnectionConfig::new(DatabaseType::Turso, "db-org.turso.io", 443, "my-token")
            .with_database("my-db");
        let adapter = TursoAdapter::new(config.clone());
        let cfg = adapter.get_config();
        assert_eq!(cfg.host, "db-org.turso.io");
        assert_eq!(cfg.port, 443);
        assert_eq!(cfg.username, "my-token");
        assert_eq!(cfg.database.as_deref(), Some("my-db"));
    }

    #[test]
    fn test_get_pool_initially_none() {
        let config = ConnectionConfig::new(DatabaseType::Turso, "my-db-org.turso.io", 443, "token");
        let adapter = TursoAdapter::new(config);
        assert!(adapter.get_pool().is_none());
    }

    // ---- URL building ----

    #[test]
    fn test_build_base_url_default_port() {
        let config = ConnectionConfig::new(DatabaseType::Turso, "my-db-org.turso.io", 443, "token");
        let adapter = TursoAdapter::new(config);
        assert_eq!(adapter.build_base_url(), "https://my-db-org.turso.io");
    }

    #[test]
    fn test_build_base_url_custom_port() {
        let config =
            ConnectionConfig::new(DatabaseType::Turso, "my-db-org.turso.io", 8080, "token");
        let adapter = TursoAdapter::new(config);
        assert_eq!(adapter.build_base_url(), "https://my-db-org.turso.io:8080");
    }

    // ---- Headers ----

    #[test]
    fn test_build_headers_with_token() {
        let config = ConnectionConfig::new(DatabaseType::Turso, "my-db-org.turso.io", 443, "user")
            .with_password("my-token");
        let adapter = TursoAdapter::new(config);
        let headers = adapter.build_headers();
        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
        assert_eq!(
            headers.get(AUTHORIZATION).unwrap().to_str().unwrap(),
            "Bearer my-token"
        );
    }

    #[test]
    fn test_build_headers_without_token() {
        let config = ConnectionConfig::new(DatabaseType::Turso, "my-db-org.turso.io", 443, "user");
        let adapter = TursoAdapter::new(config);
        let headers = adapter.build_headers();
        assert!(!headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
    }

    // ---- Pipeline body ----

    #[test]
    fn test_build_pipeline_body() {
        let body = TursoAdapter::build_pipeline_body("SELECT 1");
        let expected = serde_json::json!({
            "requests": [
                {"type": "execute", "stmt": {"sql": "SELECT 1"}},
                {"type": "close"}
            ]
        });
        assert_eq!(body, expected);
    }

    #[test]
    fn test_build_pipeline_body_with_quotes() {
        let body = TursoAdapter::build_pipeline_body("SELECT * FROM \"users\" WHERE id = 1");
        let expected = serde_json::json!({
            "requests": [
                {"type": "execute", "stmt": {"sql": "SELECT * FROM \"users\" WHERE id = 1"}},
                {"type": "close"}
            ]
        });
        assert_eq!(body, expected);
    }

    // ---- QueryValue conversion ----

    #[test]
    fn test_turso_null_to_query_value() {
        let v = TursoValue {
            value_type: "null".to_string(),
            value: None,
        };
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(v),
            QueryValue::Null,
        );
    }

    #[test]
    fn test_turso_integer_to_query_value() {
        let v = TursoValue {
            value_type: "integer".to_string(),
            value: Some("42".to_string()),
        };
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(v),
            QueryValue::Int(42),
        );
    }

    #[test]
    fn test_turso_negative_integer_to_query_value() {
        let v = TursoValue {
            value_type: "integer".to_string(),
            value: Some("-7".to_string()),
        };
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(v),
            QueryValue::Int(-7),
        );
    }

    #[test]
    fn test_turso_float_to_query_value() {
        let v = TursoValue {
            value_type: "float".to_string(),
            value: Some("3.14".to_string()),
        };
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(v),
            QueryValue::Float(3.14),
        );
    }

    #[test]
    fn test_turso_text_to_query_value() {
        let v = TursoValue {
            value_type: "text".to_string(),
            value: Some("hello".to_string()),
        };
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(v),
            QueryValue::String("hello".to_string()),
        );
    }

    #[test]
    fn test_turso_blob_to_query_value() {
        let data = b"hello";
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let v = TursoValue {
            value_type: "blob".to_string(),
            value: Some(encoded),
        };
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(v),
            QueryValue::Bytes(b"hello".to_vec()),
        );
    }

    #[test]
    fn test_turso_null_without_value_integer() {
        // Integer with no value field should be treated as Null
        let v = TursoValue {
            value_type: "integer".to_string(),
            value: None,
        };
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(v),
            QueryValue::Null,
        );
    }

    // ---- Disconnect ----

    #[test]
    fn test_disconnect_clears_state() {
        let config = ConnectionConfig::new(DatabaseType::Turso, "my-db-org.turso.io", 443, "token");
        let mut adapter = TursoAdapter::new(config);
        // Simulate connected state
        adapter.client = Some(reqwest::Client::new());
        adapter.pool = Some(Arc::new(TursoPool {
            client: reqwest::Client::new(),
            base_url: "https://my-db-org.turso.io".to_string(),
        }));

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            adapter.disconnect().await.unwrap();
        });

        assert!(adapter.client.is_none());
        assert!(adapter.pool.is_none());
    }

    // ---- Pipeline response parsing ----

    #[test]
    fn test_parse_select_response() {
        let json = serde_json::json!({
            "results": [
                {
                    "type": "execute",
                    "response": {
                        "type": "results",
                        "results": {
                            "cols": [{"name": "id"}, {"name": "name"}],
                            "rows": [
                                [{"type": "integer", "value": "1"}, {"type": "text", "value": "alice"}],
                                [{"type": "integer", "value": "2"}, {"type": "text", "value": "bob"}]
                            ]
                        }
                    }
                }
            ]
        });

        let response: TursoPipelineResponse = serde_json::from_value(json).unwrap();
        let results = TursoAdapter::extract_results(&response)
            .unwrap()
            .expect("should have results");

        assert_eq!(results.cols.len(), 2);
        assert_eq!(results.cols[0].name, "id");
        assert_eq!(results.cols[1].name, "name");
        assert_eq!(results.rows.len(), 2);

        // Check first row values via conversion
        let row0 = &results.rows[0];
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(row0[0].clone()),
            QueryValue::Int(1)
        );
        assert_eq!(
            TursoAdapter::turso_value_to_query_value(row0[1].clone()),
            QueryValue::String("alice".to_string())
        );
    }

    #[test]
    fn test_parse_error_response() {
        let json = serde_json::json!({
            "results": [
                {
                    "type": "execute",
                    "response": {
                        "type": "error",
                        "error": {"message": "no such table: missing"}
                    }
                }
            ]
        });

        let response: TursoPipelineResponse = serde_json::from_value(json).unwrap();
        let result = TursoAdapter::extract_results(&response);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no such table: missing"));
    }

    #[test]
    fn test_extract_affected_row_count() {
        let json = serde_json::json!({
            "results": [
                {
                    "type": "execute",
                    "response": {
                        "type": "results",
                        "results": {
                            "cols": [],
                            "rows": []
                        },
                        "affected_row_count": 3
                    }
                }
            ]
        });

        let response: TursoPipelineResponse = serde_json::from_value(json).unwrap();
        let count = TursoAdapter::extract_affected_row_count(&response);
        assert_eq!(count, Some(3));
    }

    #[test]
    fn test_extract_affected_row_count_none() {
        let json = serde_json::json!({
            "results": [
                {
                    "type": "execute",
                    "response": {
                        "type": "results",
                        "results": {
                            "cols": [{"name": "one"}],
                            "rows": [[{"type": "integer", "value": "1"}]]
                        }
                    }
                }
            ]
        });

        let response: TursoPipelineResponse = serde_json::from_value(json).unwrap();
        let count = TursoAdapter::extract_affected_row_count(&response);
        assert_eq!(count, None);
    }

    // ---- PRAGMA table_info parsing ----

    #[test]
    fn test_pragma_parsing() {
        // Simulate PRAGMA table_info('users') response
        let rows = vec![
            vec![
                TursoValue {
                    value_type: "integer".to_string(),
                    value: Some("0".to_string()),
                },
                TursoValue {
                    value_type: "text".to_string(),
                    value: Some("id".to_string()),
                },
                TursoValue {
                    value_type: "text".to_string(),
                    value: Some("INTEGER".to_string()),
                },
                TursoValue {
                    value_type: "integer".to_string(),
                    value: Some("1".to_string()),
                },
                TursoValue {
                    value_type: "text".to_string(),
                    value: None,
                },
                TursoValue {
                    value_type: "integer".to_string(),
                    value: Some("1".to_string()),
                },
            ],
            vec![
                TursoValue {
                    value_type: "integer".to_string(),
                    value: Some("1".to_string()),
                },
                TursoValue {
                    value_type: "text".to_string(),
                    value: Some("name".to_string()),
                },
                TursoValue {
                    value_type: "text".to_string(),
                    value: Some("TEXT".to_string()),
                },
                TursoValue {
                    value_type: "integer".to_string(),
                    value: Some("0".to_string()),
                },
                TursoValue {
                    value_type: "text".to_string(),
                    value: None,
                },
                TursoValue {
                    value_type: "integer".to_string(),
                    value: Some("0".to_string()),
                },
            ],
        ];

        // Manually convert to ColumnInfo list
        let columns: Vec<ColumnInfo> = rows
            .iter()
            .filter_map(|row| {
                if row.len() < 6 {
                    return None;
                }
                let name = row[1].value.clone().unwrap_or_default();
                let data_type = row[2].value.clone().unwrap_or_else(|| "TEXT".to_string());
                let notnull = row[3]
                    .value
                    .as_deref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                let default_value = row[4].value.clone();
                let is_pk = row[5]
                    .value
                    .as_deref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
                    != 0;

                Some(ColumnInfo {
                    name,
                    data_type,
                    nullable: notnull == 0,
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

        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[0].data_type, "INTEGER");
        assert_eq!(columns[0].nullable, false);
        assert_eq!(columns[0].is_primary_key, true);
        assert_eq!(columns[1].name, "name");
        assert_eq!(columns[1].data_type, "TEXT");
        assert_eq!(columns[1].nullable, true);
        assert_eq!(columns[1].is_primary_key, false);
    }
}
