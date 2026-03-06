//! Database browsing commands.
//!
//! This module provides Tauri commands for browsing database metadata,
//! including databases, schemas, tables, columns, and table data.

use crate::database::{DatabaseAdapter, PostgresAdapter, SqlServerAdapter, ColumnInfo, QueryResult, TableInfo};
use crate::state::{ActiveConnection, AppState};
use tauri::State;

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
/// Vector of database names.
#[tauri::command]
pub async fn list_databases(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let connections = state
        .connections
        .lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Get databases based on connection type
    let databases = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter.list_databases().await
        }
        ActiveConnection::SQLite(_) => {
            // SQLite doesn't have multiple databases in the same connection
            return Ok(vec!["main".to_string()]);
        }
    }
    .map_err(|e| format!("Failed to list databases: {}", e))?;

    Ok(databases.iter().map(|db| db.name.clone()).collect())
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
    let connections = state
        .connections
        .lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Get schemas based on connection type
    let schemas = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter.list_schemas(Some(&database)).await
        }
        ActiveConnection::MySQL(_) => {
            // MySQL doesn't have schemas separate from databases
            return Ok(vec![database]);
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter.list_schemas(Some(&database)).await
        }
        ActiveConnection::SQLite(_) => {
            // SQLite doesn't have schemas
            return Ok(vec!["main".to_string()]);
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
    let connections = state
        .connections
        .lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Get tables based on connection type
    let tables = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter
                .list_tables(Some(&database), schema.as_deref())
                .await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter.list_tables(None, None).await
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
    let connections = state
        .connections
        .lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Get table info based on connection type
    let table_info = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter
                .get_table_info(Some(&database), schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter
                .get_table_info(Some(&database), None, &table_name)
                .await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter
                .get_table_info(Some(&database), schema.as_deref(), &table_name)
                .await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter
                .lock().await;
            adapter.get_table_info(None, None, &table_name).await
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
                temp.connect().await.map_err(|e| format!("Failed to connect: {}", e))?;
                temp.list_columns(None, schema.as_deref(), &table_name).await
            } else {
                adapter.list_columns(None, schema.as_deref(), &table_name).await
            }
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_columns(Some(&database), None, &table_name).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            if Some(database.as_str()) != adapter.config.database.as_deref() {
                let mut temp_config = adapter.config.clone();
                drop(adapter);
                temp_config.database = Some(database.clone());
                let mut temp = SqlServerAdapter::new(temp_config);
                temp.connect().await.map_err(|e| format!("Failed to connect: {}", e))?;
                temp.list_columns(None, schema.as_deref(), &table_name).await
            } else {
                adapter.list_columns(None, schema.as_deref(), &table_name).await
            }
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.list_columns(None, None, &table_name).await
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
/// * `table` - Table name
/// * `schema` - Optional schema (or database for MySQL)
/// * `filter` - Optional SQL WHERE clause expression
/// * `limit` - Row limit per page (defaults to 100)
/// * `offset` - Row offset for pagination (defaults to 0)
/// * `state` - Application state
///
/// # Returns
///
/// Query result with table data for the requested page.
#[tauri::command]
pub async fn get_table_data(
    connection_id: String,
    database: Option<String>,
    table: String,
    schema: Option<String>,
    filter: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let connections = state
        .connections
        .lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    let limit_val = limit.unwrap_or(100);
    let offset_val = offset.unwrap_or(0);
    let filter_ref = filter.as_deref();

    // Execute query based on connection type with proper identifier quoting
    let result = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "postgres");
            let query = build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "postgres");
            // If a different database is requested, create a temporary connection to it.
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect().await.map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp.execute_query(&query).await.map_err(|e| format!("Failed to get table data: {}", e));
                }
            }
            adapter.execute_query(&query).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "mysql");
            let query = build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "mysql");
            adapter.execute_query(&query).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            let qualified = build_qualified_table(schema.as_deref(), &table, "sqlserver");
            let query = build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "sqlserver");
            // If a different database is requested, create a temporary connection to it.
            if let Some(ref db) = database {
                if Some(db.as_str()) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.clone());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect().await.map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    return temp.execute_query(&query).await.map_err(|e| format!("Failed to get table data: {}", e));
                }
            }
            adapter.execute_query(&query).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            // SQLite has no schemas
            let qualified = build_qualified_table(None, &table, "sqlite");
            let query = build_paginated_select(&qualified, filter_ref, limit_val, offset_val, "sqlite");
            adapter.execute_query(&query).await
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
    let connections = state
        .connections
        .lock().await;

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
                    temp.connect().await.map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    let r = temp.execute_query(&query).await.map_err(|e| format!("Failed to get table count: {}", e))?;
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
                    temp.connect().await.map_err(|e| format!("Failed to connect to database '{}': {}", db, e))?;
                    let r = temp.execute_query(&query).await.map_err(|e| format!("Failed to get table count: {}", e))?;
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
    }
    .map_err(|e| format!("Failed to get table count: {}", e))?;

    extract_count(result)
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

