//! SQL query execution commands.
//!
//! This module provides Tauri commands for executing SQL queries, canceling queries,
//! and getting query execution plans.

use crate::api_response::{db_error_to_api_error, ApiResponse};
use crate::database::{
    ClickHouseAdapter, ConnectionConfig, DatabaseAdapter, DuckDbAdapter, HttpSqlAdapter,
    JdbcBridgeAdapter, MySQLAdapter, PostgresAdapter, QueryResult, SqlServerAdapter,
};
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

/// Connect a temporary adapter, execute a query, then disconnect.
///
/// The caller must **not** hold `state.connections` when calling this function,
/// because `connect` and `execute_query` involve network I/O.
/// Always calls `disconnect()` after execution to ensure proper resource cleanup.
async fn execute_with_temp_adapter<A>(
    mut temp: A,
    sql: &str,
) -> Result<ApiResponse<QueryResult>, String>
where
    A: DatabaseAdapter,
{
    temp.connect()
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;
    let result = temp
        .execute_query(sql)
        .await
        .map(ApiResponse::success)
        .or_else(|e| Ok(ApiResponse::error(db_error_to_api_error(&e))));
    if let Err(e) = temp.disconnect().await {
        eprintln!(
            "Warning: failed to disconnect temporary adapter after query: {}",
            e
        );
    }
    result
}

/// Execute a SQL query.
///
/// # Arguments
///
/// * `connection_id` - ID of the active connection
/// * `sql` - SQL query to execute
/// * `database` - Optional target database to execute the query against (creates a
///   temporary connection if different from the active connection's database)
/// * `state` - Application state
///
/// # Returns
///
/// Query results including rows and metadata.
#[tauri::command]
pub async fn execute_query(
    connection_id: String,
    sql: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<ApiResponse<QueryResult>, String> {
    // If a target database is specified we may need a temporary adapter.
    // We hold the connections lock only long enough to clone the config, then
    // release it before doing any network I/O.
    if let Some(ref db) = database {
        enum TempKind {
            Postgres(ConnectionConfig),
            MySQL(ConnectionConfig),
            SQLServer(ConnectionConfig),
            DuckDb(ConnectionConfig),
            ClickHouse(ConnectionConfig),
            JdbcBridge(ConnectionConfig),
            HttpSql(ConnectionConfig),
        }

        let temp_kind: Option<TempKind> = {
            let connections = state.connections.lock().await;
            let connection = connections
                .get(&connection_id)
                .ok_or_else(|| "No active connection found".to_string())?;

            match connection {
                ActiveConnection::Postgres(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::Postgres(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::MySQL(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::MySQL(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::SQLServer(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::SQLServer(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::DuckDb(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::DuckDb(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::ClickHouse(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::ClickHouse(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::JdbcBridge(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::JdbcBridge(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::HttpSql(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::HttpSql(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::SQLite(_) => None,
            }
            // connections lock is dropped here, before any network I/O
        };

        if let Some(kind) = temp_kind {
            return match kind {
                TempKind::Postgres(cfg) => {
                    execute_with_temp_adapter(PostgresAdapter::new(cfg), &sql).await
                }
                TempKind::MySQL(cfg) => {
                    execute_with_temp_adapter(MySQLAdapter::new(cfg), &sql).await
                }
                TempKind::SQLServer(cfg) => {
                    execute_with_temp_adapter(SqlServerAdapter::new(cfg), &sql).await
                }
                TempKind::DuckDb(cfg) => {
                    execute_with_temp_adapter(DuckDbAdapter::new(cfg), &sql).await
                }
                TempKind::ClickHouse(cfg) => {
                    execute_with_temp_adapter(ClickHouseAdapter::new(cfg), &sql).await
                }
                TempKind::JdbcBridge(cfg) => {
                    execute_with_temp_adapter(JdbcBridgeAdapter::new(cfg), &sql).await
                }
                TempKind::HttpSql(cfg) => {
                    execute_with_temp_adapter(HttpSqlAdapter::new(cfg), &sql).await
                }
            };
        }
    }

    // Common path: use the already-connected adapter
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

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
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            adapter.execute_query(&sql).await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            adapter.execute_query(&sql).await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            adapter.execute_query(&sql).await
        }
        ActiveConnection::HttpSql(adapter) => {
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
pub async fn cancel_query(_query_id: String, _state: State<'_, AppState>) -> Result<(), String> {
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
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            adapter.execute_query(&explain_sql).await
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            adapter.execute_query(&explain_sql).await
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            adapter.execute_query(&explain_sql).await
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
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
// The tests below require a Tauri State which cannot be created in unit tests.
// Integration tests should be added in src-tauri/tests/ directory.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_plan_structure() {
        let plan = QueryPlan {
            plan: "Seq Scan on users".to_string(),
            estimated_cost: Some(1.0),
            details: Some("1 row".to_string()),
        };

        assert_eq!(plan.plan, "Seq Scan on users");
        assert_eq!(plan.estimated_cost, Some(1.0));
        assert!(plan.details.is_some());
    }

    #[test]
    fn test_sql_validation() {
        let valid_sql = "SELECT * FROM users";
        let sql_lower = valid_sql.trim().to_lowercase();
        let should_reject = sql_lower.contains(";") && !sql_lower.ends_with(";");
        assert!(
            !should_reject,
            "Single statement without semicolon should be valid"
        );

        let valid_sql_with_trailing = "SELECT * FROM users;";
        let sql_lower = valid_sql_with_trailing.trim().to_lowercase();
        let should_reject = sql_lower.contains(";") && !sql_lower.ends_with(";");
        assert!(
            !should_reject,
            "Single statement with trailing semicolon should be valid"
        );

        let invalid_sql = "SELECT * FROM users; DROP TABLE users";
        let sql_lower = invalid_sql.trim().to_lowercase();
        let should_reject = sql_lower.contains(";") && !sql_lower.ends_with(";");
        assert!(
            should_reject,
            "Multiple statements without trailing semicolon should be rejected"
        );
    }
}
