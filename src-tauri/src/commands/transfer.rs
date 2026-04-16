use crate::state::{ActiveConnection, AppState};
use crate::transfer::{
    auto_map_columns, detect_file, execute_export, execute_import, execute_migration,
    preview_export, preview_import, preview_migration,
    ExportPreview, ExportRequest, FileDetectionResult, ImportFormat, ImportRequest,
    MigrationPreview, MigrationRequest, TransferResult,
};
use crate::database::DatabaseType;
use tauri::{AppHandle, State};

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
        ($adapter:expr) => {
            {
                let adapter = $adapter.lock().await;
                let columns = adapter.list_columns(
                    database.as_deref(),
                    schema.as_deref(),
                    &table,
                ).await.map_err(|e| e.to_string())?;
                Ok(auto_map_columns(&columns, target_db_type))
            }
        };
    }

    match connection {
        ActiveConnection::Postgres(adapter) => fetch_and_map!(adapter),
        ActiveConnection::MySQL(adapter) => fetch_and_map!(adapter),
        ActiveConnection::SQLServer(adapter) => fetch_and_map!(adapter),
        ActiveConnection::SQLite(adapter) => fetch_and_map!(adapter),
    }
}