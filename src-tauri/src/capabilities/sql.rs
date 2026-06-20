use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::database::{DatabaseAdapter, QueryResult};
use crate::state::{ActiveConnection, ServerConfig};

use super::registry::CapabilityRegistry;
use super::types::{Capability, CapabilityHandler, RiskLevel, SourceKind};

fn app_handle() -> AppHandle {
    crate::APP_HANDLE
        .get()
        .expect("APP_HANDLE not initialized")
        .clone()
}

async fn resolve_adapter(connection_id: &str) -> Result<ActiveConnection, String> {
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
        .find(|c| {
            c.get("id")
                .and_then(|v| v.as_str())
                == Some(connection_id)
        })
        .ok_or_else(|| {
            format!("Connection '{}' not found in store. Connect manually first.", connection_id)
        })?;

    let server_config: ServerConfig =
        serde_json::from_value(conn_value)
            .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    let adapter = crate::commands::helpers::create_and_connect_adapter(
        &server_config.db_type,
        server_config.to_connection_config().map_err(|e| format!("Invalid connection config: {}", e))?,
    )
    .await?;

    // Store the adapter for future use
    {
        let state: tauri::State<'_, crate::state::AppState> = app.state();
        let mut conns = state.connections.write().await;
        conns.insert(connection_id.to_string(), adapter.clone());
    }

    Ok(adapter)
}

async fn execute_on_adapter(adapter: &ActiveConnection, sql: &str) -> Result<QueryResult, String> {
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
        _ => todo!(),
    }
}

fn get_connection_id(config: Option<&Value>) -> Result<String, String> {
    config
        .and_then(|c| c.get("connectionId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing connectionId in connection config".to_string())
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
        let result = execute_on_adapter(&adapter, sql).await?;
        let json = serde_json::to_string(&result).map_err(|e| e.to_string())?;
        Ok(crate::common::format::truncate_tool_output(json))
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
            _ => todo!(),
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
            _ => todo!(),
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
            _ => todo!(),
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
            _ => todo!(),
        };

        let mut schema_lines: Vec<String> = Vec::new();
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
                _ => todo!(),
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
            _ => todo!(),
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
        description: "Execute an arbitrary SQL query and return the result set. Supports SELECT, INSERT, UPDATE, DELETE, DDL, and any other SQL statement.",
        handler: Arc::new(ExecuteQueryHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "sql": {"type": "string", "description": "The SQL query to execute"}
        }, "required": ["connection_id", "sql"]}),
        risk_level: RiskLevel::Elevated,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: false,
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
        risk_level: RiskLevel::Safe, required_permission: "read",
        source_kind: SourceKind::SqlDatabase, tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__list_tables",
        description: "List all tables in a database schema.",
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
        description: "Get the full database schema (all tables and columns) as DDL-like text. Use this before writing queries to understand the structure.",
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
        description: "Get the query execution plan for a SQL statement. Useful for optimizing slow queries.",
        handler: Arc::new(ExplainQueryHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "sql": {"type": "string", "description": "The SQL query to explain"}
        }, "required": ["connection_id", "sql"]}),
        risk_level: RiskLevel::Safe, required_permission: "read",
        source_kind: SourceKind::SqlDatabase, tags: &["agent"],
        parallel_ok: true,
    });
}
