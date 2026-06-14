//! JDBC bridge adapter — implements DatabaseAdapter by delegating to a Java subprocess.

use crate::database::{
    adapter::DatabaseAdapter,
    config::ConnectionConfig,
    error::{DbError, DbResult},
    types::{
        ColumnInfo, ConnectionStatus, DatabaseSchema, QueryResult, QueryRow, QueryValue, TableInfo,
    },
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::launcher::JdbcBridgeLauncher;
use super::pool::JdbcBridgePool;
use super::protocol::{
    ConnectParams, ConnectionStatusData, JdbcMethod, JdbcRequest,
    QueryResultData,
};

/// JDBC bridge adapter.
///
/// Spawns a Java subprocess (lazily) and communicates via JSON-RPC
/// over stdin/stdout. The Java side holds a HikariCP connection pool,
/// so connections are reused across queries.
pub struct JdbcBridgeAdapter {
    pub config: ConnectionConfig,
    launcher: Option<Arc<Mutex<JdbcBridgeLauncher>>>,
    conn_id: Option<String>,
}

impl JdbcBridgeAdapter {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            launcher: None,
            conn_id: None,
        }
    }

    /// Ensure all prerequisites are met: JRE, bridge JAR, and JDBC driver.
    /// Downloads anything missing automatically (called once from `connect`).
    async fn init_bridge(&mut self) -> DbResult<Arc<Mutex<JdbcBridgeLauncher>>> {
        let db_type = self.config.db_type;

        if !super::download::is_jre_installed() {
            super::download::download_jre().await?;
        }

        if !super::download::is_bridge_installed() {
            super::download::download_bridge_plugin().await?;
        }

        if !super::download::is_driver_available(db_type) {
            super::download::download_driver(db_type).await?;
        }

        let bridge_jar = super::download::bridge_jar_path();
        let mut launcher = JdbcBridgeLauncher::new(bridge_jar);
        launcher.start()?;
        let launcher = Arc::new(Mutex::new(launcher));

        let url = super::download::build_jdbc_url(
            db_type,
            &self.config.host,
            self.config.port,
            self.config.database.as_deref(),
        );
        let driver = super::download::driver_class(db_type);

        let result = Self::send_request(
            &launcher,
            JdbcRequest::new(
                JdbcMethod::Connect,
                serde_json::to_value(ConnectParams {
                    url,
                    username: self.config.username.clone(),
                    password: self.config.password.clone(),
                    database: self.config.database.clone(),
                    driver_class: driver.to_string(),
                    pool_min: 1,
                    pool_max: 5,
                })
                .unwrap_or_default(),
            ),
        )
        .await?;

        self.conn_id = result
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| Some(format!("conn_{}", uuid::Uuid::new_v4())));
        self.launcher = Some(launcher.clone());

        Ok(launcher)
    }

    /// Get the launcher (must be initialized first via `connect`).
    fn launcher(&self) -> DbResult<&Arc<Mutex<JdbcBridgeLauncher>>> {
        self.launcher
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))
    }

    async fn send_request(
        launcher: &Arc<Mutex<JdbcBridgeLauncher>>,
        req: JdbcRequest,
    ) -> DbResult<serde_json::Value> {
        let mut guard = launcher.lock().await;
        let resp = guard.send_request(&req)?;
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }

    fn parse_query_result(data: serde_json::Value) -> DbResult<QueryResult> {
        let qr: QueryResultData = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse query result: {}", e)))?;

        let rows: Vec<QueryRow> = qr
            .rows
            .into_iter()
            .map(|row| {
                let mut map: HashMap<String, QueryValue> = HashMap::new();
                for (i, val) in row.into_iter().enumerate() {
                    let col_name = qr.columns.get(i).cloned().unwrap_or_default();
                    let qv = match val {
                        serde_json::Value::Null => QueryValue::Null,
                        serde_json::Value::String(s) => QueryValue::String(s),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                QueryValue::Int(i)
                            } else {
                                QueryValue::String(n.to_string())
                            }
                        }
                        serde_json::Value::Bool(b) => QueryValue::String(b.to_string()),
                        _ => QueryValue::String(String::new()),
                    };
                    map.insert(col_name, qv);
                }
                map
            })
            .collect();

        Ok(QueryResult {
            columns: qr.columns,
            rows,
            rows_affected: qr.rows_affected,
            execution_time_ms: None,
        })
    }

    fn parse_connection_status(data: serde_json::Value) -> DbResult<ConnectionStatus> {
        let cs: ConnectionStatusData = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse status: {}", e)))?;
        Ok(cs.into())
    }
}

