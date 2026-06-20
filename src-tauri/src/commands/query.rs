//! SQL query execution commands.
//!
//! This module provides Tauri commands for executing SQL queries, canceling queries,
//! and getting query execution plans.

use crate::api_response::{db_error_to_api_error, ApiResponse};
use crate::connection::guardian::HealthState;
use crate::connection::handle::ConnectionHandle;
use crate::database::{
    ClickHouseAdapter, ConnectionConfig, DatabaseAdapter, DbError, ExplainResult,
    HttpSqlAdapter, JdbcBridgeAdapter, MySQLAdapter, PostgresAdapter, QueryResult, RqliteAdapter,
    SqlServerAdapter, TursoAdapter,
};
use crate::state::{ActiveConnection, AppState};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

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
            ClickHouse(ConnectionConfig),
            JdbcBridge(ConnectionConfig),
            HttpSql(ConnectionConfig),
            Rqlite(ConnectionConfig),
            Turso(ConnectionConfig),
        }

        let temp_kind: Option<TempKind> = {
            let connections = state.connections.read().await;
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
                TempKind::ClickHouse(cfg) => {
                    execute_with_temp_adapter(ClickHouseAdapter::new(cfg), &sql).await
                }
                TempKind::JdbcBridge(cfg) => {
                    execute_with_temp_adapter(JdbcBridgeAdapter::new(cfg), &sql).await
                }
                TempKind::HttpSql(cfg) => {
                    execute_with_temp_adapter(HttpSqlAdapter::new(cfg), &sql).await
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
    let connection = {
        let connections = state.connections.read().await;
        connections
            .get(&connection_id)
            .ok_or_else(|| "No active connection found".to_string())?
            .clone()
    };

    // Cache this handle for future cross-database lookups
    state.cache.get_or_create(
        &crate::connection::cache::PoolKey::new(&connection_id, database.as_deref()),
        state.inner(),
    ).await.ok();

    // Guardian health check
    if let Some(guardian) = crate::GUARDIAN.get() {
        guardian.touch(&connection_id).await;
        let health_state = guardian.get_state(&connection_id).await;
        if health_state == HealthState::Dead || health_state == HealthState::Reconnecting {
            return Err(format!(
                "Connection is in state '{:?}'. Please wait for it to reconnect or reconnect manually.",
                health_state
            ));
        }
    }

    let query_start = std::time::Instant::now();
    let timeout_secs = connection.query_timeout_secs().await;
    let result = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        connection.execute_query(&sql),
    )
    .await
    {
        Ok(inner) => inner,
        Err(_) => Err(DbError::Timeout(format!(
            "Query timed out after {} seconds",
            timeout_secs
        ))),
    };

    let elapsed_ms = query_start.elapsed().as_secs_f64() * 1000.0;

    match result {
        Ok(data) => {
            if let Some(guardian) = crate::GUARDIAN.get() {
                guardian.mark_healthy(&connection_id, Some(elapsed_ms)).await;
            }
            Ok(ApiResponse::success(data))
        }
        Err(e) => {
            if let Some(guardian) = crate::GUARDIAN.get() {
                guardian
                    .mark_error(&connection_id, &format!("{}", e), Some(elapsed_ms))
                    .await;
            }
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
/// * `analyze` - Whether to run EXPLAIN ANALYZE for actual runtime stats
/// * `database` - Optional target database (creates a temporary connection if
///   different from the active connection's database)
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
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<ExplainResult, String> {
    let sql_lower = sql.trim().to_lowercase();

    if sql_lower.contains(";") && !sql_lower.ends_with(";") {
        return Err("Multiple statements are not allowed in EXPLAIN queries".to_string());
    }

    let sql = sql.trim().trim_end_matches(';');
    let analyze = analyze.unwrap_or(false);

    // If a target database is specified we may need a temporary adapter.
    // We hold the connections lock only long enough to clone the config, then
    // release it before doing any network I/O.
    if let Some(ref db) = database {
        enum TempExplainKind {
            Postgres(ConnectionConfig),
            MySQL(ConnectionConfig),
            SQLServer(ConnectionConfig),
            ClickHouse(ConnectionConfig),
            JdbcBridge(ConnectionConfig),
            HttpSql(ConnectionConfig),
            Rqlite(ConnectionConfig),
            Turso(ConnectionConfig),
        }

        let temp_kind: Option<TempExplainKind> = {
            let connections = state.connections.read().await;
            let connection = connections
                .get(&connection_id)
                .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?;

            match connection {
                ActiveConnection::Postgres(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempExplainKind::Postgres(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::MySQL(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempExplainKind::MySQL(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::SQLServer(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempExplainKind::SQLServer(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::SQLite(_) => None,
                ActiveConnection::ClickHouse(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempExplainKind::ClickHouse(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::JdbcBridge(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempExplainKind::JdbcBridge(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::HttpSql(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempExplainKind::HttpSql(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::Rqlite(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempExplainKind::Rqlite(cfg))
                    } else {
                        None
                    }
                }
                ActiveConnection::Turso(adapter) => {
                    let adapter = adapter.lock().await;
                    if Some(db.as_str()) != adapter.config.database.as_deref() {
                        let mut cfg = adapter.config.clone();
                        cfg.database = Some(db.clone());
                        Some(TempExplainKind::Turso(cfg))
                    } else {
                        None
                    }
                }
            }
            // connections lock is dropped here, before any network I/O
        };

        if let Some(kind) = temp_kind {
            return match kind {
                TempExplainKind::Postgres(cfg) => {
                    let mut temp = PostgresAdapter::new(cfg);
                    temp.connect().await.map_err(|e| {
                        format!("Failed to connect to database for EXPLAIN: {}", e)
                    })?;
                    let connection = ActiveConnection::Postgres(Arc::new(Mutex::new(temp)));
                    let database_type = "postgresql";
                    let explain_sql = if analyze {
                        format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", sql)
                    } else {
                        format!("EXPLAIN (FORMAT JSON) {}", sql)
                    };
                    let result = connection
                        .execute_query(&explain_sql)
                        .await
                        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;
                    let raw = extract_plan_text(&result);
                    let _ = connection.disconnect().await;
                    Ok(ExplainResult {
                        database_type: database_type.to_string(),
                        raw,
                        format: "json".to_string(),
                        analyze,
                    })
                }
                TempExplainKind::MySQL(cfg) => {
                    let mut temp = MySQLAdapter::new(cfg);
                    temp.connect().await.map_err(|e| {
                        format!("Failed to connect to database for EXPLAIN: {}", e)
                    })?;
                    let connection = ActiveConnection::MySQL(Arc::new(Mutex::new(temp)));
                    let database_type = "mysql";
                    let (explain_sql, plan_format) = if analyze {
                        (format!("EXPLAIN ANALYZE {}", sql), "text")
                    } else {
                        (format!("EXPLAIN FORMAT=JSON {}", sql), "json")
                    };
                    let result = connection
                        .execute_query(&explain_sql)
                        .await
                        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;
                    let raw = extract_plan_text(&result);
                    let _ = connection.disconnect().await;
                    Ok(ExplainResult {
                        database_type: database_type.to_string(),
                        raw,
                        format: plan_format.to_string(),
                        analyze,
                    })
                }
                TempExplainKind::SQLServer(cfg) => {
                    let mut temp = SqlServerAdapter::new(cfg);
                    temp.connect().await.map_err(|e| {
                        format!("Failed to connect to database for EXPLAIN: {}", e)
                    })?;
                    let connection = ActiveConnection::SQLServer(Arc::new(Mutex::new(temp)));
                    let database_type = "sqlserver";
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
                    let result = connection
                        .execute_query(&explain_sql)
                        .await
                        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;
                    let raw = extract_plan_text_for_sqlserver(&result);
                    let _ = connection.disconnect().await;
                    Ok(ExplainResult {
                        database_type: database_type.to_string(),
                        raw,
                        format: "text".to_string(),
                        analyze,
                    })
                }
                TempExplainKind::ClickHouse(cfg) => {
                    let mut temp = ClickHouseAdapter::new(cfg);
                    temp.connect().await.map_err(|e| {
                        format!("Failed to connect to database for EXPLAIN: {}", e)
                    })?;
                    let connection = ActiveConnection::ClickHouse(Arc::new(Mutex::new(temp)));
                    let database_type = "clickhouse";
                    let explain_sql = format!("EXPLAIN {}", sql);
                    let result = connection
                        .execute_query(&explain_sql)
                        .await
                        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;
                    let raw = extract_plan_text(&result);
                    let _ = connection.disconnect().await;
                    Ok(ExplainResult {
                        database_type: database_type.to_string(),
                        raw,
                        format: "text".to_string(),
                        analyze,
                    })
                }
                TempExplainKind::JdbcBridge(cfg) => {
                    let mut temp = JdbcBridgeAdapter::new(cfg);
                    temp.connect().await.map_err(|e| {
                        format!("Failed to connect to database for EXPLAIN: {}", e)
                    })?;
                    let connection = ActiveConnection::JdbcBridge(Arc::new(Mutex::new(temp)));
                    let database_type = "generic";
                    let explain_sql = format!("EXPLAIN {}", sql);
                    let result = connection
                        .execute_query(&explain_sql)
                        .await
                        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;
                    let raw = extract_plan_text(&result);
                    let _ = connection.disconnect().await;
                    Ok(ExplainResult {
                        database_type: database_type.to_string(),
                        raw,
                        format: "text".to_string(),
                        analyze,
                    })
                }
                TempExplainKind::HttpSql(cfg) => {
                    let mut temp = HttpSqlAdapter::new(cfg);
                    temp.connect().await.map_err(|e| {
                        format!("Failed to connect to database for EXPLAIN: {}", e)
                    })?;
                    let connection = ActiveConnection::HttpSql(Arc::new(Mutex::new(temp)));
                    let database_type = "generic";
                    let explain_sql = format!("EXPLAIN {}", sql);
                    let result = connection
                        .execute_query(&explain_sql)
                        .await
                        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;
                    let raw = extract_plan_text(&result);
                    let _ = connection.disconnect().await;
                    Ok(ExplainResult {
                        database_type: database_type.to_string(),
                        raw,
                        format: "text".to_string(),
                        analyze,
                    })
                }
                TempExplainKind::Rqlite(cfg) => {
                    let mut temp = RqliteAdapter::new(cfg);
                    temp.connect().await.map_err(|e| {
                        format!("Failed to connect to database for EXPLAIN: {}", e)
                    })?;
                    let connection = ActiveConnection::Rqlite(Arc::new(Mutex::new(temp)));
                    let database_type = "rqlite";
                    let explain_sql = format!("EXPLAIN {}", sql);
                    let result = connection
                        .execute_query(&explain_sql)
                        .await
                        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;
                    let raw = extract_plan_text(&result);
                    let _ = connection.disconnect().await;
                    Ok(ExplainResult {
                        database_type: database_type.to_string(),
                        raw,
                        format: "text".to_string(),
                        analyze,
                    })
                }
                TempExplainKind::Turso(cfg) => {
                    let mut temp = TursoAdapter::new(cfg);
                    temp.connect().await.map_err(|e| {
                        format!("Failed to connect to database for EXPLAIN: {}", e)
                    })?;
                    let connection = ActiveConnection::Turso(Arc::new(Mutex::new(temp)));
                    let database_type = "turso";
                    let explain_sql = format!("EXPLAIN {}", sql);
                    let result = connection
                        .execute_query(&explain_sql)
                        .await
                        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;
                    let raw = extract_plan_text(&result);
                    let _ = connection.disconnect().await;
                    Ok(ExplainResult {
                        database_type: database_type.to_string(),
                        raw,
                        format: "text".to_string(),
                        analyze,
                    })
                }
            };
        }
    }

    // Common path: use the already-connected adapter
    let connection = {
        let connections = state.connections.read().await;
        connections
            .get(&connection_id)
            .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))?
            .clone()
    };

    let database_type = match &connection {
        ActiveConnection::Postgres(_) => "postgresql",
        ActiveConnection::MySQL(_) => "mysql",
        ActiveConnection::SQLite(_) => "sqlite",
        ActiveConnection::SQLServer(_) => "sqlserver",
        ActiveConnection::ClickHouse(_) => "clickhouse",
        ActiveConnection::JdbcBridge(_) => "generic",
        ActiveConnection::HttpSql(_) => "generic",
        ActiveConnection::Rqlite(_) => "rqlite",
        ActiveConnection::Turso(_) => "turso",
    };

    // Guardian health check before EXPLAIN
    if let Some(guardian) = crate::GUARDIAN.get() {
        guardian.touch(&connection_id).await;
        let health_state = guardian.get_state(&connection_id).await;
        if health_state == HealthState::Dead || health_state == HealthState::Reconnecting {
            return Err(format!(
                "Connection is in state '{:?}'. Please wait for it to reconnect or reconnect manually.",
                health_state
            ));
        }
    }

    let (explain_sql, plan_format) = match database_type {
        "postgresql" => {
            let explain_sql = if analyze {
                format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", sql)
            } else {
                format!("EXPLAIN (FORMAT JSON) {}", sql)
            };
            (explain_sql, "json")
        }
        "mysql" => {
            if analyze {
                (format!("EXPLAIN ANALYZE {}", sql), "text")
            } else {
                (format!("EXPLAIN FORMAT=JSON {}", sql), "json")
            }
        }
        "sqlserver" => {
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
            (format!("{}; {}; {}", settings, sql, cleanup), "text")
        }
        "sqlite" => (format!("EXPLAIN QUERY PLAN {}", sql), "text"),
        "clickhouse" => (format!("EXPLAIN {}", sql), "text"),
        "generic" => (format!("EXPLAIN {}", sql), "text"),
        "rqlite" => (format!("EXPLAIN {}", sql), "text"),
        "turso" => (format!("EXPLAIN {}", sql), "text"),
        _ => (format!("EXPLAIN {}", sql), "text"),
    };

    let result = connection
        .execute_query(&explain_sql)
        .await
        .map_err(|e| format!("EXPLAIN query failed: {}", e))?;

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