// Tests for browse commands are temporarily disabled.
// TODO: Convert to integration tests with full Tauri context support.
// When re-enabling, remove the #[ignore] attribute or convert to integration tests.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::connection::connect_server;
    use crate::commands::query::execute_query;
    use crate::state::{AppState, ServerConfig};

    fn create_test_state() -> AppState {
        AppState::new()
    }

    async fn setup_connection(state: &AppState) -> String {
        let server = ServerConfig::new(
            "Test Server".to_string(),
            "sqlite".to_string(),
            ":memory:".to_string(),
            0,
            "".to_string(),
        );
        let server_id = server.id.clone();

        // Save server
        {
            let mut config = state.config.blocking_lock();
            config.servers.insert(server_id.clone(), server);
        }

        // Connect
        connect_server(server_id.clone(), State::from(state))
            .await
            .unwrap();

        server_id
    }

    #[tokio::test]
    #[ignore] // Requires Tauri context - convert to integration test
    async fn test_list_databases() {
        let state = create_test_state();
        let conn_id = setup_connection(&state).await;

        let result = list_databases(conn_id, State::from(&state)).await;
        assert!(result.is_ok());
        let databases = result.unwrap();
        assert!(databases.contains(&"main".to_string()));
    }

    #[tokio::test]
    #[ignore] // Requires Tauri context - convert to integration test
    async fn test_list_schemas() {
        let state = create_test_state();
        let conn_id = setup_connection(&state).await;

        let result = list_schemas(conn_id, "main".to_string(), State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires Tauri context - convert to integration test
    async fn test_list_tables() {
        let state = create_test_state();
        let conn_id = setup_connection(&state).await;

        // Create a test table
        execute_query(
            conn_id.clone(),
            "CREATE TABLE test (id INTEGER, name TEXT)".to_string(),
            State::from(&state),
        )
        .await
        .unwrap();

        let result = list_tables(conn_id, "main".to_string(), None, State::from(&state)).await;
        assert!(result.is_ok());
        let tables = result.unwrap();
        assert!(tables.iter().any(|t| t.name == "test"));
    }

    #[tokio::test]
    #[ignore] // Requires Tauri context - convert to integration test
    async fn test_get_table_info() {
        let state = create_test_state();
        let conn_id = setup_connection(&state).await;

        // Create a test table
        execute_query(
            conn_id.clone(),
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)".to_string(),
            State::from(&state),
        )
        .await
        .unwrap();

        let result = get_table_info(
            conn_id,
            "main".to_string(),
            None,
            "test".to_string(),
            State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        let table_info = result.unwrap();
        assert_eq!(table_info.name, "test");
        assert_eq!(table_info.columns.len(), 2);
    }

    #[tokio::test]
    #[ignore] // Requires Tauri context - convert to integration test
    async fn test_get_table_data() {
        let state = create_test_state();
        let conn_id = setup_connection(&state).await;

        // Create and populate test table
        execute_query(
            conn_id.clone(),
            "CREATE TABLE test (id INTEGER, name TEXT)".to_string(),
            State::from(&state),
        )
        .await
        .unwrap();
        execute_query(
            conn_id.clone(),
            "INSERT INTO test VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie')".to_string(),
            State::from(&state),
        )
        .await
        .unwrap();

        // Get all data
        let result = get_table_data(conn_id.clone(), "test".to_string(), None, State::from(&state))
            .await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.rows.len(), 3);

        // Get limited data
        let result =
            get_table_data(conn_id.clone(), "test".to_string(), Some(2), State::from(&state)).await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.rows.len(), 2);
    }
}
