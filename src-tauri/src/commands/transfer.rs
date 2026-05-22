use crate::database::{DatabaseAdapter, DatabaseType, PostgresAdapter, SqlServerAdapter};
use crate::state::{ActiveConnection, AppState};
use crate::transfer::{
    auto_map_columns, detect_file, execute_export, execute_import, execute_migration,
    generate_ddl_for_engine, load_profiles, preview_export, preview_import, preview_migration,
    save_profiles, DdlRequest, ExportFormat, ExportPreview, ExportRequest, ExportSource,
    FileDetectionResult, ImportFormat, ImportRequest, JobProgress, MigrationPreview,
    MigrationRequest, MigrationTablePlan, ObjectSelection, TransferError, TransferJob,
    TransferJobStatus, TransferProfile, TransferProfileKind, TransferProgress, TransferResult,
    TransferScope,
};
use chrono::Utc;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

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

fn calculate_percent(current: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (current as f32 / total as f32) * 100.0
    }
}

fn collect_selected_tables(selection: &ObjectSelection) -> Vec<(String, Option<String>, String)> {
    let mut seen: HashSet<(String, String, String)> = HashSet::new();

    selection
        .databases
        .iter()
        .flat_map(|db_name| {
            let db_prefix = format!("{}.", db_name);
            let db_prefix_for_filter = db_prefix.clone();
            selection
                .tables
                .iter()
                .filter(move |(key, _)| key.starts_with(&db_prefix_for_filter))
                .flat_map(move |(key, tables)| {
                    let schema = key.strip_prefix(&db_prefix).unwrap_or_default().to_string();
                    tables.iter().map(move |table| {
                        (
                            db_name.clone(),
                            if schema.is_empty() {
                                None
                            } else {
                                Some(schema.clone())
                            },
                            table.clone(),
                        )
                    })
                })
        })
        .filter(|(db, schema, table)| {
            let schema_key = schema.clone().unwrap_or_default();
            seen.insert((db.clone(), schema_key, table.clone()))
        })
        .collect()
}

fn emit_job_progress(
    app: &AppHandle,
    job_id: &str,
    payload: JobProgressPayload,
) -> Result<(), String> {
    let progress = TransferProgress {
        operation: "transfer".to_string(),
        phase: payload.stage,
        current_table: None,
        total_rows: Some(payload.total),
        processed_rows: payload.current,
        skipped_rows: 0,
        error_count: payload.error_count,
        percent: calculate_percent(payload.current, payload.total),
        elapsed_ms: payload.elapsed_ms,
        estimated_remaining_ms: payload.eta_ms,
        message: payload.message,
    };

    app.emit_to("main", &format!("transfer://progress/{}", job_id), progress)
        .map_err(|e| e.to_string())
}

type JobProgressPayload = JobProgressData;

struct JobProgressData {
    stage: String,
    current: u64,
    total: u64,
    elapsed_ms: u64,
    error_count: u64,
    eta_ms: Option<u64>,
    message: Option<String>,
}

