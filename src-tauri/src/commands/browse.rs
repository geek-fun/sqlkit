//! Database browsing commands.
//!
//! This module provides Tauri commands for browsing database metadata,
//! including databases, schemas, tables, columns, and table data.

use crate::database::{
    search, ColumnInfo, DatabaseAdapter, DatabaseSchema, MySQLAdapter, PostgresAdapter,
    QueryResult, SqlServerAdapter, TableInfo,
};
use crate::state::{ActiveConnection, AppState};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tauri::State;

/// Parameters for table data queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataQuery {
    /// Database name (optional, uses connection default if not specified).
    pub database: Option<String>,
    /// Table name.
    pub table: String,
    /// Schema name (optional).
    pub schema: Option<String>,
    /// SQL WHERE clause filter (optional).
    pub filter: Option<String>,
    /// Row limit (defaults to 100).
    pub limit: Option<u32>,
    /// Row offset for pagination (defaults to 0).
    pub offset: Option<u32>,
}

/// Quote identifier for safe SQL interpolation.
///
/// This function quotes identifiers according to the database type to prevent
/// SQL injection when building queries with table/column names.
fn quote_identifier(identifier: &str, db_type: &str) -> String {
    match db_type {
        "postgres" => format!("\"{}\"", identifier.replace("\"", "\"\"")),
        "mysql" => format!("`{}`", identifier.replace("`", "``")),
        "sqlserver" => format!("[{}]", identifier.replace("]", "]]")),
        "sqlite" => format!("\"{}\"", identifier.replace("\"", "\"\"")),
        "duckdb" => format!("\"{}\"", identifier.replace("\"", "\"\"")),
        "clickhouse" => format!("`{}`", identifier.replace("`", "``")),
        "jdbc" => format!("\"{}\"", identifier.replace("\"", "\"\"")),
        "trino" => identifier.to_string(),
        _ => identifier.to_string(),
    }
}

/// Build a schema-qualified table identifier.
fn build_qualified_table(schema: Option<&str>, table: &str, db_type: &str) -> String {
    match schema {
        Some(s) if !s.is_empty() => format!(
            "{}.{}",
            quote_identifier(s, db_type),
            quote_identifier(table, db_type)
        ),
        _ => quote_identifier(table, db_type),
    }
}

/// Build a paginated SELECT query.
///
/// # Security Note
///
/// The `filter` parameter is user-supplied SQL for the WHERE clause. Since SQLKit is a
/// desktop application where users manage their own database connections, this is
/// intentional — users already have full query access via the SQL editor.
fn build_paginated_select(
    qualified_table: &str,
    filter: Option<&str>,
    limit: u32,
    offset: u32,
    db_type: &str,
) -> String {
    let where_clause = filter
        .filter(|f| !f.trim().is_empty())
        .map(|f| format!(" WHERE {}", f))
        .unwrap_or_default();

    match db_type {
        "sqlserver" => format!(
            "SELECT * FROM {}{} ORDER BY (SELECT NULL) OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
            qualified_table, where_clause, offset, limit
        ),
        _ => format!(
            "SELECT * FROM {}{} LIMIT {} OFFSET {}",
            qualified_table, where_clause, limit, offset
        ),
    }
}

/// Build a COUNT(*) query for pagination.
fn build_count_query(qualified_table: &str, filter: Option<&str>) -> String {
    let where_clause = filter
        .filter(|f| !f.trim().is_empty())
        .map(|f| format!(" WHERE {}", f))
        .unwrap_or_default();
    format!("SELECT COUNT(*) FROM {}{}", qualified_table, where_clause)
}

/// List all databases on the server.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `state` - Application state
///
/// # Returns
///
/// Vector of database schemas with name, description, and is_system flag.
#[tauri::command]
pub async fn list_databases(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<DatabaseSchema>, String> {
    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Get databases based on connection type
    let databases = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::DuckDb(_) => {
            // DuckDB is file-based and has no separate databases
            return Ok(vec![]);
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_databases().await
        }
    }
    .map_err(|e| format!("Failed to list databases: {}", e))?;

    Ok(databases)
}

