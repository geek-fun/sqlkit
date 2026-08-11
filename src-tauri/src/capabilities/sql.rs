use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::database::{DatabaseAdapter, QueryResult};
use crate::state::{ActiveConnection, ServerConfig};

use data_studio_agent::capabilities::registry::CapabilityRegistry;
use data_studio_agent::capabilities::types::{
    Capability, CapabilityHandler, RiskLevel, SourceKind,
};

fn app_handle() -> AppHandle {
    crate::APP_HANDLE
        .get()
        .expect("APP_HANDLE not initialized")
        .clone()
}

pub(crate) async fn resolve_adapter(connection_id: &str) -> Result<ActiveConnection, String> {
    let app = app_handle();

    // Check if already connected
    {
        let state: tauri::State<'_, crate::state::AppState> = app.state();
        let conns = state.connections.read().await;
        if let Some(adapter) = conns.get(connection_id) {
            return Ok(adapter.clone());
        }
    }

    // Auto-connect: look up credentials from the store
    let store = app
        .store(".store.dat")
        .map_err(|e| format!("Failed to open store: {}", e))?;
    let all_connections = store
        .get("connections")
        .and_then(|v| v.as_array().cloned())
        .ok_or_else(|| "No connections found in store".to_string())?;

    let conn_value = all_connections
        .into_iter()
        .find(|c| c.get("id").and_then(|v| v.as_str()) == Some(connection_id))
        .ok_or_else(|| {
            format!(
                "Connection '{}' not found in store. Connect manually first.",
                connection_id
            )
        })?;

    let server_config: ServerConfig = serde_json::from_value(conn_value)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    let adapter = crate::commands::helpers::create_and_connect_adapter(
        &server_config.db_type,
        server_config
            .to_connection_config()
            .map_err(|e| format!("Invalid connection config: {}", e))?,
    )
    .await?;

    // Store the adapter for future use
    {
        let state: tauri::State<'_, crate::state::AppState> = app.state();
        state.connections.write().await.insert(connection_id.to_string(), adapter.clone());
        state.configs.write().await.insert(connection_id.to_string(), server_config);
    }

    Ok(adapter)
}