async fn list_columns_for_table(
    connection_id: &str,
    database: &str,
    schema: Option<&str>,
    table: &str,
    state: &State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

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
            execute_export(&*adapter, request, app).await
        }
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            execute_export(&*adapter, request, app).await
        }
        ActiveConnection::SQLServer(adapter) => {
            let adapter = adapter.lock().await;
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
    state: &State<'_, AppState>,
) -> Result<(), String> {
    let connections = state.connections.lock().await;
    let connection = connections
        .get(target_connection_id)
        .ok_or_else(|| "No target connection found".to_string())?;

    match connection {
        ActiveConnection::MySQL(adapter) => {
            let adapter = adapter.lock().await;
            adapter
                .execute_query(&format!(
                    "CREATE DATABASE IF NOT EXISTS `{}`",
                    database_name
                ))
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        ActiveConnection::Postgres(adapter) => {
            let adapter = adapter.lock().await;
            let exists = adapter
                .list_databases()
                .await
                .map_err(|e| e.to_string())?
                .iter()
                .any(|db| db.name == database_name);
            if !exists {
                adapter
                    .execute_query(&format!("CREATE DATABASE \"{}\"", database_name))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn execute_migration_request(
    request: MigrationRequest,
    app: &AppHandle,
    state: &State<'_, AppState>,
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
            execute_migration(&*$source_adapter, &*$target_adapter, request, app).await
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

#[tauri::command]
pub async fn backup_server(
    connection_id: String,
    selection: ObjectSelection,
    format: ExportFormat,
    destination: String,
    options: JsonValue,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let _ = &options;
    let job_id = Uuid::new_v4().to_string();
    let started_at = Instant::now();
    let selected_tables = collect_selected_tables(&selection);
    let total_tables = selected_tables.len() as u64;

    if total_tables == 0 {
        return Err("No tables selected for server backup".to_string());
    }

    emit_job_progress(
        &app,
        &job_id,
        JobProgressPayload {
            stage: "preparing".to_string(),
            current: 0,
            total: total_tables,
            elapsed_ms: 0,
            error_count: 0,
            eta_ms: None,
            message: Some("Starting server backup".to_string()),
        },
    )?;

    let mut outcomes: Vec<(String, Result<(), String>)> = Vec::new();
    let mut succeeded_tables = 0u64;

    for (index, (database, schema, table)) in selected_tables.iter().enumerate() {
        let db_output_dir = PathBuf::from(&destination).join(database);
        fs::create_dir_all(&db_output_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;

        let output_path = db_output_dir.join(format!("{}.{}", table, export_extension(&format)));
        let columns = match list_columns_for_table(
            &connection_id,
            database,
            schema.as_deref(),
            table,
            &state,
        )
        .await
        {
            Ok(columns) => columns,
            Err(error) => {
                outcomes.push((
                    format!(
                        "{}.{}.{}",
                        database,
                        schema.clone().unwrap_or_else(|| "default".to_string()),
                        table
                    ),
                    Err(error),
                ));

                let error_count = outcomes
                    .iter()
                    .filter(|(_, result)| result.is_err())
                    .count() as u64;

                emit_job_progress(
                    &app,
                    &job_id,
                    JobProgressPayload {
                        stage: "processing".to_string(),
                        current: index as u64 + 1,
                        total: total_tables,
                        elapsed_ms: started_at.elapsed().as_millis() as u64,
                        error_count,
                        eta_ms: None,
                        message: Some("Failed to load table columns, continuing".to_string()),
                    },
                )?;
                continue;
            }
        };

        let request = ExportRequest {
            connection_id: connection_id.clone(),
            database: Some(database.clone()),
            schema: schema.clone(),
            source: ExportSource {
                table: table.clone(),
                columns,
                where_clause: None,
                order_by: None,
                limit: None,
            },
            format: format.clone(),
            csv_options: None,
            jsonl_options: None,
            sql_options: None,
            excel_options: None,
            output_path: output_path.to_string_lossy().to_string(),
        };

        let export_result = execute_export_request(request, &app, &state).await;
        let table_label = format!(
            "{}.{}.{}",
            database,
            schema.clone().unwrap_or_else(|| "default".to_string()),
            table
        );

        match export_result {
            Ok(result) if result.success => {
                outcomes.push((table_label, Ok(())));
                succeeded_tables += 1;
            }
            Ok(result) => {
                let first_error = result
                    .errors
                    .first()
                    .map(|error| error.message.clone())
                    .unwrap_or_else(|| "Export failed".to_string());
                outcomes.push((table_label, Err(first_error)));
            }
            Err(error) => {
                outcomes.push((table_label, Err(error)));
            }
        }

        let error_count = outcomes
            .iter()
            .filter(|(_, result)| result.is_err())
            .count() as u64;

        emit_job_progress(
            &app,
            &job_id,
            JobProgressPayload {
                stage: "processing".to_string(),
                current: index as u64 + 1,
                total: total_tables,
                elapsed_ms: started_at.elapsed().as_millis() as u64,
                error_count,
                eta_ms: None,
                message: None,
            },
        )?;
    }

    let failed_tables = outcomes
        .iter()
        .filter_map(|(name, result)| result.as_ref().err().map(|_| name.clone()))
        .collect::<Vec<_>>();

    let status_text = if succeeded_tables > 0 {
        "completed"
    } else {
        "failed"
    };
    let summary = format!(
        "Backup {}: {} succeeded, {} failed{}",
        status_text,
        succeeded_tables,
        failed_tables.len(),
        if failed_tables.is_empty() {
            String::new()
        } else {
            format!(" [{}]", failed_tables.join(", "))
        }
    );

    emit_job_progress(
        &app,
        &job_id,
        JobProgressPayload {
            stage: status_text.to_string(),
            current: total_tables,
            total: total_tables,
            elapsed_ms: started_at.elapsed().as_millis() as u64,
            error_count: failed_tables.len() as u64,
            eta_ms: None,
            message: Some(summary),
        },
    )?;

    Ok(job_id)
}

#[tauri::command]
pub async fn migrate_server(
    source_connection_id: String,
    target_connection_id: String,
    selection: ObjectSelection,
    options: JsonValue,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let job_id = Uuid::new_v4().to_string();
    let started_at = Instant::now();
    let selected_tables = collect_selected_tables(&selection);
    let total_tables = selected_tables.len() as u64;

    if total_tables == 0 {
        return Err("No tables selected for server migration".to_string());
    }

    emit_job_progress(
        &app,
        &job_id,
        JobProgressPayload {
            stage: "preparing".to_string(),
            current: 0,
            total: total_tables,
            elapsed_ms: 0,
            error_count: 0,
            eta_ms: None,
            message: Some("Starting server migration".to_string()),
        },
    )?;

    let grouped = selected_tables.into_iter().fold(
        HashMap::<(String, Option<String>), Vec<String>>::new(),
        |mut acc, (database, schema, table)| {
            acc.entry((database, schema)).or_default().push(table);
            acc
        },
    );

    let mut outcomes: Vec<(String, Result<(), String>)> = Vec::new();
    let mut migrated_tables = 0u64;
    let mut processed_tables = 0u64;

    for ((database, schema), tables) in grouped {
        ensure_target_database(&target_connection_id, &database, &state).await?;

        let mappings = tables
            .iter()
            .map(|table| async {
                let columns = list_columns_for_table(
                    &source_connection_id,
                    &database,
                    schema.as_deref(),
                    table,
                    &state,
                )
                .await?;

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

                Ok::<(String, Vec<crate::transfer::MigrationMapping>), String>((
                    table.clone(),
                    mapping,
                ))
            })
            .collect::<Vec<_>>();

        let mut mapping_results: HashMap<String, Vec<crate::transfer::MigrationMapping>> =
            HashMap::new();

        for mapping_result in mappings {
            let (table_name, table_mapping) = mapping_result.await?;
            mapping_results.insert(table_name, table_mapping);
        }

        let request = build_migration_request(
            source_connection_id.clone(),
            target_connection_id.clone(),
            database.clone(),
            schema.clone(),
            tables.clone(),
            mapping_results,
            &options,
        );

        let migration_result = execute_migration_request(request, &app, &state).await;
        let group_label = format!(
            "{}.{}",
            database,
            schema.clone().unwrap_or_else(|| "default".to_string())
        );

        match migration_result {
            Ok(result) if result.success => {
                outcomes.push((group_label, Ok(())));
                migrated_tables += result.processed_rows;
            }
            Ok(result) => {
                let first_error = result
                    .errors
                    .first()
                    .map(|error| error.message.clone())
                    .unwrap_or_else(|| "Migration failed".to_string());
                outcomes.push((group_label, Err(first_error)));
            }
            Err(error) => {
                outcomes.push((group_label, Err(error)));
            }
        }

        processed_tables += tables.len() as u64;
        let error_count = outcomes
            .iter()
            .filter(|(_, result)| result.is_err())
            .count() as u64;

        emit_job_progress(
            &app,
            &job_id,
            JobProgressPayload {
                stage: "processing".to_string(),
                current: processed_tables,
                total: total_tables,
                elapsed_ms: started_at.elapsed().as_millis() as u64,
                error_count,
                eta_ms: None,
                message: None,
            },
        )?;
    }

    let failed_groups = outcomes
        .iter()
        .filter_map(|(name, result)| result.as_ref().err().map(|_| name.clone()))
        .collect::<Vec<_>>();

    let status_text = if migrated_tables > 0 {
        "completed"
    } else {
        "failed"
    };

    emit_job_progress(
        &app,
        &job_id,
        JobProgressPayload {
            stage: status_text.to_string(),
            current: total_tables,
            total: total_tables,
            elapsed_ms: started_at.elapsed().as_millis() as u64,
            error_count: failed_groups.len() as u64,
            eta_ms: None,
            message: Some(format!(
                "Migration {}: {} rows migrated, {} groups failed{}",
                status_text,
                migrated_tables,
                failed_groups.len(),
                if failed_groups.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", failed_groups.join(", "))
                }
            )),
        },
    )?;

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
                app,
                state,
            )
            .await
        }
        (TransferProfileKind::Export, TransferScope::Server)
        | (TransferProfileKind::Export, TransferScope::Database)
        | (TransferProfileKind::Export, TransferScope::Table) => {
            let selected_tables = collect_selected_tables(&profile.selection);
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
                &state,
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

            execute_export_request(request, &app, &state).await?;
            Ok(Uuid::new_v4().to_string())
        }
        (TransferProfileKind::Import, TransferScope::Server)
        | (TransferProfileKind::Import, TransferScope::Database)
        | (TransferProfileKind::Import, TransferScope::Table) => {
            let mut request: ImportRequest = serde_json::from_value(profile.options.clone())
                .map_err(|e| format!("Invalid import profile options: {}", e))?;

            request.connection_id = profile.connection_id;

            if request.table.trim().is_empty() {
                let selected_tables = collect_selected_tables(&profile.selection);
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