/// List all schemas in a database.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `database` - Name of the database
/// * `state` - Application state
///
/// # Returns
///
/// Vector of schema names.
#[tauri::command]
pub async fn list_schemas(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Get schemas based on connection type
    let schemas = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_schemas(Some(&database)).await
        }
        ActiveConnection::MySQL(_) => {
            // MySQL doesn't have schemas separate from databases
            return Ok(vec![database]);
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_schemas(Some(&database)).await
        }
        ActiveConnection::SQLite(_) => {
            // SQLite doesn't have schemas
            return Ok(vec!["main".to_string()]);
        }
        ActiveConnection::DuckDb(_) => {
            return Ok(vec![
                "main".to_string(),
                "information_schema".to_string(),
                "pg_catalog".to_string(),
            ]);
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_schemas(Some(&database)).await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_schemas(Some(&database)).await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_schemas(Some(&database)).await
        }
    }
    .map_err(|e| format!("Failed to list schemas: {}", e))?;

    Ok(schemas)
}

/// List all tables in a database/schema.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `database` - Name of the database
/// * `schema` - Optional schema name (PostgreSQL, SQL Server)
/// * `state` - Application state
///
/// # Returns
///
/// Vector of table information.
#[tauri::command]
pub async fn list_tables(
    connection_id: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String> {
    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Get tables based on connection type
    let tables = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_tables(None, None).await
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_tables(None, schema.as_deref()).await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
    }
    .map_err(|e| format!("Failed to list tables: {}", e))?;

    Ok(tables)
}

/// Get detailed information about a table.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `database` - Database name
/// * `schema` - Schema name (optional)
/// * `table_name` - Table name
/// * `state` - Application state
///
/// # Returns
///
/// Detailed table information including columns.
#[tauri::command]
pub async fn get_table_info(
    connection_id: String,
    database: String,
    schema: Option<String>,
    table_name: String,
    state: State<'_, AppState>,
) -> Result<TableInfo, String> {
    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Get table info based on connection type
    let table_info = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .get_table_info(Some(&database), schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .get_table_info(Some(&database), None, &table_name)
                .await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .get_table_info(Some(&database), schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.get_table_info(None, None, &table_name).await
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .get_table_info(None, schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .get_table_info(Some(&database), None, &table_name)
                .await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .get_table_info(Some(&database), schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .get_table_info(Some(&database), schema.as_deref(), &table_name)
                .await
        }
    }
    .map_err(|e| format!("Failed to get table info: {}", e))?;

    Ok(table_info)
}

/// List columns for a table, including name and data type.
#[tauri::command]
pub async fn list_columns(
    connection_id: String,
    database: String,
    schema: Option<String>,
    table_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<ColumnInfo>, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    let columns = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            if Some(database.as_str()) != adapter.config.database.as_deref() {
                let mut temp_config = adapter.config.clone();
                drop(adapter);
                temp_config.database = Some(database.clone());
                let mut temp = PostgresAdapter::new(temp_config);
                temp.connect()
                    .await
                    .map_err(|e| format!("Failed to connect: {}", e))?;
                temp.list_columns(None, schema.as_deref(), &table_name)
                    .await
            } else {
                adapter
                    .list_columns(None, schema.as_deref(), &table_name)
                    .await
            }
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(Some(&database), None, &table_name)
                .await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            if Some(database.as_str()) != adapter.config.database.as_deref() {
                let mut temp_config = adapter.config.clone();
                drop(adapter);
                temp_config.database = Some(database.clone());
                let mut temp = SqlServerAdapter::new(temp_config);
                temp.connect()
                    .await
                    .map_err(|e| format!("Failed to connect: {}", e))?;
                temp.list_columns(None, schema.as_deref(), &table_name)
                    .await
            } else {
                adapter
                    .list_columns(None, schema.as_deref(), &table_name)
                    .await
            }
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_columns(None, None, &table_name).await
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(None, schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(Some(&database), None, &table_name)
                .await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(None, schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(None, schema.as_deref(), &table_name)
                .await
        }
    }
    .map_err(|e| format!("Failed to list columns: {}", e))?;

    Ok(columns)
}

/// Get table data with pagination and optional WHERE-clause filter.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `query` - Table data query parameters
/// * `state` - Application state
///
/// # Returns
///
/// Query result with table data for the requested page.
#[tauri::command]
pub async fn get_table_data(
    connection_id: String,
    query: TableDataQuery,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    let limit_val = query.limit.unwrap_or(100);
    let offset_val = query.offset.unwrap_or(0);
    let filter_ref = query.filter.as_deref();

    // Execute query based on connection type with proper identifier quoting
    let result = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let qualified =
                build_qualified_table(query.schema.as_deref(), &query.table, "postgres");
            let sql =
                build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "postgres");
            if let Some(ref db) = query.database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp
                        .execute_query(&sql)
                        .await
                        .map_err(|e| format!("Failed to get table data: {}", e));
                }
            }
            adapter.execute_query(&sql).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(query.schema.as_deref(), &query.table, "mysql");
            let sql =
                build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "mysql");
            adapter.execute_query(&sql).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            let qualified =
                build_qualified_table(query.schema.as_deref(), &query.table, "sqlserver");
            let sql =
                build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "sqlserver");
            // If a different database is requested, create a temporary connection to it.
            if let Some(ref db) = query.database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp
                        .execute_query(&sql)
                        .await
                        .map_err(|e| format!("Failed to get table data: {}", e));
                }
            }
            adapter.execute_query(&sql).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            // SQLite has no schemas
            let qualified = build_qualified_table(None, &query.table, "sqlite");
            let sql =
                build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "sqlite");
            adapter.execute_query(&sql).await
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(query.schema.as_deref(), &query.table, "duckdb");
            let sql =
                build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "duckdb");
            adapter.execute_query(&sql).await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            let qualified =
                build_qualified_table(query.schema.as_deref(), &query.table, "clickhouse");
            let sql =
                build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "clickhouse");
            adapter.execute_query(&sql).await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(query.schema.as_deref(), &query.table, "jdbc");
            let sql = build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "jdbc");
            adapter.execute_query(&sql).await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(query.schema.as_deref(), &query.table, "trino");
            let sql =
                build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "trino");
            adapter.execute_query(&sql).await
        }
    }
    .map_err(|e| format!("Failed to get table data: {}", e))?;

    Ok(result)
}

