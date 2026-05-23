use crate::database::{DatabaseAdapter, DatabaseType, PostgresAdapter, SqlServerAdapter};
use crate::state::{ActiveConnection, AppState};
use crate::transfer::{
    auto_map_columns, detect_file, execute_export, execute_import, execute_migration,
    generate_ddl_for_engine, load_profiles, preview_export, preview_import, preview_migration,
    restore_csv_file_with_progress, restore_sql_file_with_progress,
    restore_xlsx_file_with_progress, save_profiles, DdlRequest, ExportFormat, ExportPreview,
    ExportRequest, ExportSource, FileDetectionResult, ImportFormat, ImportRequest, JobEventPayload,
    JobProgress, MigrationPreview, MigrationRequest, MigrationTablePlan, ObjectSelection,
    RestoreOptions, RestoreStats, TransferError, TransferJob, TransferJobStatus, TransferProfile,
    TransferProfileKind, TransferResult, TransferScope,
};
use chrono::Utc;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use uuid::Uuid;

const MYSQL_SYSTEM_DATABASES: &[&str] =
    &["mysql", "information_schema", "performance_schema", "sys"];
// PostgreSQL: template databases are not openable and should always be skipped.
// Keep `postgres` available because users may intentionally store data there.
const POSTGRES_SYSTEM_DATABASES: &[&str] = &["template0", "template1"];
const SQLSERVER_SYSTEM_DATABASES: &[&str] = &["master", "msdb", "tempdb", "model"];

#[tauri::command]
pub async fn preview_export_data(
    request: ExportRequest,
    preview_rows: u32,
    state: State<'_, AppState>,
) -> Result<ExportPreview, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&request.connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            preview_export(&*adapter, request, preview_rows).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            preview_export(&*adapter, request, preview_rows).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            preview_export(&*adapter, request, preview_rows).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            preview_export(&*adapter, request, preview_rows).await
        }
    }
}

#[tauri::command]
pub async fn execute_export_data(
    request: ExportRequest,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&request.connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            execute_export(&*adapter, request, &app_handle).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            execute_export(&*adapter, request, &app_handle).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            execute_export(&*adapter, request, &app_handle).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            execute_export(&*adapter, request, &app_handle).await
        }
    }
}

#[tauri::command]
pub fn detect_file_format(file_path: String) -> Result<FileDetectionResult, String> {
    detect_file(&file_path)
}

#[tauri::command]
pub fn preview_import_data(
    file_path: String,
    format: ImportFormat,
    preview_rows: u32,
) -> Result<ExportPreview, String> {
    preview_import(&file_path, format, preview_rows)
}

#[tauri::command]
pub async fn execute_import_data(
    request: ImportRequest,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&request.connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            execute_import(&*adapter, request, &app_handle).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            execute_import(&*adapter, request, &app_handle).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            execute_import(&*adapter, request, &app_handle).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            execute_import(&*adapter, request, &app_handle).await
        }
    }
}

#[tauri::command]
pub async fn preview_migration_data(
    request: MigrationRequest,
    state: State<'_, AppState>,
) -> Result<MigrationPreview, String> {
    let connections = state.connections.lock().await;

    let source_connection = connections
        .get(&request.source_connection_id)
        .ok_or_else(|| "No source connection found".to_string())?;

    match source_connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            preview_migration(&*adapter, &request).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            preview_migration(&*adapter, &request).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            preview_migration(&*adapter, &request).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            preview_migration(&*adapter, &request).await
        }
    }
}