pub(crate) async fn execute_on_adapter(adapter: &ActiveConnection, sql: &str) -> Result<QueryResult, String> {
    match adapter {
        ActiveConnection::Postgres(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
        ActiveConnection::MySQL(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
        ActiveConnection::SQLite(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
        ActiveConnection::SQLServer(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
        ActiveConnection::ClickHouse(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
        ActiveConnection::JdbcBridge(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
        ActiveConnection::HttpSql(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
        ActiveConnection::Rqlite(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
        ActiveConnection::Turso(a) => a
            .lock()
            .await
            .execute_query(sql)
            .await
            .map_err(|e| e.to_string()),
    }
}

pub(crate) fn get_connection_id(config: Option<&Value>) -> Result<String, String> {
    match config {
        None => Err(
            "No connection was provided for this tool call. Supply a connection_id \
             (list them with sqlkit__list_connections) or enable it in Settings → MCP Bridge"
                .to_string(),
        ),
        Some(c) => c
            .get("connectionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Connection config is missing the 'connectionId' field".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Handler structs
// ---------------------------------------------------------------------------

struct ExecuteQueryHandler;
struct ListDatabasesHandler;
struct ListSchemasHandler;
struct ListTablesHandler;
struct GetSchemaHandler;
struct DescribeTableHandler;
struct ExplainQueryHandler;
struct ListIndexesHandler;
struct ListForeignKeysHandler;
struct ListViewsHandler;
struct ListProceduresHandler;
struct ListFunctionsHandler;
struct ListTriggersHandler;
struct GetObjectDdlHandler;
struct GetTableInfoHandler;
struct GetForeignKeysHandler;

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

#[async_trait]
impl CapabilityHandler for ExecuteQueryHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let sql = args
            .get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'sql' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;

        // Read-only guard: reject write/delete/ddl statements with actionable
        // guidance so agents migrate to the split write tools.
        let db_type = crate::capabilities::sql_write::adapter_db_type(&adapter);
        crate::capabilities::sql_write::ensure_read_only(&db_type, sql)?;

        // Check connection quality and warn the AI agent about flaky connections
        let mut guardian_warning: Option<String> = None;
        if let Some(guardian) = crate::GUARDIAN.get() {
            if let Some(quality) = guardian.quality_score(&conn_id).await {
                if quality.score < 50.0 {
                    guardian_warning = Some(format!(
                        "Connection quality is low (score: {:.0}/100). \
                         Error count: {}, avg latency: {:.0}ms. \
                         The agent should be cautious about flaky connections.",
                        quality.score, quality.error_count, quality.avg_latency_ms
                    ));
                }
            }
        }

        let result = execute_on_adapter(&adapter, sql).await?;
        let mut response_map = serde_json::Map::new();
        let json_val = serde_json::to_value(&result).map_err(|e| e.to_string())?;
        response_map.insert("data".to_string(), json_val);
        if let Some(warning) = guardian_warning {
            response_map.insert(
                "guardian_warning".to_string(),
                serde_json::Value::String(warning),
            );
        }
        let output = serde_json::to_string(&response_map).map_err(|e| e.to_string())?;
        Ok(crate::common::format::truncate_tool_output(output))
    }
}

#[async_trait]
impl CapabilityHandler for ListDatabasesHandler {
    async fn handle(
        &self,
        _args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let adapter = resolve_adapter(&conn_id).await?;
        let dbs = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_databases()
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&dbs).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ListSchemasHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;
        let schemas = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_schemas(database)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_schemas(database)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(_) => vec![],
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_schemas(database)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_schemas(database)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_schemas(database)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_schemas(database)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_schemas(database)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_schemas(database)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&schemas).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ListTablesHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;
        let tables = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_tables(None, None)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&tables).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for GetSchemaHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;

        let tables = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_tables(None, None)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_tables(database, schema)
                .await
                .map_err(|e| e.to_string())?,
        };

        const MAX_SCHEMA_TABLES: usize = 30;
        let tables: Vec<_> = tables.into_iter().take(MAX_SCHEMA_TABLES).collect();

        let mut schema_lines: Vec<String> = Vec::new();
        if tables.len() >= MAX_SCHEMA_TABLES {
            schema_lines.push(format!(
                "-- Showing first {} tables. Specify a schema filter for complete results.\n",
                MAX_SCHEMA_TABLES
            ));
        }
        for table in &tables {
            let cols = match &adapter {
                ActiveConnection::Postgres(a) => a
                    .lock()
                    .await
                    .list_columns(database, schema, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
                ActiveConnection::MySQL(a) => a
                    .lock()
                    .await
                    .list_columns(database, schema, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
                ActiveConnection::SQLite(a) => a
                    .lock()
                    .await
                    .list_columns(None, None, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
                ActiveConnection::SQLServer(a) => a
                    .lock()
                    .await
                    .list_columns(database, schema, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
                ActiveConnection::ClickHouse(a) => a
                    .lock()
                    .await
                    .list_columns(database, schema, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
                ActiveConnection::JdbcBridge(a) => a
                    .lock()
                    .await
                    .list_columns(database, schema, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
                ActiveConnection::HttpSql(a) => a
                    .lock()
                    .await
                    .list_columns(database, schema, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
                ActiveConnection::Rqlite(a) => a
                    .lock()
                    .await
                    .list_columns(database, schema, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
                ActiveConnection::Turso(a) => a
                    .lock()
                    .await
                    .list_columns(database, schema, &table.name)
                    .await
                    .map_err(|e| e.to_string())?,
            };

            let schema_name = table.schema.as_deref().unwrap_or("public");
            let table_type = &table.table_type;
            schema_lines.push(format!(
                "-- {}.{} ({})",
                schema_name, table.name, table_type
            ));
            for col in &cols {
                let nullable = if col.nullable { "NULL" } else { "NOT NULL" };
                let pk = if col.is_primary_key {
                    " PRIMARY KEY"
                } else {
                    ""
                };
                let default = col
                    .default_value
                    .as_ref()
                    .map(|d| format!(" DEFAULT {}", d))
                    .unwrap_or_default();
                schema_lines.push(format!(
                    "  {} {} {}{}{}",
                    col.name, col.data_type, nullable, default, pk
                ));
            }
            schema_lines.push(String::new());
        }
        Ok(schema_lines.join("\n"))
    }
}

#[async_trait]
impl CapabilityHandler for DescribeTableHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let table = args
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'table' argument".to_string())?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;
        let cols = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_columns(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_columns(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_columns(None, None, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_columns(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_columns(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_columns(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_columns(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_columns(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_columns(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&cols).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ExplainQueryHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let sql = args
            .get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'sql' argument".to_string())?;
        let explain_sql = format!("EXPLAIN ANALYZE {}", sql);
        let adapter = resolve_adapter(&conn_id).await?;
        let result = execute_on_adapter(&adapter, &explain_sql).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ListIndexesHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let table = args
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'table' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;
        let indexes = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_indexes(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_indexes(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_indexes(None, None, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_indexes(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_indexes(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_indexes(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_indexes(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_indexes(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_indexes(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&indexes).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ListForeignKeysHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let table = args
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'table' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;
        let fks = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_foreign_keys(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_foreign_keys(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_foreign_keys(None, None, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_foreign_keys(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_foreign_keys(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_foreign_keys(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_foreign_keys(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_foreign_keys(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_foreign_keys(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&fks).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ListViewsHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;
        let views = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_views(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_views(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_views(None, None)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_views(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_views(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_views(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_views(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_views(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_views(database, schema)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&views).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ListProceduresHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;
        let procs = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_procedures(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_procedures(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_procedures(None, None)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_procedures(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_procedures(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_procedures(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_procedures(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_procedures(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_procedures(database, schema)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&procs).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ListFunctionsHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;
        let funcs = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_functions(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_functions(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_functions(None, None)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_functions(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_functions(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_functions(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_functions(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_functions(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_functions(database, schema)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&funcs).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for ListTriggersHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let table = args
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'table' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;
        let triggers = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .list_triggers(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .list_triggers(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .list_triggers(None, None, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .list_triggers(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .list_triggers(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .list_triggers(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .list_triggers(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .list_triggers(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .list_triggers(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&triggers).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for GetObjectDdlHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let object_name = args
            .get("object_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'object_name' argument".to_string())?;
        let object_type = args
            .get("object_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'object_type' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;
        let ddl = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .get_object_ddl(database, schema, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .get_object_ddl(database, schema, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .get_object_ddl(None, None, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .get_object_ddl(database, schema, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .get_object_ddl(database, schema, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .get_object_ddl(database, schema, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .get_object_ddl(database, schema, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .get_object_ddl(database, schema, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .get_object_ddl(database, schema, object_name, object_type)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&ddl).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for GetTableInfoHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let table = args
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'table' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;
        let info = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .get_table_info(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .get_table_info(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .get_table_info(None, None, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .get_table_info(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .get_table_info(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .get_table_info(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .get_table_info(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .get_table_info(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .get_table_info(database, schema, table)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&info).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for GetForeignKeysHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let schema = args.get("schema").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;
        let fks = match &adapter {
            ActiveConnection::Postgres(a) => a
                .lock()
                .await
                .get_foreign_keys(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::MySQL(a) => a
                .lock()
                .await
                .get_foreign_keys(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLite(a) => a
                .lock()
                .await
                .get_foreign_keys(None, None)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::SQLServer(a) => a
                .lock()
                .await
                .get_foreign_keys(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::ClickHouse(a) => a
                .lock()
                .await
                .get_foreign_keys(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::JdbcBridge(a) => a
                .lock()
                .await
                .get_foreign_keys(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::HttpSql(a) => a
                .lock()
                .await
                .get_foreign_keys(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Rqlite(a) => a
                .lock()
                .await
                .get_foreign_keys(database, schema)
                .await
                .map_err(|e| e.to_string())?,
            ActiveConnection::Turso(a) => a
                .lock()
                .await
                .get_foreign_keys(database, schema)
                .await
                .map_err(|e| e.to_string())?,
        };
        serde_json::to_string(&fks).map_err(|e| e.to_string())
    }
}

/// Register all SQL agent tools. These tools work across all SQL database types.
/// They use `SourceKind::SqlDatabase` so they match any SQL database type.
fn connection_id_schema() -> Value {
    json!({
        "type": "string",
        "description": "The connection alias to use (e.g. 'mac-postgresql'). Use sqlkit__list_connections to see available connections."
    })
}

pub fn register_sql_tools(reg: &mut CapabilityRegistry) {
    reg.register(Capability {
        name: "sqlkit__execute_query",
        description: "Execute a read-only SQL query (SELECT, SHOW, EXPLAIN) and return the result set with columns, types, rows, and execution time.\n\nUse when a task needs database data: row counts, table contents, aggregations, or data checks — instead of shelling out to psql/mysql CLI.\n\nExample: {\"sql\": \"SELECT COUNT(*) FROM users\"}.\n\nWrite statements (INSERT/UPDATE/MERGE), deletes (DELETE/TRUNCATE), and DDL (CREATE/ALTER/DROP) are rejected — use sqlkit__execute_write, sqlkit__execute_delete, or sqlkit__execute_ddl respectively.",
        handler: Arc::new(ExecuteQueryHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "sql": {"type": "string", "description": "The read-only SQL query (SELECT/SHOW/EXPLAIN)"}
        }, "required": ["connection_id", "sql"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_databases",
        description: "List all databases on the connected server.",
        handler: Arc::new(ListDatabasesHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema()
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_schemas",
        description: "List all schemas in a database.",
        handler: Arc::new(ListSchemasHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_tables",
        description: "List all tables in a database schema. Returns table names, types, and row counts — fast and lightweight. Use this to check if tables exist or browse available objects. For full column details, use sqlkit__describe_table or sqlkit__get_schema.",
        handler: Arc::new(ListTablesHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name"},
            "schema": {"type": "string", "description": "Schema name (optional)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe, required_permission: "read",
        source_kind: SourceKind::SqlDatabase, tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__get_schema",
        description: "Get the full database schema (all tables and all columns) as DDL-like text. SLOW on databases with many objects. Prefer sqlkit__list_tables for browsing and sqlkit__describe_table for single-table details.",
        handler: Arc::new(GetSchemaHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"},
            "schema": {"type": "string", "description": "Schema name (optional)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe, required_permission: "read",
        source_kind: SourceKind::SqlDatabase, tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__describe_table",
        description: "Get detailed column info for a table including types, nullability, defaults, and keys.",
        handler: Arc::new(DescribeTableHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "table": {"type": "string", "description": "Table name"},
            "database": {"type": "string"},
            "schema": {"type": "string"}
        }, "required": ["connection_id", "table"]}),
        risk_level: RiskLevel::Safe, required_permission: "read",
        source_kind: SourceKind::SqlDatabase, tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__explain_query",
        description:
            "Get the query execution plan for a SQL statement. Useful for optimizing slow queries.",
        handler: Arc::new(ExplainQueryHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "sql": {"type": "string", "description": "The SQL query to explain"}
        }, "required": ["connection_id", "sql"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_indexes",
        description: "List all indexes for a specific table.\n\nUse when you need to inspect a table's indexing strategy — check which columns are indexed, index types (BTREE, HASH, etc.), and whether constraints enforce uniqueness.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\", \"table\": \"users\"}.",
        handler: Arc::new(ListIndexesHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name"},
            "schema": {"type": "string", "description": "Schema name (optional)"},
            "table": {"type": "string", "description": "Table name"}
        }, "required": ["connection_id", "table"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_foreign_keys",
        description: "List all foreign key constraints for a specific table.\n\nUse when you need to trace foreign key relationships for a single table — referenced tables, columns, and ON DELETE/ON UPDATE actions.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\", \"table\": \"orders\"}.",
        handler: Arc::new(ListForeignKeysHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name"},
            "schema": {"type": "string", "description": "Schema name (optional)"},
            "table": {"type": "string", "description": "Table name"}
        }, "required": ["connection_id", "table"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_views",
        description: "List all views in a database schema.\n\nUse when you need to discover available views — their names, types, and detail information like column lists or definition summaries.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\"}.",
        handler: Arc::new(ListViewsHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"},
            "schema": {"type": "string", "description": "Schema name (optional)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_procedures",
        description: "List all stored procedures in a database schema.\n\nUse when you need to discover available stored procedures — their names, types, and detail information like parameter lists.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\"}.",
        handler: Arc::new(ListProceduresHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"},
            "schema": {"type": "string", "description": "Schema name (optional)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_functions",
        description: "List all functions in a database schema.\n\nUse when you need to discover available functions — their names, types, and detail information like return types and parameter signatures.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\"}.",
        handler: Arc::new(ListFunctionsHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"},
            "schema": {"type": "string", "description": "Schema name (optional)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_triggers",
        description: "List all triggers for a specific table.\n\nUse when you need to inspect a table's triggers — their names, action timings (BEFORE/AFTER/INSTEAD OF), triggering events (INSERT/UPDATE/DELETE), and DDL source.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\", \"table\": \"users\"}.",
        handler: Arc::new(ListTriggersHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name"},
            "schema": {"type": "string", "description": "Schema name (optional)"},
            "table": {"type": "string", "description": "Table name"}
        }, "required": ["connection_id", "table"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__get_object_ddl",
        description: "Get the DDL source for a database object (table, view, procedure, function, trigger, index).\n\nUse when you need to see the exact CREATE statement for an object — useful for schema reviews, migration scripts, or debugging object definitions.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\", \"object_name\": \"users\", \"object_type\": \"table\"}.",
        handler: Arc::new(GetObjectDdlHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name"},
            "schema": {"type": "string", "description": "Schema name (optional)"},
            "object_name": {"type": "string", "description": "Object name"},
            "object_type": {"type": "string", "description": "Object type (table, view, procedure, function, trigger, index)"}
        }, "required": ["connection_id", "object_name", "object_type"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__get_table_info",
        description: "Get detailed metadata for a specific table.\n\nUse when you need table-level info — schema, table type, row count estimate, size in bytes, and description. For column details, use sqlkit__describe_table instead.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\", \"table\": \"users\"}.",
        handler: Arc::new(GetTableInfoHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"},
            "schema": {"type": "string", "description": "Schema name (optional)"},
            "table": {"type": "string", "description": "Table name"}
        }, "required": ["connection_id", "table"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__get_foreign_keys",
        description: "Get all foreign key relationships for tables in a schema.\n\nUse when you need a schema-level overview of foreign key constraints — constraint names, source/target tables and columns, and ON DELETE/ON UPDATE actions. For a single-table view, use sqlkit__list_foreign_keys instead.\n\nExample: {\"database\": \"mydb\", \"schema\": \"public\"}.",
        handler: Arc::new(GetForeignKeysHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"},
            "schema": {"type": "string", "description": "Schema name (optional)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_connection_id_returns_id_when_present() {
        let config = json!({ "connectionId": "conn-1" });
        assert_eq!(get_connection_id(Some(&config)), Ok("conn-1".to_string()));
    }

    #[test]
    fn get_connection_id_explains_missing_config() {
        let err = get_connection_id(None).unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[test]
    fn get_connection_id_rejects_config_without_field() {
        let err = get_connection_id(Some(&json!({ "host": "x" }))).unwrap_err();
        assert!(err.contains("connectionId"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_indexes_missing_config() {
        let err = ListIndexesHandler
            .handle(&json!({ "table": "users" }), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_indexes_rejects_missing_table() {
        let config = json!({ "connectionId": "conn-1" });
        let err = ListIndexesHandler
            .handle(&json!({ "database": "db" }), Some(&config))
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'table'"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_foreign_keys_missing_config() {
        let err = ListForeignKeysHandler
            .handle(&json!({ "table": "orders" }), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_foreign_keys_rejects_missing_table() {
        let config = json!({ "connectionId": "conn-1" });
        let err = ListForeignKeysHandler
            .handle(&json!({ "database": "db" }), Some(&config))
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'table'"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_views_missing_config() {
        let err = ListViewsHandler.handle(&json!({}), None).await.unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_procedures_missing_config() {
        let err = ListProceduresHandler
            .handle(&json!({}), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_functions_missing_config() {
        let err = ListFunctionsHandler
            .handle(&json!({}), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_triggers_missing_config() {
        let err = ListTriggersHandler
            .handle(&json!({ "table": "users" }), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn list_triggers_rejects_missing_table() {
        let config = json!({ "connectionId": "conn-1" });
        let err = ListTriggersHandler
            .handle(&json!({ "database": "db" }), Some(&config))
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'table'"), "got: {}", err);
    }

    #[tokio::test]
    async fn get_object_ddl_missing_config() {
        let err = GetObjectDdlHandler
            .handle(
                &json!({ "object_name": "users", "object_type": "table" }),
                None,
            )
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn get_object_ddl_rejects_missing_object_name() {
        let config = json!({ "connectionId": "conn-1" });
        let err = GetObjectDdlHandler
            .handle(&json!({ "object_type": "table" }), Some(&config))
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'object_name'"), "got: {}", err);
    }

    #[tokio::test]
    async fn get_object_ddl_rejects_missing_object_type() {
        let config = json!({ "connectionId": "conn-1" });
        let err = GetObjectDdlHandler
            .handle(&json!({ "object_name": "users" }), Some(&config))
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'object_type'"), "got: {}", err);
    }

    #[tokio::test]
    async fn get_table_info_missing_config() {
        let err = GetTableInfoHandler
            .handle(&json!({ "table": "users" }), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn get_table_info_rejects_missing_table() {
        let config = json!({ "connectionId": "conn-1" });
        let err = GetTableInfoHandler
            .handle(&json!({ "database": "db" }), Some(&config))
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'table'"), "got: {}", err);
    }

    #[tokio::test]
    async fn get_foreign_keys_missing_config() {
        let err = GetForeignKeysHandler
            .handle(&json!({}), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }
}
