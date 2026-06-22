//! JSON-RPC protocol types for the JDBC bridge.
//!
//! The bridge communicates with a Java subprocess over stdin/stdout
//! using newline-delimited JSON (one JSON object per line).

use crate::database::config::OracleConnectionOptions;
use serde::{Deserialize, Serialize};

/// Request methods the Rust side can invoke on the Java bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JdbcMethod {
    Connect,
    Disconnect,
    ExecuteQuery,
    ListDatabases,
    ListSchemas,
    ListTables,
    ListColumns,
    Ping,
    ResolveDriver,
    TestConnection,
}

/// A JSON-RPC request sent from Rust to Java.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdbcRequest {
    pub id: u64,
    pub method: JdbcMethod,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

impl JdbcRequest {
    pub fn new(method: JdbcMethod, params: serde_json::Value) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            method,
            params,
        }
    }

    pub fn ping() -> Self {
        Self::new(JdbcMethod::Ping, serde_json::Value::Null)
    }
}

/// A JSON-RPC response from Java to Rust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdbcResponse {
    pub id: u64,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_type: Option<String>,
}

// ── Connection parameters ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectParams {
    pub url: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    pub driver_class: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub driver_jars: Vec<String>,
    #[serde(default = "default_pool_min")]
    pub pool_min: u32,
    #[serde(default = "default_pool_max")]
    pub pool_max: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oracle_options: Option<OracleConnectionOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials_in_url: Option<bool>,
}

fn default_pool_min() -> u32 {
    1
}
fn default_pool_max() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteQueryParams {
    pub conn_id: String,
    pub sql: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListParams {
    pub conn_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveDriverParams {
    pub maven_group: String,
    pub maven_artifact: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_cap: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maven_classifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveDriverResult {
    pub jar_path: String,
    pub resolved_version: String,
}

// ── Response result types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    #[serde(default)]
    pub rows_affected: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatusData {
    pub is_connected: bool,
    pub server_version: Option<String>,
    pub current_database: Option<String>,
    pub current_user: Option<String>,
}

impl From<ConnectionStatusData> for crate::database::types::ConnectionStatus {
    fn from(d: ConnectionStatusData) -> Self {
        Self {
            is_connected: d.is_connected,
            server_version: d.server_version,
            current_database: d.current_database,
            current_user: d.current_user,
            metadata: std::collections::HashMap::new(),
        }
    }
}