#[async_trait]
impl DatabaseAdapter for JdbcBridgeAdapter {
    type Pool = JdbcBridgePool;

    async fn connect(&mut self) -> DbResult<()> {
        self.init_bridge().await?;
        Ok(())

    }

    async fn disconnect(&mut self) -> DbResult<()> {
        if let Some(launcher) = &self.launcher {
            let mut guard = launcher.lock().await;
            guard.shutdown();
        }
        self.launcher = None;
        self.conn_id = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(JdbcMethod::TestConnection, serde_json::json!({
                "conn_id": self.conn_id,
            })),
        )
        .await?;
        Self::parse_connection_status(data)
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(JdbcMethod::ExecuteQuery, serde_json::json!({
                "conn_id": self.conn_id,
                "sql": query,
            })),
        )
        .await?;
        Self::parse_query_result(data)
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(JdbcMethod::ListDatabases, serde_json::json!({
                "conn_id": self.conn_id,
            })),
        )
        .await?;
        let names: Vec<String> = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse database list: {}", e)))?;
        Ok(names
            .into_iter()
            .map(|name| DatabaseSchema {
                name,
                description: None,
                is_system: false,
                metadata: HashMap::new(),
            })
            .collect())
    }

    async fn list_schemas(&self, database: Option<&str>) -> DbResult<Vec<String>> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(JdbcMethod::ListSchemas, serde_json::json!({
                "conn_id": self.conn_id,
                "database": database,
            })),
        )
        .await?;
        serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse schema list: {}", e)))
    }

    async fn list_tables(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(JdbcMethod::ListTables, serde_json::json!({
                "conn_id": self.conn_id,
                "database": database,
                "schema": schema,
            })),
        )
        .await?;
        let tables: Vec<serde_json::Value> = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse table list: {}", e)))?;
        Ok(tables
            .into_iter()
            .filter_map(|t| {
                Some(TableInfo {
                    schema: t.get("schema")?.as_str().map(|s| s.to_string()),
                    name: t.get("name")?.as_str()?.to_string(),
                    table_type: t
                        .get("table_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("TABLE")
                        .to_string(),
                    row_count: t.get("row_count").and_then(|v| v.as_u64()),
                    size_bytes: None,
                    description: None,
                    metadata: HashMap::new(),
                })
            })
            .collect())
    }

    async fn list_columns(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(JdbcMethod::ListColumns, serde_json::json!({
                "conn_id": self.conn_id,
                "database": database,
                "schema": schema,
                "table": table,
            })),
        )
        .await?;
        let cols: Vec<serde_json::Value> = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse column list: {}", e)))?;
        Ok(cols
            .into_iter()
            .filter_map(|c| {
                Some(ColumnInfo {
                    name: c.get("name")?.as_str()?.to_string(),
                    data_type: c
                        .get("data_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    nullable: c
                        .get("nullable")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    default_value: c
                        .get("default_value")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    is_primary_key: c
                        .get("is_primary_key")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    is_auto_increment: c
                        .get("is_auto_increment")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    max_length: c.get("max_length").and_then(|v| v.as_u64().map(|x| x as u32)),
                    precision: c.get("precision").and_then(|v| v.as_u64().map(|x| x as u32)),
                    scale: c.get("scale").and_then(|v| v.as_u64().map(|x| x as u32)),
                    description: None,
                    metadata: HashMap::new(),
                })
            })
            .collect())
    }

    async fn get_table_info(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        let tables = self.list_tables(database, schema).await?;
        tables
            .into_iter()
            .find(|t| t.name == table)
            .ok_or_else(|| DbError::Connection(format!("Table '{}' not found", table)))
    }

    fn get_pool(&self) -> Option<Arc<Self::Pool>> {
        None
    }

    fn get_config(&self) -> &ConnectionConfig {
        &self.config
    }
}