#[tauri::command]
pub async fn execute_migration_data(
    request: MigrationRequest,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> {
    let connections = state.connections.lock().await;

    let source_connection = connections
        .get(&request.source_connection_id)
        .ok_or_else(|| "No source connection found".to_string())?;

    let target_connection = connections
        .get(&request.target_connection_id)
        .ok_or_else(|| "No target connection found".to_string())?;

    macro_rules! run_migration {
        ($source_adapter:expr, $target_adapter:expr) => {
            execute_migration(&*$source_adapter, &*$target_adapter, request, &app_handle).await
        };
    }

    match (source_connection, target_connection) {
        (ActiveConnection::Postgres(src), ActiveConnection::Postgres(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::Postgres(src), ActiveConnection::MySQL(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::Postgres(src), ActiveConnection::SQLServer(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::Postgres(src), ActiveConnection::SQLite(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::MySQL(src), ActiveConnection::Postgres(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::MySQL(src), ActiveConnection::MySQL(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::MySQL(src), ActiveConnection::SQLServer(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::MySQL(src), ActiveConnection::SQLite(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLServer(src), ActiveConnection::Postgres(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLServer(src), ActiveConnection::MySQL(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLServer(src), ActiveConnection::SQLServer(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLServer(src), ActiveConnection::SQLite(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLite(src), ActiveConnection::Postgres(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLite(src), ActiveConnection::MySQL(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLite(src), ActiveConnection::SQLServer(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLite(src), ActiveConnection::SQLite(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
    }
}

#[tauri::command]
pub async fn auto_map_migration_columns(
    connection_id: String,
    database: Option<String>,
    schema: Option<String>,
    table: String,
    target_engine: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::transfer::MigrationMapping>, String> {
    use crate::database::DatabaseAdapter;

    let connections = state.connections.lock().await;
    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    let target_db_type = match target_engine.to_lowercase().as_str() {
        "postgresql" | "postgres" => DatabaseType::PostgreSQL,
        "mysql" | "mariadb" => DatabaseType::MySQL,
        "sqlite" => DatabaseType::SQLite,
        "sqlserver" | "mssql" => DatabaseType::SqlServer,
        _ => return Err("Unknown target engine".to_string()),
    };

    macro_rules! fetch_and_map {
        ($adapter:expr) => {{
            let adapter = $adapter.lock().await;
            let columns = adapter
                .list_columns(database.as_deref(), schema.as_deref(), &table)
                .await
                .map_err(|e| e.to_string())?;
            Ok(auto_map_columns(&columns, target_db_type))
        }};
    }

    match connection {
        ActiveConnection::Postgres(adapter) => fetch_and_map!(adapter),
        ActiveConnection::MySQL(adapter) => fetch_and_map!(adapter),
        ActiveConnection::SQLServer(adapter) => fetch_and_map!(adapter),
        ActiveConnection::SQLite(adapter) => fetch_and_map!(adapter),
    }
}

#[tauri::command]
pub async fn generate_ddl_for_objects(
    request: DdlRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&request.connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    let engine = match connection {
        ActiveConnection::Postgres(_) => DatabaseType::PostgreSQL,
        ActiveConnection::MySQL(_) => DatabaseType::MySQL,
        ActiveConnection::SQLServer(_) => DatabaseType::SqlServer,
        ActiveConnection::SQLite(_) => DatabaseType::SQLite,
    };

    async fn collect<A: DatabaseAdapter>(
        adapter: &A,
        request: &DdlRequest,
        database: Option<&str>,
        engine: DatabaseType,
    ) -> Result<String, String> {
        let mut parts: Vec<String> = Vec::with_capacity(request.objects.len());
        for obj in &request.objects {
            let schema = obj.schema.as_deref().or(request.schema.as_deref());
            let columns = adapter
                .list_columns(database, schema, &obj.name)
                .await
                .map_err(|e| e.to_string())?;
            parts.push(generate_ddl_for_engine(
                engine,
                schema,
                &obj.name,
                &columns,
                &request.options,
            ));
        }
        Ok(parts.join("\n\n"))
    }

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            // Postgres rejects cross-database access on the same connection;
            // reconnect to the requested database when it differs from the active one.
            if let Some(db) = request.database.as_deref() {
                if Some(db) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.to_string());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to '{}': {}", db, e))?;
                    return collect(&temp, &request, None, engine).await;
                }
            }
            collect(&*adapter, &request, None, engine).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            collect(&*adapter, &request, request.database.as_deref(), engine).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            if let Some(db) = request.database.as_deref() {
                if Some(db) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.to_string());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to '{}': {}", db, e))?;
                    return collect(&temp, &request, None, engine).await;
                }
            }
            collect(&*adapter, &request, None, engine).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            collect(&*adapter, &request, None, engine).await
        }
    }
}

/// Split a SQL script into individual statements by trailing semicolons on a line.
///
/// Matches the splitter used by the SQL preview path in `import.rs` for consistency.
/// Lines starting with `--` and empty lines are skipped. Multi-line statements are
/// joined until a line ends with `;`.
fn split_sql_statements(content: &str) -> Vec<String> {
    let mut statements: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }
        current.push_str(line);
        current.push('\n');
        if trimmed.ends_with(';') {
            let stmt = current.trim().trim_end_matches(';').trim().to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
        }
    }
    let tail = current.trim().trim_end_matches(';').trim().to_string();
    if !tail.is_empty() {
        statements.push(tail);
    }
    statements
}

/// Execute the contents of a SQL script against the active connection.
///
/// `on_error` accepts "stop" (default), "skipAndContinue", or "rollback".
/// NOTE: "rollback" currently aborts on the first error like "stop". True
/// transactional rollback is not supported here because adapters acquire a
/// fresh pooled connection per `execute_query` call, so a BEGIN/COMMIT issued
/// from this command would not span the executed statements. Wrapping the
/// script in explicit `BEGIN; ... COMMIT;` statements does work because all
/// three live in the same script body and are dispatched as separate calls but
/// still execute on independent sessions — so even author-written transactions
/// must rely on engine-side autocommit semantics. Users needing atomic batches
/// should run them via the Query editor (single session) for now.
#[tauri::command]
pub async fn execute_sql_content(
    connection_id: String,
    database: Option<String>,
    content: String,
    on_error: Option<String>,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    let strategy = on_error.as_deref().unwrap_or("stop");
    let statements = split_sql_statements(&content);
    let started = Instant::now();

    async fn run<A: DatabaseAdapter>(
        adapter: &A,
        statements: &[String],
        strategy: &str,
        started: Instant,
    ) -> TransferResult {
        let total = statements.len() as u64;
        let mut processed: u64 = 0;
        let mut skipped: u64 = 0;
        let mut errors: Vec<TransferError> = Vec::new();
        let mut aborted = false;

        for (idx, stmt) in statements.iter().enumerate() {
            match adapter.execute_query(stmt).await {
                Ok(_) => processed += 1,
                Err(e) => {
                    errors.push(TransferError {
                        row_number: None,
                        statement_number: Some(idx as u64 + 1),
                        message: e.to_string(),
                        sql: Some(stmt.clone()),
                    });
                    if strategy == "skipAndContinue" {
                        skipped += 1;
                    } else {
                        // "stop" and "rollback" both abort on first error.
                        // True multi-statement rollback is unavailable because
                        // each execute_query runs on a fresh pooled session.
                        aborted = true;
                        break;
                    }
                }
            }
        }

        TransferResult {
            success: !aborted && errors.is_empty(),
            total_rows: total,
            processed_rows: processed,
            skipped_rows: skipped,
            error_count: errors.len() as u64,
            duration_ms: started.elapsed().as_millis() as u64,
            output_path: None,
            output_size_bytes: None,
            errors,
        }
    }

    // For Postgres / SQL Server, reconnect when targeting a non-default database.
    let result = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            if let Some(db) = database.as_deref() {
                if Some(db) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.to_string());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to '{}': {}", db, e))?;
                    run(&temp, &statements, strategy, started).await
                } else {
                    run(&*adapter, &statements, strategy, started).await
                }
            } else {
                run(&*adapter, &statements, strategy, started).await
            }
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            if let Some(db) = database.as_deref() {
                // MySQL allows USE <db> on the same connection.
                let _ = adapter.execute_query(&format!("USE `{}`", db)).await;
            }
            run(&*adapter, &statements, strategy, started).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            if let Some(db) = database.as_deref() {
                if Some(db) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(db.to_string());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to '{}': {}", db, e))?;
                    run(&temp, &statements, strategy, started).await
                } else {
                    run(&*adapter, &statements, strategy, started).await
                }
            } else {
                run(&*adapter, &statements, strategy, started).await
            }
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            run(&*adapter, &statements, strategy, started).await
        }
    };

    Ok(result)
}

fn export_extension(format: &ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Jsonl => "jsonl",
        ExportFormat::Sql => "sql",
        ExportFormat::Excel => "xlsx",
    }
}

fn quote_ident_mysql(name: &str) -> Result<String, String> {
    if name.contains('\0') {
        return Err("Invalid identifier: contains null byte".to_string());
    }
    Ok(format!("`{}`", name.replace('`', "``")))
}

fn quote_ident_pg(name: &str) -> Result<String, String> {
    if name.contains('\0') {
        return Err("Invalid identifier: contains null byte".to_string());
    }
    Ok(format!("\"{}\"", name.replace('"', "\"\"")))
}

fn quote_ident_mssql(name: &str) -> Result<String, String> {
    if name.contains('\0') {
        return Err("Invalid identifier: contains null byte".to_string());
    }
    Ok(format!("[{}]", name.replace(']', "]]")))
}

fn normalize_schema(database: &str, schema: &str) -> Option<String> {
    if schema == database || schema == "main" {
        None
    } else {
        Some(schema.to_string())
    }
}

async fn get_connection(
    connection_id: &str,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
) -> Result<ActiveConnection, String> {
    let guard = connections.lock().await;
    guard
        .get(connection_id)
        .cloned()
        .ok_or_else(|| format!("No active connection found for ID '{}'", connection_id))
}

async fn list_databases_for_connection(
    connection_id: &str,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
    exclude_system_databases: bool,
) -> Result<Vec<String>, String> {
    let connection = get_connection(connection_id, connections).await?;

    let (db_type, databases) = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            (
                Some(DatabaseType::PostgreSQL),
                adapter
                    .list_databases()
                    .await
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|db| db.name)
                    .collect::<Vec<_>>(),
            )
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            (
                Some(DatabaseType::MySQL),
                adapter
                    .list_databases()
                    .await
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|db| db.name)
                    .collect::<Vec<_>>(),
            )
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            (
                Some(DatabaseType::SqlServer),
                adapter
                    .list_databases()
                    .await
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|db| db.name)
                    .collect::<Vec<_>>(),
            )
        }
        ActiveConnection::SQLite(_) => (None, vec!["main".to_string()]),
    };

    let databases = if exclude_system_databases {
        filter_system_databases_for_whole_server(db_type, databases)
    } else {
        databases
    };

    Ok(databases)
}

fn should_exclude_system_database(db_type: DatabaseType, database: &str) -> bool {
    let deny_list = match db_type {
        DatabaseType::MySQL => MYSQL_SYSTEM_DATABASES,
        DatabaseType::PostgreSQL => POSTGRES_SYSTEM_DATABASES,
        DatabaseType::SqlServer => SQLSERVER_SYSTEM_DATABASES,
        _ => &[],
    };

    deny_list
        .iter()
        .any(|system_db| database.eq_ignore_ascii_case(system_db))
}

fn filter_system_databases_for_whole_server(
    db_type: Option<DatabaseType>,
    databases: Vec<String>,
) -> Vec<String> {
    match db_type {
        Some(database_type) => databases
            .into_iter()
            .filter(|database| !should_exclude_system_database(database_type, database))
            .collect::<Vec<_>>(),
        None => databases,
    }
}

async fn list_schemas_for_database(
    connection_id: &str,
    database: &str,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
) -> Result<Vec<String>, String> {
    let connection = get_connection(connection_id, connections).await?;

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_schemas(Some(database))
                .await
                .map_err(|e| e.to_string())
        }
        ActiveConnection::MySQL(_) => Ok(vec![database.to_string()]),
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_schemas(Some(database))
                .await
                .map_err(|e| e.to_string())
        }
        ActiveConnection::SQLite(_) => Ok(vec!["main".to_string()]),
    }
}

async fn list_table_names(
    connection_id: &str,
    database: &str,
    schema: Option<&str>,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
) -> Result<Vec<String>, String> {
    let connection = get_connection(connection_id, connections).await?;

    let tables = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(database), schema)
                .await
                .map_err(|e| e.to_string())?
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(database), schema)
                .await
                .map_err(|e| e.to_string())?
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(Some(database), schema)
                .await
                .map_err(|e| e.to_string())?
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_tables(None, None)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    Ok(tables.into_iter().map(|table| table.name).collect())
}

