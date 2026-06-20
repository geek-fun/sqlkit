use crate::database::{DatabaseAdapter, DatabaseType, PostgresAdapter, SqlServerAdapter};
use crate::state::{ActiveConnection, AppState};
use crate::transfer::{
    auto_map_columns, detect_file, execute_export, execute_import, execute_migration,
    generate_ddl_for_engine, preview_export, preview_import, preview_migration, DdlRequest,
    ExportPreview, ExportRequest, FileDetectionResult, ImportFormat, ImportRequest,
    MigrationPreview, MigrationRequest, TransferError, TransferResult,
};
use std::time::Instant;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn preview_export_data(
    request: ExportRequest,
    preview_rows: u32,
    state: State<'_, AppState>,
) -> Result<ExportPreview, String> {
    let connections = state.connections.read().await;
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
        _ => return Err("Transfer not supported for this database type".to_string()),
    }
}

#[tauri::command]
pub async fn execute_export_data(
    request: ExportRequest,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> {
    let connections = state.connections.read().await;
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
        _ => return Err("Transfer not supported for this database type".to_string()),
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
    let connections = state.connections.read().await;
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
        _ => return Err("Transfer not supported for this database type".to_string()),
    }
}

#[tauri::command]
pub async fn preview_migration_data(
    request: MigrationRequest,
    state: State<'_, AppState>,
) -> Result<MigrationPreview, String> {
    let connections = state.connections.read().await;

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
        _ => return Err("Transfer not supported for this database type".to_string()),
    }
}

#[tauri::command]
pub async fn execute_migration_data(
    request: MigrationRequest,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> {
    let connections = state.connections.read().await;

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
        _ => todo!(),
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

    let connections = state.connections.read().await;
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
        _ => return Err("Transfer not supported for this database type".to_string()),
    }
}

#[tauri::command]
pub async fn generate_ddl_for_objects(
    request: DdlRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let connections = state.connections.read().await;
    let connection = connections
        .get(&request.connection_id)
        .ok_or_else(|| "No active connection found".to_string())?;

    let engine = match connection {
        ActiveConnection::Postgres(_) => DatabaseType::PostgreSQL,
        ActiveConnection::MySQL(_) => DatabaseType::MySQL,
        ActiveConnection::SQLServer(_) => DatabaseType::SqlServer,
        ActiveConnection::SQLite(_) => DatabaseType::SQLite,
        _ => return Err("Transfer not supported for this database type".to_string()),
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
        _ => return Err("Transfer not supported for this database type".to_string()),
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
    let connections = state.connections.read().await;
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
        _ => return Err("Transfer not supported for this database type".to_string()),
    };

    Ok(result)
}
