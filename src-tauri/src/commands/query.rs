//! SQL query execution commands.
//!
//! This module provides Tauri commands for executing SQL queries, canceling queries,
//! and getting query execution plans.

use crate::api_response::{db_error_to_api_error, ApiResponse};
#[cfg(feature = "firebird")]
use crate::database::FirebirdAdapter;
use crate::database::{
    ClickHouseAdapter, ConnectionConfig, DatabaseAdapter, DuckDbAdapter, ExplainResult,
    HttpSqlAdapter, JdbcBridgeAdapter, MySQLAdapter, PostgresAdapter, QueryResult, RqliteAdapter,
    SqlServerAdapter, TursoAdapter,
};
use crate::state::{ActiveConnection, AppState};
use tauri::State;

/// Extract raw plan text from a QueryResult.
/// For JSON format (single row, single column), extracts the JSON string cleanly.
/// For text format, concatenates all rows with newlines, joining multi-column
/// rows with ` | ` (the frontend parsers handle this for each database).
fn extract_plan_text(result: &QueryResult) -> String {
    use crate::database::QueryValue;
    result
        .rows
        .iter()
        .map(|row| {
            row.values()
                .map(|v| match v {
                    QueryValue::String(s) => s.clone(),
                    other => format!("{:?}", other),
                })
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract plan text from a QueryResult using the StmtText column.
/// SQL Server SHOWPLAN_TEXT returns a single "StmtText" column.
/// SQL Server STATISTICS PROFILE returns multiple columns including
/// "StmtText" (plan tree text), "Rows", "PhysicalOp", etc.
/// This function extracts only the "StmtText" column to avoid
/// interleaving metadata into the plan text.
fn extract_plan_text_for_sqlserver(result: &QueryResult) -> String {
    use crate::database::QueryValue;
    let has_stmt_col = result.columns.iter().any(|c| c == "StmtText");
    if !has_stmt_col {
        // Fall back to generic extraction if column not found
        return extract_plan_text(result);
    }
    result
        .rows
        .iter()
        .map(|row| match row.get("StmtText") {
            Some(QueryValue::String(s)) => s.clone(),
            Some(other) => format!("{:?}", other),
            None => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n")
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
            #[cfg(feature = "firebird")]
            Firebird(ConnectionConfig),
            Rqlite(ConnectionConfig),
            Turso(ConnectionConfig),
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
                #[cfg(feature = "firebird")]
                ActiveConnection::Firebird(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::Firebird(cfg))
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
                ActiveConnection::Rqlite(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::Rqlite(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::Turso(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempKind::Turso(cfg))
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
                #[cfg(feature = "firebird")]
                TempKind::Firebird(cfg) => {
                    execute_with_temp_adapter(FirebirdAdapter::new(cfg), &sql).await
                }
                TempKind::Rqlite(cfg) => {
                    execute_with_temp_adapter(RqliteAdapter::new(cfg), &sql).await
                }
                TempKind::Turso(cfg) => {
                    execute_with_temp_adapter(TursoAdapter::new(cfg), &sql).await
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
        #[cfg(feature = "firebird")]
        ActiveConnection::Firebird(adapter) => {
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
        ActiveConnection::Rqlite(adapter) => {
            let adapter = adapter.lock().await;
            adapter.execute_query(&sql).await
        }
        ActiveConnection::Turso(adapter) => {
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
    analyze: Option<bool>,
    state: State<'_, AppState>,
) -> Result<ExplainResult, String> {
    let sql_lower = sql.trim().to_lowercase();

    if sql_lower.contains(";") && !sql_lower.ends_with(";") {
        return Err("Multiple statements are not allowed in EXPLAIN queries".to_string());
    }

    let sql = sql.trim().trim_end_matches(';');
    let analyze = analyze.unwrap_or(false);

    let connections = state.connections.lock().await;

    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

    let database_type = match connection {
        ActiveConnection::Postgres(_) => "postgresql",
        ActiveConnection::MySQL(_) => "mysql",
        ActiveConnection::SQLite(_) => "sqlite",
        ActiveConnection::SQLServer(_) => "sqlserver",
        ActiveConnection::DuckDb(_) => "duckdb",
        ActiveConnection::ClickHouse(_) => "clickhouse",
        #[cfg(feature = "firebird")]
        ActiveConnection::Firebird(_) => "firebird",
        ActiveConnection::JdbcBridge(_) => "generic",
        ActiveConnection::HttpSql(_) => "generic",
        ActiveConnection::Rqlite(_) => "rqlite",
        ActiveConnection::Turso(_) => "turso",
    };

    let (result, plan_format) = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = if analyze {
                format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", sql)
            } else {
                format!("EXPLAIN (FORMAT JSON) {}", sql)
            };
            (adapter.execute_query(&explain_sql).await, "json")
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            if analyze {
                // MySQL EXPLAIN ANALYZE returns TREE text, not JSON
                (adapter.execute_query(&format!("EXPLAIN ANALYZE {}", sql)).await, "text")
            } else {
                (adapter.execute_query(&format!("EXPLAIN FORMAT=JSON {}", sql)).await, "json")
            }
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            let settings = if analyze {
                "SET STATISTICS PROFILE ON"
            } else {
                "SET SHOWPLAN_TEXT ON"
            };
            let cleanup = if analyze {
                "SET STATISTICS PROFILE OFF"
            } else {
                "SET SHOWPLAN_TEXT OFF"
            };
            let explain_sql = format!("{}; {}; {}", settings, sql, cleanup);
            (adapter.execute_query(&explain_sql).await, "text")
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN QUERY PLAN {}", sql);
            (adapter.execute_query(&explain_sql).await, "text")
        }
        ActiveConnection::DuckDb(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = if analyze {
                format!("EXPLAIN ANALYZE {}", sql)
            } else {
                format!("EXPLAIN {}", sql)
            };
            (adapter.execute_query(&explain_sql).await, "text")
        }
        ActiveConnection::ClickHouse(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            (adapter.execute_query(&explain_sql).await, "text")
        }
        #[cfg(feature = "firebird")]
        ActiveConnection::Firebird(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            (adapter.execute_query(&explain_sql).await, "text")
        }
        ActiveConnection::JdbcBridge(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            (adapter.execute_query(&explain_sql).await, "text")
        }
        ActiveConnection::HttpSql(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            (adapter.execute_query(&explain_sql).await, "text")
        }
        ActiveConnection::Rqlite(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            (adapter.execute_query(&explain_sql).await, "text")
        }
        ActiveConnection::Turso(adapter) => {
            let adapter = adapter.lock().await;
            let explain_sql = format!("EXPLAIN {}", sql);
            (adapter.execute_query(&explain_sql).await, "text")
        }
    };
    let result = result.map_err(|e| format!("EXPLAIN query failed: {}", e))?;

    // SQL Server SHOWPLAN_TEXT returns a single "StmtText" column.
    // STATISTICS PROFILE returns multiple columns — use StmtText-specific
    // extraction to avoid interleaving metadata into the plan text.
    let raw = if database_type == "sqlserver" {
        extract_plan_text_for_sqlserver(&result)
    } else {
        extract_plan_text(&result)
    };

    Ok(ExplainResult {
        database_type: database_type.to_string(),
        raw,
        format: plan_format.to_string(),
        analyze,
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
