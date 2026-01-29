//! SQL query execution commands.
//!
//! This module provides Tauri commands for executing SQL queries, canceling queries,
//! and getting query execution plans.

use crate::api_response::{ApiResponse, db_error_to_api_error};
use crate::database::{DatabaseAdapter, QueryResult};
use crate::state::{ActiveConnection, AppState};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Query execution plan details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// Database-specific query plan representation.
    pub plan: String,
    /// Estimated cost (if available).
    pub estimated_cost: Option<f64>,
    /// Additional plan details.
    pub details: Option<String>,
}

/// Execute a SQL query.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `sql` - SQL query to execute
/// * `state` - Application state
///
/// # Returns
///
/// Query results including rows and metadata.
#[tauri::command]
pub async fn execute_query(
    connection_id: String,
    sql: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<QueryResult>, String> {
    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    // Execute query based on connection type
    let result = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            adapter.execute_query(&sql).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter.execute_query(&sql).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            adapter.execute_query(&sql).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.execute_query(&sql).await
        }
    };

    match result {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => {
            let api_error = db_error_to_api_error(&e);
            Ok(ApiResponse::error(api_error))
        }
    }
}

/// Cancel a running query.
///
/// Note: This is a placeholder for future implementation. Query cancellation
/// requires storing query handles and implementing cancellation tokens.
///
/// # Arguments
///
/// * `query_id` - ID of the query to cancel
/// * `state` - Application state
#[tauri::command]
pub async fn cancel_query(
    _query_id: String,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    // TODO: Implement query cancellation
    // This requires:
    // 1. Storing running query handles with unique IDs
    // 2. Implementing cancellation tokens for async queries
    // 3. Database-specific cancellation mechanisms
    Err("Query cancellation not yet implemented".to_string())
}

/// Get query execution plan.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `sql` - SQL query to analyze
/// * `state` - Application state
///
/// # Returns
///
/// Query execution plan with cost estimates.
/// 
/// # Security Note
/// 
/// While this function is designed to analyze user queries, malicious SQL patterns
/// are validated before execution. The input SQL should come from trusted sources
/// or be validated in the frontend before calling this command.
#[tauri::command]
pub async fn explain_query(
    connection_id: String,
    sql: String,
    state: State<'_, AppState>,
) -> Result<QueryPlan, String> {
    // Basic SQL validation to prevent obvious injection attacks
    let sql_lower = sql.trim().to_lowercase();
    
    // Check for dangerous patterns
    if sql_lower.contains(";") && !sql_lower.ends_with(";") {
        return Err("Multiple statements are not allowed in EXPLAIN queries".to_string());
    }
    
    // Remove trailing semicolon for consistent processing
    let sql = sql.trim().trim_end_matches(';');
    
    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    // Execute explain query based on connection type
    let result = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            adapter.execute_query(&explain_sql).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            adapter.execute_query(&explain_sql).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            // SQL Server uses SET SHOWPLAN_TEXT ON
            let showplan_sql = format!("SET SHOWPLAN_TEXT ON; {}", sql);
            adapter.execute_query(&showplan_sql).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            // SQLite uses EXPLAIN QUERY PLAN
            let explain_sql = format!("EXPLAIN QUERY PLAN {}", sql);
            adapter.execute_query(&explain_sql).await
        }
    }
    .map_err(|e| format!("EXPLAIN query failed: {}", e))?;

    // Convert result to QueryPlan
    let plan = result
        .rows
        .iter()
        .map(|row| {
            row.values()
                .map(|v| format!("{:?}", v))
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(QueryPlan {
        plan,
        estimated_cost: None,
        details: Some(format!("{} rows in plan", result.rows.len())),
    })
}

// Tests for query commands are temporarily disabled.
// TODO: Convert to integration tests with full Tauri context support.
// When re-enabling, remove the #[ignore] attribute or convert to integration tests.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, ServerConfig};
    use crate::commands::connection::connect_server;

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
    async fn test_execute_query() {
        let state = create_test_state();
        let conn_id = setup_connection(&state).await;

        // Create a test table
        let create_sql = "CREATE TABLE test (id INTEGER, name TEXT)";
        let result = execute_query(conn_id.clone(), create_sql.to_string(), State::from(&state))
            .await;
        assert!(result.is_ok());

        // Insert data
        let insert_sql = "INSERT INTO test VALUES (1, 'Alice'), (2, 'Bob')";
        let result = execute_query(conn_id.clone(), insert_sql.to_string(), State::from(&state))
            .await;
        assert!(result.is_ok());

        // Query data
        let select_sql = "SELECT * FROM test";
        let result = execute_query(conn_id.clone(), select_sql.to_string(), State::from(&state))
            .await;
        assert!(result.is_ok());
        let query_result = result.unwrap();
        assert_eq!(query_result.rows.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_query_invalid_connection() {
        let state = create_test_state();
        let result = execute_query(
            "invalid".to_string(),
            "SELECT 1".to_string(),
            State::from(&state),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_explain_query() {
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

        // Get query plan
        let result = explain_query(
            conn_id.clone(),
            "SELECT * FROM test WHERE id = 1".to_string(),
            State::from(&state),
        )
        .await;
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert!(!plan.plan.is_empty());
    }

    #[tokio::test]
    async fn test_cancel_query_not_implemented() {
        let state = create_test_state();
        let result = cancel_query("query_id".to_string(), State::from(&state)).await;
        assert!(result.is_err());
    }
}