async fn expand_selection(
    connection_id: &str,
    selection: &ObjectSelection,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
) -> Result<Vec<(String, Option<String>, String)>, String> {
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    let mut expanded: Vec<(String, Option<String>, String)> = Vec::new();

    if !selection.tables.is_empty() {
        selection.tables.iter().for_each(|(key, tables)| {
            let (database, schema) = key
                .split_once('.')
                .map(|(db, sch)| (db.to_string(), Some(sch.to_string())))
                .unwrap_or_else(|| (key.clone(), None));

            tables.iter().for_each(|table| {
                let schema_key = schema.clone().unwrap_or_default();
                if seen.insert((database.clone(), schema_key, table.clone())) {
                    expanded.push((database.clone(), schema.clone(), table.clone()));
                }
            });
        });

        return Ok(expanded);
    }

    let mut databases_to_expand = selection.databases.clone();
    if databases_to_expand.is_empty() && !selection.schemas.is_empty() {
        databases_to_expand = selection.schemas.keys().cloned().collect();
    }
    if databases_to_expand.is_empty() {
        databases_to_expand =
            list_databases_for_connection(connection_id, connections, true).await?;
    }

    for database in databases_to_expand {
        let explicit_schemas = selection
            .schemas
            .get(&database)
            .cloned()
            .unwrap_or_default();
        let schemas = if explicit_schemas.is_empty() {
            list_schemas_for_database(connection_id, &database, connections).await?
        } else {
            explicit_schemas
        };

        for schema_name in schemas {
            let tables = list_table_names(
                connection_id,
                &database,
                Some(schema_name.as_str()),
                connections,
            )
            .await?;

            for table in tables {
                let normalized_schema = normalize_schema(&database, &schema_name);
                let schema_key = normalized_schema.clone().unwrap_or_default();
                if seen.insert((database.clone(), schema_key, table.clone())) {
                    expanded.push((database.clone(), normalized_schema, table));
                }
            }
        }
    }

    Ok(expanded)
}

#[allow(clippy::too_many_arguments)]
fn emit_job_event(
    app: &AppHandle,
    job_id: &str,
    status: TransferJobStatus,
    stage: &str,
    current: u64,
    total: u64,
    eta_ms: Option<u64>,
    error: Option<String>,
) -> Result<(), String> {
    let payload = JobEventPayload {
        status,
        progress: JobProgress {
            stage: stage.to_string(),
            current,
            total,
            eta_ms,
        },
        error,
    };

    app.emit_to("main", &format!("transfer://progress/{}", job_id), payload)
        .map_err(|e| e.to_string())
}

async fn list_columns_for_table(
    connection_id: &str,
    database: &str,
    schema: Option<&str>,
    table: &str,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
) -> Result<Vec<String>, String> {
    let connection = get_connection(connection_id, connections).await?;

    let columns = match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            if Some(database) != adapter.config.database.as_deref() {
                let mut temp_config = adapter.config.clone();
                drop(adapter);
                temp_config.database = Some(database.to_string());
                let mut temp = PostgresAdapter::new(temp_config);
                temp.connect()
                    .await
                    .map_err(|e| format!("Failed to connect to '{}': {}", database, e))?;
                temp.list_columns(None, schema, table)
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                adapter
                    .list_columns(None, schema, table)
                    .await
                    .map_err(|e| e.to_string())?
            }
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(Some(database), schema, table)
                .await
                .map_err(|e| e.to_string())?
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            if Some(database) != adapter.config.database.as_deref() {
                let mut temp_config = adapter.config.clone();
                drop(adapter);
                temp_config.database = Some(database.to_string());
                let mut temp = SqlServerAdapter::new(temp_config);
                temp.connect()
                    .await
                    .map_err(|e| format!("Failed to connect to '{}': {}", database, e))?;
                temp.list_columns(None, schema, table)
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                adapter
                    .list_columns(None, schema, table)
                    .await
                    .map_err(|e| e.to_string())?
            }
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .list_columns(Some(database), schema, table)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    Ok(columns.into_iter().map(|column| column.name).collect())
}

async fn execute_export_request(
    request: ExportRequest,
    target_database: Option<&str>,
    app: &AppHandle,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
) -> Result<TransferResult, String> {
    let request_database = request.database.clone();
    let effective_database = target_database.or(request_database.as_deref());

    let connection = get_connection(&request.connection_id, connections).await?;

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            if let Some(database) = effective_database {
                if Some(database) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(database.to_string());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to '{}': {}", database, e))?;
                    return execute_export(&temp, request, app).await;
                }
            }

            execute_export(&*adapter, request, app).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            let mut adjusted_request = request;
            if let Some(database) = effective_database {
                adjusted_request.database = Some(database.to_string());
            }
            execute_export(&*adapter, adjusted_request, app).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            if let Some(database) = effective_database {
                if Some(database) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(database.to_string());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to '{}': {}", database, e))?;
                    return execute_export(&temp, request, app).await;
                }
            }

            execute_export(&*adapter, request, app).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            execute_export(&*adapter, request, app).await
        }
    }
}

async fn execute_import_request(
    request: ImportRequest,
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<TransferResult, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(&request.connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            execute_import(&*adapter, request, app).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            execute_import(&*adapter, request, app).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            execute_import(&*adapter, request, app).await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            execute_import(&*adapter, request, app).await
        }
    }
}

async fn ensure_target_database(
    target_connection_id: &str,
    database_name: &str,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
) -> Result<(), String> {
    let connection = get_connection(target_connection_id, connections)
        .await
        .map_err(|_| "No target connection found".to_string())?;

    match connection {
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            // MySQL uses CREATE DATABASE IF NOT EXISTS and continues with the same
            // server connection, so an additional per-database probe is unnecessary here.
            adapter
                .execute_query(&format!(
                    "CREATE DATABASE IF NOT EXISTS {}",
                    quote_ident_mysql(database_name)?
                ))
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let mut system_config = adapter.config.clone();
            let mut target_config = adapter.config.clone();
            drop(adapter);
            system_config.database = Some("postgres".to_string());
            target_config.database = Some(database_name.to_string());

            let mut system_adapter = PostgresAdapter::new(system_config);
            system_adapter
                .connect()
                .await
                .map_err(|e| format!("Failed to connect to 'postgres': {}", e))?;

            let exists = system_adapter
                .list_databases()
                .await
                .map_err(|e| e.to_string())?
                .iter()
                .any(|db| db.name == database_name);
            if !exists {
                system_adapter
                    .execute_query(&format!(
                        "CREATE DATABASE {}",
                        quote_ident_pg(database_name)?
                    ))
                    .await
                    .map_err(|e| e.to_string())?;
            }

            let mut target_adapter = PostgresAdapter::new(target_config);
            target_adapter
                .connect()
                .await
                .map_err(|e| target_database_inaccessible_error(database_name, &e.to_string()))?;
            target_adapter
                .execute_query("SELECT 1")
                .await
                .map_err(|e| target_database_inaccessible_error(database_name, &e.to_string()))?;

            Ok(())
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            let mut target_config = adapter.config.clone();
            target_config.database = Some(database_name.to_string());
            let exists = adapter
                .list_databases()
                .await
                .map_err(|e| e.to_string())?
                .iter()
                .any(|db| db.name == database_name);
            if !exists {
                adapter
                    .execute_query(&format!(
                        "CREATE DATABASE {}",
                        quote_ident_mssql(database_name)?
                    ))
                    .await
                    .map_err(|e| e.to_string())?;
            }

            drop(adapter);

            let mut target_adapter = SqlServerAdapter::new(target_config);
            target_adapter
                .connect()
                .await
                .map_err(|e| target_database_inaccessible_error(database_name, &e.to_string()))?;
            target_adapter
                .execute_query("SELECT 1")
                .await
                .map_err(|e| target_database_inaccessible_error(database_name, &e.to_string()))?;

            Ok(())
        }
        _ => Ok(()),
    }
}