/// Get the total row count for a table, optionally filtered by a WHERE clause.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `table` - Table name
/// * `schema` - Optional schema (or database for MySQL)
/// * `filter` - Optional SQL WHERE clause expression
/// * `state` - Application state
///
/// # Returns
///
/// Total number of rows matching the filter.
#[tauri::command]
pub async fn get_table_count(
    connection_id: String,
    database: Option<String>,
    table: String,
    schema: Option<String>,
    filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    let filter_ref = filter.as_deref();

    let result = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "postgres");
            let query = build_count_query(&qualified, filter_ref);
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    let r = temp
                        .execute_query(&query)
                        .await
                        .map_err(|e| format!("Failed to get table count: {}", e))?;
                    return extract_count(r);
                }
            }
            adapter.execute_query(&query).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "mysql");
            let query = build_count_query(&qualified, filter_ref);
            adapter.execute_query(&query).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "sqlserver");
            let query = build_count_query(&qualified, filter_ref);
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    let r = temp
                        .execute_query(&query)
                        .await
                        .map_err(|e| format!("Failed to get table count: {}", e))?;
                    return extract_count(r);
                }
            }
            adapter.execute_query(&query).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(None, &table, "sqlite");
            let query = build_count_query(&qualified, filter_ref);
            adapter.execute_query(&query).await
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "duckdb");
            let query = build_count_query(&qualified, filter_ref);
            adapter.execute_query(&query).await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "clickhouse");
            let query = build_count_query(&qualified, filter_ref);
            adapter.execute_query(&query).await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "jdbc");
            let query = build_count_query(&qualified, filter_ref);
            adapter.execute_query(&query).await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "trino");
            let query = build_count_query(&qualified, filter_ref);
            adapter.execute_query(&query).await
        }
    }
    .map_err(|e| format!("Failed to get table count: {}", e))?;

    extract_count(result)
}

/// Convert a JSON value to a SQL literal for safe embedding in UPDATE/DELETE queries.
///
/// # Security Note
///
/// Column names (`key`) are quoted via `quote_identifier`; only values are serialised
/// here — they are embedded as properly-quoted SQL literals (not via user-supplied SQL
/// strings), so the risk of injection is limited. This is intentional in SQLKit, which
/// is a desktop application where the user is already authenticated to the target DB.
fn json_value_to_sql_literal(val: &JsonValue) -> String {
    match val {
        JsonValue::Null => "NULL".to_string(),
        JsonValue::Bool(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => format!("'{}'", s.replace('\'', "''")),
        JsonValue::Array(_) | JsonValue::Object(_) => {
            let json_str = val.to_string().replace('\'', "''");
            format!("'{}'", json_str)
        }
    }
}

/// Build a WHERE clause from a map of primary-key column → value pairs.
fn build_pk_where(pk_values: &HashMap<String, JsonValue>, db_type: &str) -> String {
    pk_values
        .iter()
        .map(|(col, val)| {
            let quoted_col = quote_identifier(col, db_type);
            if val.is_null() {
                format!("{} IS NULL", quoted_col)
            } else {
                format!("{} = {}", quoted_col, json_value_to_sql_literal(val))
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// Update a single row in a table identified by its primary key values.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `table` - Table name
/// * `schema` - Optional schema name
/// * `pk_values` - Map of primary-key column name → current value (used to identify the row)
/// * `updates` - Map of column name → new value to write
/// * `state` - Application state
#[tauri::command]
pub async fn update_table_row(
    connection_id: String,
    database: Option<String>,
    table: String,
    schema: Option<String>,
    pk_values: HashMap<String, JsonValue>,
    updates: HashMap<String, JsonValue>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if updates.is_empty() {
        return Ok(());
    }

    let connections = state.connections.lock().await;
    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    let build_update_sql = |db_type: &str| -> Result<String, String> {
        if pk_values.is_empty() {
            return Err("Cannot update row: no primary key values provided".to_string());
        }
        let qualified = build_qualified_table(schema.as_deref(), &table, db_type);
        let set_clause = updates
            .iter()
            .map(|(col, val)| {
                format!(
                    "{} = {}",
                    quote_identifier(col, db_type),
                    json_value_to_sql_literal(val)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let where_clause = build_pk_where(&pk_values, db_type);
        Ok(format!(
            "UPDATE {} SET {} WHERE {}",
            qualified, set_clause, where_clause
        ))
    };

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_update_sql("postgres")?;
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp
                        .execute_query(&sql)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("Failed to update row: {}", e));
                }
            }
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to update row: {}", e))?;
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_update_sql("mysql")?;
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = MySQLAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp
                        .execute_query(&sql)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("Failed to update row: {}", e));
                }
            }
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to update row: {}", e))?;
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_update_sql("sqlserver")?;
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp
                        .execute_query(&sql)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("Failed to update row: {}", e));
                }
            }
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to update row: {}", e))?;
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_update_sql("sqlite")?;
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to update row: {}", e))?;
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_update_sql("duckdb")?;
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to update row: {}", e))?;
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_update_sql("clickhouse")?;
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to update row: {}", e))?;
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_update_sql("jdbc")?;
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to update row: {}", e))?;
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_update_sql("trino")?;
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to update row: {}", e))?;
        }
    }

    Ok(())
}