fn target_database_inaccessible_error(database_name: &str, error: &str) -> String {
    format!(
        "Target database '{}' is inaccessible: {}",
        database_name, error
    )
}

async fn execute_migration_request(
    request: MigrationRequest,
    source_database: Option<&str>,
    target_database: Option<&str>,
    app: &AppHandle,
    connections: &Arc<Mutex<HashMap<String, ActiveConnection>>>,
) -> Result<TransferResult, String> {
    let request_source_database = request.source_database.clone();
    let request_target_database = request.target_database.clone();
    let effective_source_database = source_database.or(request_source_database.as_deref());
    let effective_target_database = target_database.or(request_target_database.as_deref());

    let source_connection = get_connection(&request.source_connection_id, connections)
        .await
        .map_err(|_| "No source connection found".to_string())?;

    let target_connection = get_connection(&request.target_connection_id, connections)
        .await
        .map_err(|_| "No target connection found".to_string())?;

    macro_rules! run_migration {
        ($source_adapter:expr, $target_adapter:expr) => {
            execute_migration(&*$source_adapter, &*$target_adapter, request, app).await
        };
    }

    match (&source_connection, &target_connection) {
        (ActiveConnection::Postgres(src), ActiveConnection::Postgres(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(source_db) = effective_source_database {
                if Some(source_db) != src.config.database.as_deref() {
                    let mut src_config = src.config.clone();
                    drop(src);
                    src_config.database = Some(source_db.to_string());
                    let mut src_temp = PostgresAdapter::new(src_config);
                    src_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to source '{}': {}", source_db, e)
                    })?;

                    if let Some(target_db) = effective_target_database {
                        if Some(target_db) != tgt.config.database.as_deref() {
                            let mut tgt_config = tgt.config.clone();
                            drop(tgt);
                            tgt_config.database = Some(target_db.to_string());
                            let mut tgt_temp = PostgresAdapter::new(tgt_config);
                            tgt_temp.connect().await.map_err(|e| {
                                format!("Failed to connect to target '{}': {}", target_db, e)
                            })?;
                            return execute_migration(&src_temp, &tgt_temp, request, app).await;
                        }
                    }

                    return execute_migration(&src_temp, &*tgt, request, app).await;
                }
            }

            if let Some(target_db) = effective_target_database {
                if Some(target_db) != tgt.config.database.as_deref() {
                    let mut tgt_config = tgt.config.clone();
                    drop(tgt);
                    tgt_config.database = Some(target_db.to_string());
                    let mut tgt_temp = PostgresAdapter::new(tgt_config);
                    tgt_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to target '{}': {}", target_db, e)
                    })?;
                    return execute_migration(&*src, &tgt_temp, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::Postgres(src), ActiveConnection::MySQL(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(source_db) = effective_source_database {
                if Some(source_db) != src.config.database.as_deref() {
                    let mut src_config = src.config.clone();
                    drop(src);
                    src_config.database = Some(source_db.to_string());
                    let mut src_temp = PostgresAdapter::new(src_config);
                    src_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to source '{}': {}", source_db, e)
                    })?;
                    return execute_migration(&src_temp, &*tgt, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::Postgres(src), ActiveConnection::SQLServer(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(source_db) = effective_source_database {
                if Some(source_db) != src.config.database.as_deref() {
                    let mut src_config = src.config.clone();
                    drop(src);
                    src_config.database = Some(source_db.to_string());
                    let mut src_temp = PostgresAdapter::new(src_config);
                    src_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to source '{}': {}", source_db, e)
                    })?;

                    if let Some(target_db) = effective_target_database {
                        if Some(target_db) != tgt.config.database.as_deref() {
                            let mut tgt_config = tgt.config.clone();
                            drop(tgt);
                            tgt_config.database = Some(target_db.to_string());
                            let mut tgt_temp = SqlServerAdapter::new(tgt_config);
                            tgt_temp.connect().await.map_err(|e| {
                                format!("Failed to connect to target '{}': {}", target_db, e)
                            })?;
                            return execute_migration(&src_temp, &tgt_temp, request, app).await;
                        }
                    }

                    return execute_migration(&src_temp, &*tgt, request, app).await;
                }
            }

            if let Some(target_db) = effective_target_database {
                if Some(target_db) != tgt.config.database.as_deref() {
                    let mut tgt_config = tgt.config.clone();
                    drop(tgt);
                    tgt_config.database = Some(target_db.to_string());
                    let mut tgt_temp = SqlServerAdapter::new(tgt_config);
                    tgt_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to target '{}': {}", target_db, e)
                    })?;
                    return execute_migration(&*src, &tgt_temp, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::Postgres(src), ActiveConnection::SQLite(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(source_db) = effective_source_database {
                if Some(source_db) != src.config.database.as_deref() {
                    let mut src_config = src.config.clone();
                    drop(src);
                    src_config.database = Some(source_db.to_string());
                    let mut src_temp = PostgresAdapter::new(src_config);
                    src_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to source '{}': {}", source_db, e)
                    })?;
                    return execute_migration(&src_temp, &*tgt, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::MySQL(src), ActiveConnection::Postgres(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(target_db) = effective_target_database {
                if Some(target_db) != tgt.config.database.as_deref() {
                    let mut tgt_config = tgt.config.clone();
                    drop(tgt);
                    tgt_config.database = Some(target_db.to_string());
                    let mut tgt_temp = PostgresAdapter::new(tgt_config);
                    tgt_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to target '{}': {}", target_db, e)
                    })?;
                    return execute_migration(&*src, &tgt_temp, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::MySQL(src), ActiveConnection::MySQL(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::MySQL(src), ActiveConnection::SQLServer(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(target_db) = effective_target_database {
                if Some(target_db) != tgt.config.database.as_deref() {
                    let mut tgt_config = tgt.config.clone();
                    drop(tgt);
                    tgt_config.database = Some(target_db.to_string());
                    let mut tgt_temp = SqlServerAdapter::new(tgt_config);
                    tgt_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to target '{}': {}", target_db, e)
                    })?;
                    return execute_migration(&*src, &tgt_temp, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::MySQL(src), ActiveConnection::SQLite(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLServer(src), ActiveConnection::Postgres(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(source_db) = effective_source_database {
                if Some(source_db) != src.config.database.as_deref() {
                    let mut src_config = src.config.clone();
                    drop(src);
                    src_config.database = Some(source_db.to_string());
                    let mut src_temp = SqlServerAdapter::new(src_config);
                    src_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to source '{}': {}", source_db, e)
                    })?;

                    if let Some(target_db) = effective_target_database {
                        if Some(target_db) != tgt.config.database.as_deref() {
                            let mut tgt_config = tgt.config.clone();
                            drop(tgt);
                            tgt_config.database = Some(target_db.to_string());
                            let mut tgt_temp = PostgresAdapter::new(tgt_config);
                            tgt_temp.connect().await.map_err(|e| {
                                format!("Failed to connect to target '{}': {}", target_db, e)
                            })?;
                            return execute_migration(&src_temp, &tgt_temp, request, app).await;
                        }
                    }

                    return execute_migration(&src_temp, &*tgt, request, app).await;
                }
            }

            if let Some(target_db) = effective_target_database {
                if Some(target_db) != tgt.config.database.as_deref() {
                    let mut tgt_config = tgt.config.clone();
                    drop(tgt);
                    tgt_config.database = Some(target_db.to_string());
                    let mut tgt_temp = PostgresAdapter::new(tgt_config);
                    tgt_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to target '{}': {}", target_db, e)
                    })?;
                    return execute_migration(&*src, &tgt_temp, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLServer(src), ActiveConnection::MySQL(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(source_db) = effective_source_database {
                if Some(source_db) != src.config.database.as_deref() {
                    let mut src_config = src.config.clone();
                    drop(src);
                    src_config.database = Some(source_db.to_string());
                    let mut src_temp = SqlServerAdapter::new(src_config);
                    src_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to source '{}': {}", source_db, e)
                    })?;
                    return execute_migration(&src_temp, &*tgt, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLServer(src), ActiveConnection::SQLServer(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(source_db) = effective_source_database {
                if Some(source_db) != src.config.database.as_deref() {
                    let mut src_config = src.config.clone();
                    drop(src);
                    src_config.database = Some(source_db.to_string());
                    let mut src_temp = SqlServerAdapter::new(src_config);
                    src_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to source '{}': {}", source_db, e)
                    })?;

                    if let Some(target_db) = effective_target_database {
                        if Some(target_db) != tgt.config.database.as_deref() {
                            let mut tgt_config = tgt.config.clone();
                            drop(tgt);
                            tgt_config.database = Some(target_db.to_string());
                            let mut tgt_temp = SqlServerAdapter::new(tgt_config);
                            tgt_temp.connect().await.map_err(|e| {
                                format!("Failed to connect to target '{}': {}", target_db, e)
                            })?;
                            return execute_migration(&src_temp, &tgt_temp, request, app).await;
                        }
                    }

                    return execute_migration(&src_temp, &*tgt, request, app).await;
                }
            }

            if let Some(target_db) = effective_target_database {
                if Some(target_db) != tgt.config.database.as_deref() {
                    let mut tgt_config = tgt.config.clone();
                    drop(tgt);
                    tgt_config.database = Some(target_db.to_string());
                    let mut tgt_temp = SqlServerAdapter::new(tgt_config);
                    tgt_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to target '{}': {}", target_db, e)
                    })?;
                    return execute_migration(&*src, &tgt_temp, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLServer(src), ActiveConnection::SQLite(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;

            if let Some(source_db) = effective_source_database {
                if Some(source_db) != src.config.database.as_deref() {
                    let mut src_config = src.config.clone();
                    drop(src);
                    src_config.database = Some(source_db.to_string());
                    let mut src_temp = SqlServerAdapter::new(src_config);
                    src_temp.connect().await.map_err(|e| {
                        format!("Failed to connect to source '{}': {}", source_db, e)
                    })?;
                    return execute_migration(&src_temp, &*tgt, request, app).await;
                }
            }

            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLite(src), ActiveConnection::Postgres(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLite(src), ActiveConnection::MySQL(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLite(src), ActiveConnection::SQLServer(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
        (ActiveConnection::SQLite(src), ActiveConnection::SQLite(tgt)) => {
            let src = src.lock().await;
            let tgt = tgt.lock().await;
            run_migration!(src, tgt)
        }
    }
}

fn collect_profile_options(profile_options: &JsonValue) -> HashMap<String, JsonValue> {
    profile_options
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn build_migration_request(
    source_connection_id: String,
    target_connection_id: String,
    database: String,
    schema: Option<String>,
    table_names: Vec<String>,
    mappings: HashMap<String, Vec<crate::transfer::MigrationMapping>>,
    options: &JsonValue,
) -> MigrationRequest {
    let options_map = collect_profile_options(options);
    let batch_size = options_map
        .get("batchSize")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32)
        .unwrap_or(5000);

    let on_error = options_map
        .get("onError")
        .and_then(|value| value.as_str())
        .map(|value| match value {
            "skipTable" => crate::transfer::MigrationErrorStrategy::SkipTable,
            "abort" => crate::transfer::MigrationErrorStrategy::Abort,
            _ => crate::transfer::MigrationErrorStrategy::SkipRow,
        })
        .unwrap_or(crate::transfer::MigrationErrorStrategy::SkipRow);

    let create_tables = options_map
        .get("createTables")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);

    let drop_tables = options_map
        .get("dropTables")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let migrate_indexes = options_map
        .get("migrateIndexes")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let migrate_foreign_keys = options_map
        .get("migrateForeignKeys")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let migrate_constraints = options_map
        .get("migrateConstraints")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);

    let disable_fk_checks = options_map
        .get("disableFkChecks")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let table_plans = table_names
        .into_iter()
        .map(|table| MigrationTablePlan {
            source_table: table.clone(),
            target_table: table.clone(),
            column_mappings: mappings.get(&table).cloned().unwrap_or_default(),
        })
        .collect();

    MigrationRequest {
        source_connection_id,
        source_database: Some(database.clone()),
        source_schema: schema.clone(),
        target_connection_id,
        target_database: Some(database),
        target_schema: schema,
        table_plans,
        batch_size,
        on_error,
        create_tables,
        drop_tables,
        migrate_indexes,
        migrate_foreign_keys,
        migrate_constraints,
        disable_fk_checks,
    }
}

fn profile_to_transfer_job(profile: &TransferProfile) -> TransferJob {
    TransferJob {
        id: Uuid::new_v4().to_string(),
        name: profile.name.clone(),
        kind: profile.kind.clone(),
        scope: profile.scope.clone(),
        connection_id: profile.connection_id.clone(),
        status: TransferJobStatus::Queued,
        progress: JobProgress {
            stage: "queued".to_string(),
            current: 0,
            total: 0,
            eta_ms: None,
        },
        started_at: Utc::now().timestamp_millis(),
        finished_at: None,
        error: None,
    }
}

fn summarize_migration_outcome(
    succeeded_tables: u64,
    failed_tables: u64,
    failures: &[String],
) -> (TransferJobStatus, Option<String>) {
    let summary = if failures.is_empty() {
        None
    } else {
        Some(format!(
            "Migration summary: {} succeeded, {} failed [{}]",
            succeeded_tables,
            failed_tables,
            failures.join("; ")
        ))
    };

    if succeeded_tables > 0 {
        (TransferJobStatus::Completed, summary)
    } else {
        (
            TransferJobStatus::Failed,
            summary.or_else(|| Some("Migration failed".to_string())),
        )
    }
}

fn summarize_backup_outcome(
    succeeded_tables: u64,
    failed_tables: u64,
    failures: &[String],
) -> (TransferJobStatus, Option<String>) {
    let summary = if failures.is_empty() {
        None
    } else {
        Some(format!(
            "Backup summary: {} succeeded, {} failed [{}]",
            succeeded_tables,
            failed_tables,
            failures.join("; ")
        ))
    };

    if succeeded_tables > 0 {
        (TransferJobStatus::Completed, summary)
    } else {
        (
            TransferJobStatus::Failed,
            summary.or_else(|| Some("Backup failed".to_string())),
        )
    }
}

fn summarize_restore_outcome(stats: &RestoreStats) -> (TransferJobStatus, Option<String>) {
    let failed_units = stats
        .statements_total
        .saturating_sub(stats.statements_succeeded);

    let summary = if stats.errors.is_empty() {
        None
    } else {
        Some(format!(
            "Restore summary: {} succeeded, {} failed [{}]",
            stats.statements_succeeded,
            failed_units,
            stats.errors.join("; ")
        ))
    };

    if stats.statements_succeeded > 0 {
        (TransferJobStatus::Completed, summary)
    } else {
        (
            TransferJobStatus::Failed,
            summary.or_else(|| Some("Restore failed".to_string())),
        )
    }
}

async fn restore_with_connection(
    connection: ActiveConnection,
    target_database: Option<String>,
    file_path: String,
    file_format: String,
    target_table: Option<String>,
    drop_target_first: bool,
    on_progress: impl FnMut(u64, u64),
) -> Result<RestoreStats, String> {
    let format = file_format.to_lowercase();
    let options = RestoreOptions::default();

    fn split_target_table(target_table: &str) -> (Option<String>, String) {
        target_table
            .split_once('.')
            .map(|(schema, table)| (Some(schema.to_string()), table.to_string()))
            .unwrap_or_else(|| (None, target_table.to_string()))
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_restore<A: DatabaseAdapter>(
        adapter: &A,
        target_database: Option<&str>,
        file_path: &str,
        format: &str,
        target_table: Option<&str>,
        _drop_target_first: bool,
        options: &RestoreOptions,
        mut on_progress: impl FnMut(u64, u64),
    ) -> Result<RestoreStats, String> {
        if let Some(database) = target_database {
            match adapter.get_config().db_type {
                DatabaseType::MySQL => {
                    adapter
                        .execute_query(&format!("USE `{}`", database.replace('`', "``")))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                DatabaseType::SqlServer => {
                    adapter
                        .execute_query(&format!("USE [{}]", database.replace(']', "]]")))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                _ => {}
            }
        }

        match format {
            "sql" => {
                restore_sql_file_with_progress(adapter, file_path, options, |current, total| {
                    on_progress(current, total)
                })
                .await
                .map_err(|e| e.to_string())
            }
            "csv" => {
                let table = target_table
                    .ok_or_else(|| "targetTable is required for csv restore format".to_string())?;

                let (target_schema, target_table_name) = split_target_table(table);

                restore_csv_file_with_progress(
                    adapter,
                    file_path,
                    target_schema.as_deref(),
                    &target_table_name,
                    options,
                    &mut on_progress,
                )
                .await
                .map_err(|e| e.to_string())
            }
            "xlsx" | "excel" => {
                let table = target_table
                    .ok_or_else(|| "targetTable is required for xlsx restore format".to_string())?;

                let (target_schema, target_table_name) = split_target_table(table);

                restore_xlsx_file_with_progress(
                    adapter,
                    file_path,
                    target_schema.as_deref(),
                    &target_table_name,
                    options,
                    &mut on_progress,
                )
                .await
                .map_err(|e| e.to_string())
            }
            _ => Err(format!(
                "Unsupported restore file format '{}'. Use sql, csv, xlsx, or excel.",
                format
            )),
        }
    }

    match connection {
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            if let Some(database) = target_database.as_deref() {
                if Some(database) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(database.to_string());
                    let mut temp = PostgresAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to '{}': {}", database, e))?;
                    return run_restore(
                        &temp,
                        None,
                        &file_path,
                        &format,
                        target_table.as_deref(),
                        drop_target_first,
                        &options,
                        on_progress,
                    )
                    .await;
                }
            }

            run_restore(
                &*adapter,
                None,
                &file_path,
                &format,
                target_table.as_deref(),
                drop_target_first,
                &options,
                on_progress,
            )
            .await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            run_restore(
                &*adapter,
                target_database.as_deref(),
                &file_path,
                &format,
                target_table.as_deref(),
                drop_target_first,
                &options,
                on_progress,
            )
            .await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
            if let Some(database) = target_database.as_deref() {
                if Some(database) != adapter.config.database.as_deref() {
                    let mut temp_config = adapter.config.clone();
                    drop(adapter);
                    temp_config.database = Some(database.to_string());
                    let mut temp = SqlServerAdapter::new(temp_config);
                    temp.connect()
                        .await
                        .map_err(|e| format!("Failed to connect to '{}': {}", database, e))?;
                    return run_restore(
                        &temp,
                        target_database.as_deref(),
                        &file_path,
                        &format,
                        target_table.as_deref(),
                        drop_target_first,
                        &options,
                        on_progress,
                    )
                    .await;
                }
            }

            run_restore(
                &*adapter,
                target_database.as_deref(),
                &file_path,
                &format,
                target_table.as_deref(),
                drop_target_first,
                &options,
                on_progress,
            )
            .await
        }
        ActiveConnection::SQLite(adapter) => {
            let adapter = adapter.lock().await;
            run_restore(
                &*adapter,
                None,
                &file_path,
                &format,
                target_table.as_deref(),
                drop_target_first,
                &options,
                on_progress,
            )
            .await
        }
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn restore_backup(
    connection_id: String,
    target_database: Option<String>,
    file_path: String,
    file_format: String,
    target_table: Option<String>,
    drop_target_first: bool,
    job_id: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let job_id = job_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let job_id_clone = job_id.clone();
    let app_clone = app.clone();
    let connections = state.connections.clone();

    tokio::spawn(async move {
        let _ = emit_job_event(
            &app_clone,
            &job_id_clone,
            TransferJobStatus::Queued,
            "queued",
            0,
            0,
            None,
            None,
        );

        let connection = match get_connection(&connection_id, &connections).await {
            Ok(connection) => connection,
            Err(error) => {
                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    TransferJobStatus::Failed,
                    "failed",
                    0,
                    0,
                    None,
                    Some(error),
                );
                return;
            }
        };

        let _ = emit_job_event(
            &app_clone,
            &job_id_clone,
            TransferJobStatus::Running,
            "running",
            0,
            0,
            None,
            None,
        );

        let restore_result = restore_with_connection(
            connection,
            target_database,
            file_path,
            file_format,
            target_table,
            drop_target_first,
            |current, total| {
                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    TransferJobStatus::Running,
                    "processing",
                    current,
                    total,
                    None,
                    None,
                );
            },
        )
        .await;

        match restore_result {
            Ok(stats) => {
                let (status, summary) = summarize_restore_outcome(&stats);
                let stage = if status == TransferJobStatus::Completed {
                    "completed"
                } else {
                    "failed"
                };

                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    status,
                    stage,
                    stats.statements_total,
                    stats.statements_total,
                    None,
                    summary,
                );
            }
            Err(error) => {
                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    TransferJobStatus::Failed,
                    "failed",
                    0,
                    0,
                    None,
                    Some(error),
                );
            }
        }
    });

    Ok(job_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn backup_server(
    connection_id: String,
    selection: ObjectSelection,
    format: ExportFormat,
    destination: String,
    options: JsonValue,
    job_id: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let _ = &options;
    let job_id = job_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let connections = state.connections.clone();

    let app_clone = app.clone();
    let job_id_clone = job_id.clone();
    let connection_id_clone = connection_id.clone();
    let selection_clone = selection.clone();
    let format_clone = format.clone();
    let destination_clone = destination.clone();
    let connections_clone = connections.clone();

    tokio::spawn(async move {
        let started_at = Instant::now();
        let outcome: Result<(TransferJobStatus, Option<String>, u64), String> = async {
            let _ = emit_job_event(
                &app_clone,
                &job_id_clone,
                TransferJobStatus::Queued,
                "queued",
                0,
                0,
                None,
                None,
            );

            let selected_tables =
                expand_selection(&connection_id_clone, &selection_clone, &connections_clone)
                    .await?;

            let total_tables = selected_tables.len() as u64;
            if total_tables == 0 {
                return Ok((
                    TransferJobStatus::Failed,
                    Some("No tables selected for server backup".to_string()),
                    0,
                ));
            }

            let _ = emit_job_event(
                &app_clone,
                &job_id_clone,
                TransferJobStatus::Running,
                "running",
                0,
                total_tables,
                None,
                None,
            );

            let mut failed_tables = 0u64;
            let mut failures: Vec<String> = Vec::new();
            let mut succeeded_tables = 0u64;

            for (index, (database, schema, table)) in selected_tables.iter().enumerate() {
                let db_output_dir = PathBuf::from(&destination_clone).join(database);
                let table_label = format!(
                    "{}.{}.{}",
                    database,
                    schema.clone().unwrap_or_else(|| "default".to_string()),
                    table
                );

                if let Err(error) = fs::create_dir_all(&db_output_dir)
                    .map_err(|e| format!("Failed to create backup directory: {}", e))
                {
                    failed_tables += 1;
                    failures.push(format!("{}: {}", table_label, error));
                    let _ = emit_job_event(
                        &app_clone,
                        &job_id_clone,
                        TransferJobStatus::Running,
                        "processing",
                        index as u64 + 1,
                        total_tables,
                        None,
                        None,
                    );
                    continue;
                }

                let output_path =
                    db_output_dir.join(format!("{}.{}", table, export_extension(&format_clone)));
                let columns = match list_columns_for_table(
                    &connection_id_clone,
                    database,
                    schema.as_deref(),
                    table,
                    &connections_clone,
                )
                .await
                {
                    Ok(columns) => columns,
                    Err(error) => {
                        failed_tables += 1;
                        failures.push(format!("{}: {}", table_label, error));
                        let _ = emit_job_event(
                            &app_clone,
                            &job_id_clone,
                            TransferJobStatus::Running,
                            "processing",
                            index as u64 + 1,
                            total_tables,
                            None,
                            None,
                        );
                        continue;
                    }
                };

                let request = ExportRequest {
                    connection_id: connection_id_clone.clone(),
                    database: Some(database.clone()),
                    schema: schema.clone(),
                    source: ExportSource {
                        table: table.clone(),
                        columns,
                        where_clause: None,
                        order_by: None,
                        limit: None,
                    },
                    format: format_clone.clone(),
                    csv_options: None,
                    jsonl_options: None,
                    sql_options: None,
                    excel_options: None,
                    output_path: output_path.to_string_lossy().to_string(),
                };

                let export_result = execute_export_request(
                    request,
                    Some(database.as_str()),
                    &app_clone,
                    &connections_clone,
                )
                .await;

                match export_result {
                    Ok(result) if result.success => {
                        succeeded_tables += 1;
                    }
                    Ok(result) => {
                        failed_tables += 1;
                        let first_error = result
                            .errors
                            .first()
                            .map(|error| error.message.clone())
                            .unwrap_or_else(|| "Export failed".to_string());
                        failures.push(format!("{}: {}", table_label, first_error));
                    }
                    Err(error) => {
                        failed_tables += 1;
                        failures.push(format!("{}: {}", table_label, error));
                    }
                }

                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    TransferJobStatus::Running,
                    "processing",
                    index as u64 + 1,
                    total_tables,
                    None,
                    None,
                );
            }

            let (status, summary) =
                summarize_backup_outcome(succeeded_tables, failed_tables, &failures);

            Ok((status, summary, total_tables))
        }
        .await;

        match outcome {
            Ok((status, error, total_tables)) => {
                let stage = if status == TransferJobStatus::Completed {
                    "completed"
                } else {
                    "failed"
                };
                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    status,
                    stage,
                    total_tables,
                    total_tables,
                    None,
                    error,
                );
            }
            Err(error) => {
                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    TransferJobStatus::Failed,
                    "failed",
                    0,
                    0,
                    None,
                    Some(error),
                );
            }
        }

        let _ = started_at;
    });

    Ok(job_id)
}

#[tauri::command]
pub async fn migrate_server(
    source_connection_id: String,
    target_connection_id: String,
    selection: ObjectSelection,
    options: JsonValue,
    job_id: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let job_id = job_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let connections = state.connections.clone();

    let app_clone = app.clone();
    let job_id_clone = job_id.clone();
    let source_connection_id_clone = source_connection_id.clone();
    let target_connection_id_clone = target_connection_id.clone();
    let selection_clone = selection.clone();
    let options_clone = options.clone();
    let connections_clone = connections.clone();

    tokio::spawn(async move {
        let outcome: Result<(TransferJobStatus, Option<String>, u64), String> = async {
            let _ = emit_job_event(
                &app_clone,
                &job_id_clone,
                TransferJobStatus::Queued,
                "queued",
                0,
                0,
                None,
                None,
            );

            let selected_tables = expand_selection(
                &source_connection_id_clone,
                &selection_clone,
                &connections_clone,
            )
            .await?;

            let total_tables = selected_tables.len() as u64;
            if total_tables == 0 {
                return Ok((
                    TransferJobStatus::Failed,
                    Some("No tables selected for server migration".to_string()),
                    0,
                ));
            }

            let _ = emit_job_event(
                &app_clone,
                &job_id_clone,
                TransferJobStatus::Running,
                "running",
                0,
                total_tables,
                None,
                None,
            );

            let mut processed_tables = 0u64;
            let mut succeeded_tables = 0u64;
            let mut failed_tables = 0u64;
            let mut failures: Vec<String> = Vec::new();

            for (database, schema, table) in selected_tables {
                if let Err(error) = ensure_target_database(
                    &target_connection_id_clone,
                    &database,
                    &connections_clone,
                )
                .await
                {
                    failed_tables += 1;
                    failures.push(format!(
                        "{}.{}.{}: {}",
                        database,
                        schema.clone().unwrap_or_else(|| "default".to_string()),
                        table,
                        error
                    ));
                    processed_tables += 1;
                    let _ = emit_job_event(
                        &app_clone,
                        &job_id_clone,
                        TransferJobStatus::Running,
                        "processing",
                        processed_tables,
                        total_tables,
                        None,
                        None,
                    );
                    continue;
                }

                let mapping_results = match list_columns_for_table(
                    &source_connection_id_clone,
                    &database,
                    schema.as_deref(),
                    &table,
                    &connections_clone,
                )
                .await
                {
                    Ok(columns) => {
                        let mapping = columns
                            .into_iter()
                            .map(|column| crate::transfer::MigrationMapping {
                                source_column: column.clone(),
                                source_type: "text".to_string(),
                                target_column: column,
                                target_type: "text".to_string(),
                                conversion: crate::transfer::MigrationConversion::Direct,
                            })
                            .collect::<Vec<_>>();

                        HashMap::from([(table.clone(), mapping)])
                    }
                    Err(error) => {
                        failed_tables += 1;
                        failures.push(format!(
                            "{}.{}.{}: {}",
                            database,
                            schema.clone().unwrap_or_else(|| "default".to_string()),
                            table,
                            error
                        ));
                        processed_tables += 1;
                        let _ = emit_job_event(
                            &app_clone,
                            &job_id_clone,
                            TransferJobStatus::Running,
                            "processing",
                            processed_tables,
                            total_tables,
                            None,
                            None,
                        );
                        continue;
                    }
                };

                let request = build_migration_request(
                    source_connection_id_clone.clone(),
                    target_connection_id_clone.clone(),
                    database.clone(),
                    schema.clone(),
                    vec![table.clone()],
                    mapping_results,
                    &options_clone,
                );

                let migration_result = execute_migration_request(
                    request,
                    Some(database.as_str()),
                    Some(database.as_str()),
                    &app_clone,
                    &connections_clone,
                )
                .await;

                match migration_result {
                    Ok(result) if result.success => {
                        succeeded_tables += 1;
                    }
                    Ok(result) => {
                        failed_tables += 1;
                        let first_error = result
                            .errors
                            .first()
                            .map(|error| error.message.clone())
                            .unwrap_or_else(|| "Migration failed".to_string());
                        failures.push(format!(
                            "{}.{}.{}: {}",
                            database,
                            schema.clone().unwrap_or_else(|| "default".to_string()),
                            table,
                            first_error
                        ));
                    }
                    Err(error) => {
                        failed_tables += 1;
                        failures.push(format!(
                            "{}.{}.{}: {}",
                            database,
                            schema.clone().unwrap_or_else(|| "default".to_string()),
                            table,
                            error
                        ));
                    }
                }

                processed_tables += 1;
                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    TransferJobStatus::Running,
                    "processing",
                    processed_tables,
                    total_tables,
                    None,
                    None,
                );
            }

            let (status, summary) =
                summarize_migration_outcome(succeeded_tables, failed_tables, &failures);

            Ok((status, summary, total_tables))
        }
        .await;

        match outcome {
            Ok((status, error, total_tables)) => {
                let stage = if status == TransferJobStatus::Completed {
                    "completed"
                } else {
                    "failed"
                };
                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    status,
                    stage,
                    total_tables,
                    total_tables,
                    None,
                    error,
                );
            }
            Err(error) => {
                let _ = emit_job_event(
                    &app_clone,
                    &job_id_clone,
                    TransferJobStatus::Failed,
                    "failed",
                    0,
                    0,
                    None,
                    Some(error),
                );
            }
        }
    });

    Ok(job_id)
}

#[tauri::command]
pub fn save_transfer_profile(
    mut profile: TransferProfile,
    app: AppHandle,
) -> Result<String, String> {
    if profile.id.trim().is_empty() {
        profile.id = Uuid::new_v4().to_string();
    }

    profile.created_at = Utc::now().timestamp_millis();

    let mut profiles = load_profiles(&app)?;
    profiles.push(profile.clone());
    save_profiles(&app, &profiles)?;

    Ok(profile.id)
}

#[tauri::command]
pub fn list_transfer_profiles(app: AppHandle) -> Result<Vec<TransferProfile>, String> {
    load_profiles(&app)
}

#[tauri::command]
pub async fn run_transfer_profile(
    profile_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut profiles = load_profiles(&app)?;
    let profile_index = profiles
        .iter()
        .position(|profile| profile.id == profile_id)
        .ok_or_else(|| "Transfer profile not found".to_string())?;

    profiles[profile_index].last_run_at = Some(Utc::now().timestamp_millis());
    let profile = profiles[profile_index].clone();
    save_profiles(&app, &profiles)?;

    let _job = profile_to_transfer_job(&profile);

    match (profile.kind, profile.scope) {
        (TransferProfileKind::Backup, TransferScope::Server)
        | (TransferProfileKind::Backup, TransferScope::Database)
        | (TransferProfileKind::Backup, TransferScope::Table) => {
            let format = profile
                .format
                .ok_or_else(|| "Backup profile requires format".to_string())?;
            let destination = profile
                .destination
                .ok_or_else(|| "Backup profile requires destination".to_string())?;
            backup_server(
                profile.connection_id,
                profile.selection,
                format,
                destination,
                profile.options,
                None,
                app,
                state,
            )
            .await
        }
        (TransferProfileKind::Migrate, TransferScope::Server)
        | (TransferProfileKind::Migrate, TransferScope::Database)
        | (TransferProfileKind::Migrate, TransferScope::Table) => {
            let target_connection_id = profile
                .target_connection_id
                .ok_or_else(|| "Migrate profile requires targetConnectionId".to_string())?;
            migrate_server(
                profile.connection_id,
                target_connection_id,
                profile.selection,
                profile.options,
                None,
                app,
                state,
            )
            .await
        }
        (TransferProfileKind::Export, TransferScope::Server)
        | (TransferProfileKind::Export, TransferScope::Database)
        | (TransferProfileKind::Export, TransferScope::Table) => {
            let connections = state.connections.clone();
            let selected_tables =
                expand_selection(&profile.connection_id, &profile.selection, &connections).await?;
            let (database, schema, table) = selected_tables
                .first()
                .cloned()
                .ok_or_else(|| "Export profile has no selected table".to_string())?;

            let format = profile
                .format
                .ok_or_else(|| "Export profile requires format".to_string())?;
            let destination = profile
                .destination
                .ok_or_else(|| "Export profile requires destination".to_string())?;

            let columns = list_columns_for_table(
                &profile.connection_id,
                &database,
                schema.as_deref(),
                &table,
                &connections,
            )
            .await?;

            let output_path =
                PathBuf::from(destination).join(format!("{}.{}", table, export_extension(&format)));
            let request = ExportRequest {
                connection_id: profile.connection_id,
                database: Some(database),
                schema,
                source: ExportSource {
                    table,
                    columns,
                    where_clause: None,
                    order_by: None,
                    limit: None,
                },
                format,
                csv_options: None,
                jsonl_options: None,
                sql_options: None,
                excel_options: None,
                output_path: output_path.to_string_lossy().to_string(),
            };

            execute_export_request(request, None, &app, &connections).await?;
            Ok(Uuid::new_v4().to_string())
        }
        (TransferProfileKind::Import, TransferScope::Server)
        | (TransferProfileKind::Import, TransferScope::Database)
        | (TransferProfileKind::Import, TransferScope::Table) => {
            let mut request: ImportRequest = serde_json::from_value(profile.options.clone())
                .map_err(|e| format!("Invalid import profile options: {}", e))?;

            request.connection_id = profile.connection_id;

            if request.table.trim().is_empty() {
                let connections = state.connections.clone();
                let selected_tables =
                    expand_selection(&request.connection_id, &profile.selection, &connections)
                        .await?;
                let (database, schema, table) = selected_tables
                    .first()
                    .cloned()
                    .ok_or_else(|| "Import profile has no selected table".to_string())?;
                request.database = Some(database);
                request.schema = schema;
                request.table = table;
            }

            execute_import_request(request, &app, &state).await?;
            Ok(Uuid::new_v4().to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        filter_system_databases_for_whole_server, quote_ident_mysql, quote_ident_pg,
        summarize_backup_outcome, summarize_migration_outcome,
    };
    use crate::database::DatabaseType;
    use crate::transfer::TransferJobStatus;

    #[test]
    fn quote_ident_mysql_quotes_simple_name() {
        assert_eq!(quote_ident_mysql("foo").unwrap_or_default(), "`foo`");
    }

    #[test]
    fn quote_ident_mysql_escapes_backticks() {
        assert_eq!(
            quote_ident_mysql("foo`bar").unwrap_or_default(),
            "`foo``bar`"
        );
    }

    #[test]
    fn quote_ident_pg_escapes_quotes() {
        assert_eq!(
            quote_ident_pg("foo\"bar").unwrap_or_default(),
            "\"foo\"\"bar\""
        );
    }

    #[test]
    fn quote_ident_mysql_rejects_null_byte() {
        assert!(quote_ident_mysql("foo\0bar").is_err());
    }

    #[test]
    fn quote_ident_pg_prevents_injection_escape() {
        assert_eq!(
            quote_ident_pg("a; DROP TABLE x;--").unwrap_or_default(),
            "\"a; DROP TABLE x;--\""
        );
    }

    #[test]
    fn summarize_migration_outcome_returns_completed_with_partial_failures() {
        let (status, summary) =
            summarize_migration_outcome(2, 1, &["db.public.users: duplicate key".to_string()]);

        assert_eq!(status, TransferJobStatus::Completed);
        assert!(summary.unwrap_or_default().contains("1 failed"));
    }

    #[test]
    fn summarize_migration_outcome_returns_failed_when_no_success() {
        let (status, summary) =
            summarize_migration_outcome(0, 2, &["db.public.orders: timeout".to_string()]);

        assert_eq!(status, TransferJobStatus::Failed);
        assert!(summary.unwrap_or_default().contains("2 failed"));
    }

    #[test]
    fn summarize_backup_outcome_all_succeeded_has_no_summary() {
        let (status, summary) = summarize_backup_outcome(3, 0, &[]);

        assert_eq!(status, TransferJobStatus::Completed);
        assert!(summary.is_none());
    }

    #[test]
    fn summarize_backup_outcome_partial_failures_is_completed_with_summary() {
        let (status, summary) =
            summarize_backup_outcome(2, 1, &["db.public.users: disk full".to_string()]);

        assert_eq!(status, TransferJobStatus::Completed);
        assert!(summary.unwrap_or_default().contains("1 failed"));
    }

    #[test]
    fn summarize_backup_outcome_all_failed_is_failed_with_summary() {
        let (status, summary) =
            summarize_backup_outcome(0, 2, &["db.public.orders: timeout".to_string()]);

        assert_eq!(status, TransferJobStatus::Failed);
        assert!(summary.unwrap_or_default().contains("2 failed"));
    }

    #[test]
    fn filter_system_databases_mysql_excludes_known_system_databases() {
        let filtered = filter_system_databases_for_whole_server(
            Some(DatabaseType::MySQL),
            vec![
                "app_db".to_string(),
                "mysql".to_string(),
                "information_schema".to_string(),
                "sys".to_string(),
            ],
        );

        assert_eq!(filtered, vec!["app_db".to_string()]);
    }

    #[test]
    fn filter_system_databases_postgres_excludes_templates_only() {
        let filtered = filter_system_databases_for_whole_server(
            Some(DatabaseType::PostgreSQL),
            vec![
                "postgres".to_string(),
                "template0".to_string(),
                "template1".to_string(),
                "app_db".to_string(),
            ],
        );

        assert_eq!(filtered, vec!["postgres".to_string(), "app_db".to_string()]);
    }

    #[test]
    fn filter_system_databases_sqlserver_excludes_system_databases() {
        let filtered = filter_system_databases_for_whole_server(
            Some(DatabaseType::SqlServer),
            vec![
                "master".to_string(),
                "msdb".to_string(),
                "tempdb".to_string(),
                "model".to_string(),
                "tenant_db".to_string(),
            ],
        );

        assert_eq!(filtered, vec!["tenant_db".to_string()]);
    }

    #[test]
    fn filter_system_databases_sqlite_is_unchanged() {
        let filtered = filter_system_databases_for_whole_server(
            Some(DatabaseType::SQLite),
            vec!["main".to_string()],
        );

        assert_eq!(filtered, vec!["main".to_string()]);
    }
}