/// Delete a single row from a table identified by its primary key values.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `table` - Table name
/// * `schema` - Optional schema name
/// * `pk_values` - Map of primary-key column name → value (used to identify the row)
/// * `state` - Application state
#[tauri::command]
pub async fn delete_table_row(
    connection_id: String,
    database: Option<String>,
    table: String,
    schema: Option<String>,
    pk_values: HashMap<String, JsonValue>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if pk_values.is_empty() {
        return Err("Cannot delete row: no primary key values provided".to_string());
    }

    let connections = state.connections.lock().await;
    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    let build_delete_sql = |db_type: &str| -> String {
        let qualified = build_qualified_table(schema.as_deref(), &table, db_type);
        let where_clause = build_pk_where(&pk_values, db_type);
        format!("DELETE FROM {} WHERE {}", qualified, where_clause)
    };

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_delete_sql("postgres");
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp
                        .execute_query(&sql)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("Failed to delete row: {}", e));
                }
            }
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to delete row: {}", e))?;
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_delete_sql("mysql");
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = MySQLAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp
                        .execute_query(&sql)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("Failed to delete row: {}", e));
                }
            }
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to delete row: {}", e))?;
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_delete_sql("sqlserver");
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp
                        .execute_query(&sql)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("Failed to delete row: {}", e));
                }
            }
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to delete row: {}", e))?;
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_delete_sql("sqlite");
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to delete row: {}", e))?;
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_delete_sql("duckdb");
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to delete row: {}", e))?;
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_delete_sql("clickhouse");
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to delete row: {}", e))?;
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_delete_sql("jdbc");
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to delete row: {}", e))?;
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            let sql = build_delete_sql("trino");
            adapter
                .execute_query(&sql)
                .await
                .map_err(|e| format!("Failed to delete row: {}", e))?;
        }
    }

    Ok(())
}

/// Extract a COUNT(*) value from a single-cell query result.
fn extract_count(result: QueryResult) -> Result<u64, String> {
    result
        .rows
        .first()
        .and_then(|row| row.values().next())
        .and_then(|val| match val {
            crate::database::types::QueryValue::Int(n) => Some(*n as u64),
            crate::database::types::QueryValue::String(s) => s.parse::<u64>().ok(),
            _ => None,
        })
        .ok_or_else(|| "Failed to extract row count from query result".to_string())
}

/// Map an [`ActiveConnection`] variant to the db_type string used by
/// [`quote_identifier`] and [`search::build_table_search_where`].
fn get_db_type_string(connection: &ActiveConnection) -> &'static str {
    match connection {
        ActiveConnection::Postgres(_) => "postgres",
        ActiveConnection::MySQL(_) => "mysql",
        ActiveConnection::SQLServer(_) => "sqlserver",
        ActiveConnection::SQLite(_) => "sqlite",
        ActiveConnection::DuckDb(_) => "duckdb",
        ActiveConnection::ClickHouse(_) => "clickhouse",
        ActiveConnection::JdbcBridge(_) => "jdbc",
        ActiveConnection::HttpSql(_) => "trino",
    }
}

/// Build a SQL WHERE clause that searches across all text and numeric columns in a table.
///
/// The generated WHERE clause uses dialect-aware casting and LIKE matching,
/// skipping BLOB/BINARY/geometry columns entirely. The frontend can pass the
/// returned string as the `filter` parameter to [`get_table_data`] and [`get_table_count`]
/// to show only matching rows.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `database` - Database name containing the table
/// * `schema` - Optional schema name (PostgreSQL, SQL Server)
/// * `table_name` - Table name to search
/// * `search_term` - The user's search term
/// * `state` - Application state
///
/// # Returns
///
/// A WHERE clause string like `(LOWER(CAST("col1" AS TEXT)) LIKE '%term%' OR ...)`,
/// or an empty string if no searchable columns are found.
#[tauri::command]
pub async fn build_table_search_filter(
    connection_id: String,
    database: String,
    schema: Option<String>,
    table_name: String,
    search_term: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    let db_type = get_db_type_string(connection);

    let columns = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            if Some(database.as_str()) != adapter.config.database.as_deref() {
                let mut temp_config = adapter.config.clone();
                drop(adapter);
                temp_config.database = Some(database.clone());
                let mut temp = PostgresAdapter::new(temp_config);
                temp.connect()
                    .await
                    .map_err(|e| format!("Failed to connect to database '{}': {}", database, e))?;
                temp.list_columns(None, schema.as_deref(), &table_name)
                    .await
            } else {
                adapter
                    .list_columns(None, schema.as_deref(), &table_name)
                    .await
            }
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(Some(&database), None, &table_name)
                .await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            if Some(database.as_str()) != adapter.config.database.as_deref() {
                let mut temp_config = adapter.config.clone();
                drop(adapter);
                temp_config.database = Some(database.clone());
                let mut temp = SqlServerAdapter::new(temp_config);
                temp.connect()
                    .await
                    .map_err(|e| format!("Failed to connect to database '{}': {}", database, e))?;
                temp.list_columns(None, schema.as_deref(), &table_name)
                    .await
            } else {
                adapter
                    .list_columns(None, schema.as_deref(), &table_name)
                    .await
            }
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_columns(None, None, &table_name).await
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(None, schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(Some(&database), None, &table_name)
                .await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(None, schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(None, schema.as_deref(), &table_name)
                .await
        }
    }
    .map_err(|e| format!("Failed to list columns for search: {}", e))?;

    let where_clause = search::build_table_search_where(db_type, &columns, &search_term);
    Ok(where_clause.unwrap_or_default())
}

// Tests for browse commands are temporarily disabled.
// TODO: Convert to integration tests with full Tauri context support.
// The tests below require a Tauri State which cannot be created in unit tests.
// Integration tests should be added in src-tauri/tests/ directory.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_identifier() {
        assert_eq!(quote_identifier("table", "postgres"), "\"table\"");
        assert_eq!(quote_identifier("table", "mysql"), "`table`");
        assert_eq!(quote_identifier("table", "sqlserver"), "[table]");
        assert_eq!(quote_identifier("table", "sqlite"), "\"table\"");
    }

    #[test]
    fn test_quote_identifier_with_special_chars() {
        assert_eq!(quote_identifier("ta\"ble", "postgres"), "\"ta\"\"ble\"");
        assert_eq!(quote_identifier("ta`ble", "mysql"), "`ta``ble`");
        assert_eq!(quote_identifier("ta]ble", "sqlserver"), "[ta]]ble]");
    }

    #[test]
    fn test_build_qualified_table() {
        assert_eq!(
            build_qualified_table(Some("schema"), "table", "postgres"),
            "\"schema\".\"table\""
        );
        assert_eq!(
            build_qualified_table(None, "table", "postgres"),
            "\"table\""
        );
    }

    #[test]
    fn test_build_paginated_select() {
        let query = build_paginated_select("\"table\"", None, 10, 0, "postgres");
        assert!(query.contains("LIMIT 10"));
        assert!(query.contains("OFFSET 0"));

        let query_with_filter =
            build_paginated_select("\"table\"", Some("id = 1"), 10, 5, "postgres");
        assert!(query_with_filter.contains("WHERE id = 1"));
        assert!(query_with_filter.contains("OFFSET 5"));
    }

    #[test]
    fn test_build_count_query() {
        let query = build_count_query("\"table\"", None);
        assert_eq!(query, "SELECT COUNT(*) FROM \"table\"");

        let query_with_filter = build_count_query("\"table\"", Some("id = 1"));
        assert_eq!(
            query_with_filter,
            "SELECT COUNT(*) FROM \"table\" WHERE id = 1"
        );
    }

    #[test]
    fn test_json_value_to_sql_literal() {
        use serde_json::json;

        assert_eq!(json_value_to_sql_literal(&json!(null)), "NULL");
        assert_eq!(json_value_to_sql_literal(&json!(true)), "TRUE");
        assert_eq!(json_value_to_sql_literal(&json!(false)), "FALSE");
        assert_eq!(json_value_to_sql_literal(&json!(42)), "42");
        assert_eq!(json_value_to_sql_literal(&json!("hello")), "'hello'");
        assert_eq!(json_value_to_sql_literal(&json!("it's")), "'it''s'");
    }

    #[test]
    fn test_build_pk_where() {
        use serde_json::json;
        use std::collections::HashMap;

        let mut pk_values = HashMap::new();
        pk_values.insert("id".to_string(), json!(1));
        pk_values.insert("name".to_string(), json!("test"));

        let where_clause = build_pk_where(&pk_values, "postgres");
        // Order may vary, so just check both parts are present
        assert!(where_clause.contains("\"id\" = 1"));
        assert!(where_clause.contains("\"name\" = 'test'"));
        assert!(where_clause.contains(" AND "));
    }
}
