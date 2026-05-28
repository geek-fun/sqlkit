//! Export implementation for CSV, JSONL, SQL, and Excel formats.
//! Supports scope-based export with ZIP creation for Server, Database, and Tables scopes.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use rust_xlsxwriter::{Workbook, Worksheet};
use serde_json::Value as JsonValue;
use zip::write::FileOptions;
use zip::ZipWriter;

use super::defaults::*;
use super::progress::*;
use super::types::*;
use crate::database::{DatabaseAdapter, QueryValue};

const BATCH_SIZE: u64 = 1000;

// ── Chat2DB-style naming ─────────────────────────────────────────

/// Format a table filename with Chat2DB-style timestamp suffix.
fn format_chat2db_filename(table: &str, ext: &str) -> String {
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    format!("{}_{}.{}", table, timestamp, ext)
}

/// Get the file extension for an export format.
fn format_extension(format: &ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Jsonl => "jsonl",
        ExportFormat::Sql => "sql",
        ExportFormat::Excel => "xlsx",
    }
}

// ── Scope-based export entry point ───────────────────────────────

/// Executes a data export operation.
///
/// Supports three scopes:
/// - `Tables`: Export specified sources. Single source → single file, multiple → ZIP.
/// - `Database`: List all tables in the database, export each to ZIP.
/// - `Server`: List all databases + tables, export each with nested paths to ZIP.
pub async fn execute_export<A: DatabaseAdapter>(
    adapter: &A,
    request: ExportRequest,
    app_handle: &tauri::AppHandle,
) -> Result<TransferResult, String> {
    let start_time = Instant::now();

    match &request.scope {
        TransferScope::Tables if request.sources.len() <= 1 => {
            execute_single_table_export(adapter, request, app_handle, start_time).await
        }
        TransferScope::Tables => {
            // Multiple sources → ZIP with flat paths
            let sources_with_paths: Vec<(Option<String>, ExportSource)> = request
                .sources
                .iter()
                .map(|s| (None, s.clone()))
                .collect();
            execute_zip_export(
                adapter,
                request,
                sources_with_paths,
                app_handle,
                start_time,
            )
            .await
        }
        TransferScope::Database => {
            let db_name = request
                .database
                .clone()
                .ok_or_else(|| "database name is required for Database scope".to_string())?;

            let mut progress = create_progress(
                "export",
                "discovering",
                0,
                None,
                start_time.elapsed().as_millis() as u64,
            );
            progress.current_database = Some(db_name.clone());
            emit_progress(app_handle, &progress);

            let tables = adapter
                .list_tables(Some(&db_name), request.schema.as_deref())
                .await
                .map_err(|e| format!("Failed to list tables in '{}': {}", db_name, e))?;

            let mut sources: Vec<ExportSource> = Vec::with_capacity(tables.len());
            for table_info in &tables {
                let columns = adapter
                    .list_columns(Some(&db_name), request.schema.as_deref(), &table_info.name)
                    .await
                    .map_err(|e| format!("Failed to list columns for '{}': {}", table_info.name, e))?;
                sources.push(ExportSource {
                    table: table_info.name.clone(),
                    columns: columns.iter().map(|c| c.name.clone()).collect(),
                });
            }

            // Database scope uses flat paths ({table}.{ext}) in ZIP,
            // but we still pass the db name for progress tracking.
            // The execute_zip_export uses the Option for path nesting,
            // so we pass None here for flat entries.
            let sources_flat: Vec<ExportSource> = sources;
            let sources_with_paths: Vec<(Option<String>, ExportSource)> =
                sources_flat.into_iter().map(|s| (None, s)).collect();

            execute_zip_export(adapter, request, sources_with_paths, app_handle, start_time).await
        }
        TransferScope::Server => {
            emit_progress(
                app_handle,
                &create_progress(
                    "export",
                    "discovering",
                    0,
                    None,
                    start_time.elapsed().as_millis() as u64,
                ),
            );

            let databases = adapter
                .list_databases()
                .await
                .map_err(|e| format!("Failed to list databases: {}", e))?;

            let mut sources_with_paths: Vec<(Option<String>, ExportSource)> = Vec::new();

            for (db_idx, db) in databases.iter().enumerate() {
                let mut progress = create_progress(
                    "export",
                    "discovering",
                    db_idx as u64,
                    Some(databases.len() as u64),
                    start_time.elapsed().as_millis() as u64,
                );
                progress.current_database = Some(db.name.clone());
                emit_progress(app_handle, &progress);

                let tables = adapter
                    .list_tables(Some(&db.name), request.schema.as_deref())
                    .await
                    .map_err(|e| format!("Failed to list tables in '{}': {}", db.name, e))?;

                for table_info in &tables {
                    let columns = adapter
                        .list_columns(Some(&db.name), request.schema.as_deref(), &table_info.name)
                        .await
                        .map_err(|e| {
                            format!(
                                "Failed to list columns for '{}' in '{}': {}",
                                table_info.name, db.name, e
                            )
                        })?;
                    sources_with_paths.push((
                        Some(db.name.clone()),
                        ExportSource {
                            table: table_info.name.clone(),
                            columns: columns.iter().map(|c| c.name.clone()).collect(),
                        },
                    ));
                }
            }

            execute_zip_export(adapter, request, sources_with_paths, app_handle, start_time).await
        }
    }
}

// ── Single-table export (original behavior) ─────────────────────

/// Execute export for a single table to a single file.
async fn execute_single_table_export<A: DatabaseAdapter>(
    adapter: &A,
    request: ExportRequest,
    app_handle: &tauri::AppHandle,
    start_time: Instant,
) -> Result<TransferResult, String> {
    let source = request
        .sources
        .first()
        .ok_or("No export sources specified")?
        .clone();
    let columns = source.columns.clone();
    let table = source.table.clone();
    let schema = request.schema.clone();

    let csv_opts = request
        .csv_options
        .clone()
        .unwrap_or_else(csv_export_defaults);
    let jsonl_opts = request
        .jsonl_options
        .clone()
        .unwrap_or_else(jsonl_export_defaults);
    let sql_opts = request
        .sql_options
        .clone()
        .unwrap_or_else(|| sql_export_defaults(&table));
    let excel_opts = request
        .excel_options
        .clone()
        .unwrap_or_else(excel_export_defaults);

    let base_query = build_export_query(&schema, &table, &columns);

    let count_query = build_count_query(&schema, &table);
    let count_result = adapter
        .execute_query(&count_query)
        .await
        .map_err(|e| e.to_string())?;
    let total_rows = count_result
        .rows
        .first()
        .and_then(|row| row.get("count"))
        .and_then(|v| match v {
            QueryValue::Int(n) => Some(*n as u64),
            _ => None,
        })
        .unwrap_or(0);

    emit_progress(
        app_handle,
        &create_progress("export", "preparing", 0, Some(total_rows), 0),
    );

    let output_path = Path::new(&request.output_path);
    let file = File::create(output_path).map_err(|e| format!("Failed to create file: {}", e))?;
    let mut writer = BufWriter::new(file);

    let mut processed_rows: u64 = 0;
    let mut errors: Vec<TransferError> = Vec::new();

    match request.format {
        ExportFormat::Csv => {
            if csv_opts.include_header {
                write_csv_header(&mut writer, &columns, csv_opts.delimiter)
                    .map_err(|e| e.to_string())?;
            }

            let mut offset = 0u64;
            while offset < total_rows {
                let query =
                    format!("{} LIMIT {} OFFSET {}", base_query, BATCH_SIZE, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    write_csv_row(&mut writer, &columns, row, &csv_opts).map_err(|e| {
                        errors.push(TransferError {
                            row_number: Some(processed_rows + 1),
                            statement_number: None,
                            message: e,
                            sql: None,
                        });
                        String::new()
                    })?;
                    processed_rows += 1;
                }

                offset += BATCH_SIZE;
                emit_progress(
                    app_handle,
                    &create_progress(
                        "export",
                        "processing",
                        processed_rows,
                        Some(total_rows),
                        start_time.elapsed().as_millis() as u64,
                    ),
                );
            }
        }

        ExportFormat::Jsonl => {
            let mut offset = 0u64;
            while offset < total_rows {
                let query =
                    format!("{} LIMIT {} OFFSET {}", base_query, BATCH_SIZE, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    let json_obj = row_to_json_object(row, &jsonl_opts.date_format);
                    let json_line =
                        serde_json::to_string(&json_obj).map_err(|e| e.to_string())?;
                    writer
                        .write_all(json_line.as_bytes())
                        .map_err(|e| e.to_string())?;
                    writer.write_all(b"\n").map_err(|e| e.to_string())?;
                    processed_rows += 1;
                }

                offset += BATCH_SIZE;
                emit_progress(
                    app_handle,
                    &create_progress(
                        "export",
                        "processing",
                        processed_rows,
                        Some(total_rows),
                        start_time.elapsed().as_millis() as u64,
                    ),
                );
            }
        }

        ExportFormat::Sql => {
            if sql_opts.include_create_table {
                let table_info = adapter
                    .get_table_info(schema.as_deref(), None, &table)
                    .await
                    .map_err(|e| e.to_string())?;
                let create_stmt =
                    generate_create_table_sql(&table, &table_info, sql_opts.include_drop_table);
                writer
                    .write_all(create_stmt.as_bytes())
                    .map_err(|e| e.to_string())?;
                writer.write_all(b"\n\n").map_err(|e| e.to_string())?;
            }

            let mut batch_rows: Vec<Vec<QueryValue>> = Vec::new();
            let mut offset = 0u64;

            while offset < total_rows {
                let query =
                    format!("{} LIMIT {} OFFSET {}", base_query, BATCH_SIZE, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    let values: Vec<QueryValue> = columns
                        .iter()
                        .map(|col| row.get(col).cloned().unwrap_or(QueryValue::Null))
                        .collect();
                    batch_rows.push(values);
                    processed_rows += 1;

                    if batch_rows.len() >= sql_opts.batch_size as usize {
                        let insert_stmt =
                            generate_insert_sql(&schema, &table, &columns, &batch_rows);
                        writer
                            .write_all(insert_stmt.as_bytes())
                            .map_err(|e| e.to_string())?;
                        writer.write_all(b"\n").map_err(|e| e.to_string())?;
                        batch_rows.clear();
                    }
                }

                offset += BATCH_SIZE;
                emit_progress(
                    app_handle,
                    &create_progress(
                        "export",
                        "processing",
                        processed_rows,
                        Some(total_rows),
                        start_time.elapsed().as_millis() as u64,
                    ),
                );
            }

            if !batch_rows.is_empty() {
                let insert_stmt = generate_insert_sql(&schema, &table, &columns, &batch_rows);
                writer
                    .write_all(insert_stmt.as_bytes())
                    .map_err(|e| e.to_string())?;
            }
        }

        ExportFormat::Excel => {
            let mut workbook = Workbook::new();
            let worksheet = workbook
                .add_worksheet()
                .set_name(&excel_opts.sheet_name)
                .map_err(|e| e.to_string())?;

            if excel_opts.include_header {
                for (col_idx, col_name) in columns.iter().enumerate() {
                    worksheet
                        .write_string(0, col_idx as u16, col_name)
                        .map_err(|e| e.to_string())?;
                }
            }

            if excel_opts.freeze_header && excel_opts.include_header {
                worksheet
                    .set_freeze_panes(1, 0)
                    .map_err(|e| e.to_string())?;
            }

            let header_row_offset = if excel_opts.include_header { 1 } else { 0 };

            let mut offset = 0u64;
            let mut row_idx = header_row_offset;

            while offset < total_rows {
                let query =
                    format!("{} LIMIT {} OFFSET {}", base_query, BATCH_SIZE, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    for (col_idx, col_name) in columns.iter().enumerate() {
                        let value = row.get(col_name).cloned().unwrap_or(QueryValue::Null);
                        write_excel_cell(worksheet, row_idx, col_idx as u16, &value)?;
                    }
                    row_idx += 1;
                    processed_rows += 1;
                }

                offset += BATCH_SIZE;
                emit_progress(
                    app_handle,
                    &create_progress(
                        "export",
                        "processing",
                        processed_rows,
                        Some(total_rows),
                        start_time.elapsed().as_millis() as u64,
                    ),
                );
            }

            if excel_opts.auto_fit_columns {
                let max_col = columns.len() as u16;
                for col_idx in 0..max_col {
                    worksheet
                        .set_column_width(col_idx, 12.0)
                        .map_err(|e| e.to_string())?;
                }
            }

            workbook
                .save(output_path)
                .map_err(|e| format!("Failed to save Excel file: {}", e))?;
        }
    }

    writer
        .flush()
        .map_err(|e| format!("Failed to flush file: {}", e))?;

    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    emit_progress(
        app_handle,
        &create_progress(
            "export",
            "finalizing",
            processed_rows,
            Some(total_rows),
            start_time.elapsed().as_millis() as u64,
        ),
    );

    Ok(TransferResult {
        success: errors.is_empty(),
        total_rows,
        processed_rows,
        skipped_rows: 0,
        error_count: errors.len() as u64,
        duration_ms: start_time.elapsed().as_millis() as u64,
        output_path: Some(request.output_path),
        output_size_bytes: Some(file_size),
        errors,
    })
}

// ── ZIP-based multi-table export ─────────────────────────────────

/// Execute export for multiple tables into a single ZIP file.
///
/// Each source is accompanied by an optional database name.
/// When `Some(db)`, the ZIP entry path is `{db}/{chat2db_name}`.
/// When `None`, the entry path is `{chat2db_name}` (flat).
async fn execute_zip_export<A: DatabaseAdapter>(
    adapter: &A,
    request: ExportRequest,
    sources_with_db: Vec<(Option<String>, ExportSource)>,
    app_handle: &tauri::AppHandle,
    start_time: Instant,
) -> Result<TransferResult, String> {
    let csv_opts = request
        .csv_options
        .clone()
        .unwrap_or_else(csv_export_defaults);
    let jsonl_opts = request
        .jsonl_options
        .clone()
        .unwrap_or_else(jsonl_export_defaults);
    let excel_opts = request
        .excel_options
        .clone()
        .unwrap_or_else(excel_export_defaults);
    let ext = format_extension(&request.format);
    let schema = request.schema.as_deref();

    let output_path = Path::new(&request.output_path);
    let file = File::create(output_path).map_err(|e| format!("Failed to create ZIP file: {}", e))?;
    let mut zip = ZipWriter::new(file);

    let mut grand_total: u64 = 0;
    let mut grand_processed: u64 = 0;
    let mut errors: Vec<TransferError> = Vec::new();

    let total_sources = sources_with_db.len() as u64;

    for (source_idx, (db_name_opt, source)) in sources_with_db.iter().enumerate() {
        let table = &source.table;

        // Emit discovering progress for this table
        {
            let mut progress = create_progress(
                "export",
                if total_sources > 0 && source_idx == 0 {
                    "discovering"
                } else {
                    "processing"
                },
                source_idx as u64,
                Some(total_sources),
                start_time.elapsed().as_millis() as u64,
            );
            progress.current_database = db_name_opt.clone();
            progress.current_table = Some(table.clone());
            progress.message = Some(format!("Exporting table: {}", table));
            emit_progress(app_handle, &progress);
        }

        // Determine ZIP entry path
        let chat2db_name = format_chat2db_filename(table, ext);
        let zip_path = match db_name_opt {
            Some(db) => format!("{}/{}", db, chat2db_name),
            None => chat2db_name,
        };

        // Export this table to a byte buffer
        // Build the SqlExportOptions for each table (some options reference the table name)
        let sql_opts = request
            .sql_options
            .clone()
            .unwrap_or_else(|| sql_export_defaults(table));

        let data = export_table_to_bytes(
            adapter,
            source,
            schema,
            db_name_opt.as_deref(),
            &request.format,
            &csv_opts,
            &jsonl_opts,
            &sql_opts,
            &excel_opts,
            app_handle,
            &mut grand_total,
            &mut grand_processed,
            &mut errors,
            start_time,
        )
        .await?;

        // Add entry to ZIP
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file(&zip_path, options)
            .map_err(|e| format!("ZIP error: {}", e))?;
        zip.write_all(&data)
            .map_err(|e| format!("ZIP write error: {}", e))?;
    }

    // Finalize ZIP
    let _zip_output = zip
        .finish()
        .map_err(|e| format!("ZIP finalize error: {}", e))?;

    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    emit_progress(
        app_handle,
        &create_progress(
            "export",
            "finalizing",
            grand_processed,
            Some(grand_total),
            start_time.elapsed().as_millis() as u64,
        ),
    );

    Ok(TransferResult {
        success: errors.is_empty(),
        total_rows: grand_total,
        processed_rows: grand_processed,
        skipped_rows: 0,
        error_count: errors.len() as u64,
        duration_ms: start_time.elapsed().as_millis() as u64,
        output_path: Some(request.output_path),
        output_size_bytes: Some(file_size),
        errors,
    })
}

/// Export a single table's data to a byte vector, suitable for ZIP inclusion.
#[allow(clippy::too_many_arguments)]
async fn export_table_to_bytes<A: DatabaseAdapter>(
    adapter: &A,
    source: &ExportSource,
    schema: Option<&str>,
    database: Option<&str>,
    format: &ExportFormat,
    csv_opts: &CsvExportOptions,
    jsonl_opts: &JsonlExportOptions,
    sql_opts: &SqlExportOptions,
    excel_opts: &ExcelExportOptions,
    app_handle: &tauri::AppHandle,
    accumulated_total: &mut u64,
    accumulated_processed: &mut u64,
    errors: &mut Vec<TransferError>,
    start_time: Instant,
) -> Result<Vec<u8>, String> {
    let columns = &source.columns;
    let table = &source.table;

    let base_query = build_export_query(&schema.map(|s| s.to_string()), table, columns);
    let count_query = build_count_query(&schema.map(|s| s.to_string()), table);

    let count_result = adapter
        .execute_query(&count_query)
        .await
        .map_err(|e| e.to_string())?;
    let total_rows = count_result
        .rows
        .first()
        .and_then(|row| row.get("count"))
        .and_then(|v| match v {
            QueryValue::Int(n) => Some(*n as u64),
            _ => None,
        })
        .unwrap_or(0);
    *accumulated_total += total_rows;

    let mut local_processed: u64 = 0;

    match format {
        ExportFormat::Csv => {
            let mut buffer = Vec::new();

            if csv_opts.include_header {
                let mut buf = BufWriter::new(&mut buffer);
                write_csv_header(&mut buf, columns, csv_opts.delimiter)
                    .map_err(|e| e.to_string())?;
                buf.flush().map_err(|e| e.to_string())?;
            }

            let mut offset = 0u64;
            while offset < total_rows {
                let query =
                    format!("{} LIMIT {} OFFSET {}", base_query, BATCH_SIZE, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    let mut buf = BufWriter::new(&mut buffer);
                    write_csv_row(&mut buf, columns, row, csv_opts).map_err(|e| {
                        errors.push(TransferError {
                            row_number: Some(*accumulated_processed + local_processed + 1),
                            statement_number: None,
                            message: e,
                            sql: None,
                        });
                        String::new()
                    })?;
                    buf.flush().map_err(|e| e.to_string())?;
                    local_processed += 1;
                }

                offset += BATCH_SIZE;
                let mut progress = create_progress(
                    "export",
                    "processing",
                    *accumulated_processed + local_processed,
                    Some(*accumulated_total),
                    start_time.elapsed().as_millis() as u64,
                );
                progress.current_database = database.map(|s| s.to_string());
                progress.current_table = Some(table.clone());
                emit_progress(app_handle, &progress);
            }

            *accumulated_processed += local_processed;
            Ok(buffer)
        }

        ExportFormat::Jsonl => {
            let mut buffer = Vec::new();

            let mut offset = 0u64;
            while offset < total_rows {
                let query =
                    format!("{} LIMIT {} OFFSET {}", base_query, BATCH_SIZE, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    let json_obj = row_to_json_object(row, &jsonl_opts.date_format);
                    let json_line =
                        serde_json::to_string(&json_obj).map_err(|e| e.to_string())?;
                    buffer.extend_from_slice(json_line.as_bytes());
                    buffer.push(b'\n');
                    local_processed += 1;
                }

                offset += BATCH_SIZE;
                let mut progress = create_progress(
                    "export",
                    "processing",
                    *accumulated_processed + local_processed,
                    Some(*accumulated_total),
                    start_time.elapsed().as_millis() as u64,
                );
                progress.current_database = database.map(|s| s.to_string());
                progress.current_table = Some(table.clone());
                emit_progress(app_handle, &progress);
            }

            *accumulated_processed += local_processed;
            Ok(buffer)
        }

        ExportFormat::Sql => {
            let mut buffer = Vec::new();
            let schema_ref = schema.map(|s| s.to_string());

            if sql_opts.include_create_table {
                let table_info = adapter
                    .get_table_info(database, schema, table)
                    .await
                    .map_err(|e| e.to_string())?;
                let create_stmt =
                    generate_create_table_sql(table, &table_info, sql_opts.include_drop_table);
                buffer.extend_from_slice(create_stmt.as_bytes());
                buffer.extend_from_slice(b"\n\n");
            }

            let mut batch_rows: Vec<Vec<QueryValue>> = Vec::new();
            let mut offset = 0u64;

            while offset < total_rows {
                let query =
                    format!("{} LIMIT {} OFFSET {}", base_query, BATCH_SIZE, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    let values: Vec<QueryValue> = columns
                        .iter()
                        .map(|col| row.get(col).cloned().unwrap_or(QueryValue::Null))
                        .collect();
                    batch_rows.push(values);
                    local_processed += 1;

                    if batch_rows.len() >= sql_opts.batch_size as usize {
                        let insert_stmt =
                            generate_insert_sql(&schema_ref, table, columns, &batch_rows);
                        buffer.extend_from_slice(insert_stmt.as_bytes());
                        buffer.push(b'\n');
                        batch_rows.clear();
                    }
                }

                offset += BATCH_SIZE;
                let mut progress = create_progress(
                    "export",
                    "processing",
                    *accumulated_processed + local_processed,
                    Some(*accumulated_total),
                    start_time.elapsed().as_millis() as u64,
                );
                progress.current_database = database.map(|s| s.to_string());
                progress.current_table = Some(table.clone());
                emit_progress(app_handle, &progress);
            }

            if !batch_rows.is_empty() {
                let insert_stmt = generate_insert_sql(&schema_ref, table, columns, &batch_rows);
                buffer.extend_from_slice(insert_stmt.as_bytes());
            }

            *accumulated_processed += local_processed;
            Ok(buffer)
        }

        ExportFormat::Excel => {
            let mut workbook = Workbook::new();
            let worksheet = workbook
                .add_worksheet()
                .set_name(&excel_opts.sheet_name)
                .map_err(|e| e.to_string())?;

            if excel_opts.include_header {
                for (col_idx, col_name) in columns.iter().enumerate() {
                    worksheet
                        .write_string(0, col_idx as u16, col_name)
                        .map_err(|e| e.to_string())?;
                }
            }

            if excel_opts.freeze_header && excel_opts.include_header {
                worksheet
                    .set_freeze_panes(1, 0)
                    .map_err(|e| e.to_string())?;
            }

            let header_row_offset = if excel_opts.include_header { 1 } else { 0 };
            let mut offset = 0u64;
            let mut row_idx = header_row_offset;

            while offset < total_rows {
                let query =
                    format!("{} LIMIT {} OFFSET {}", base_query, BATCH_SIZE, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    for (col_idx, col_name) in columns.iter().enumerate() {
                        let value = row.get(col_name).cloned().unwrap_or(QueryValue::Null);
                        write_excel_cell(worksheet, row_idx, col_idx as u16, &value)?;
                    }
                    row_idx += 1;
                    local_processed += 1;
                }

                offset += BATCH_SIZE;
                let mut progress = create_progress(
                    "export",
                    "processing",
                    *accumulated_processed + local_processed,
                    Some(*accumulated_total),
                    start_time.elapsed().as_millis() as u64,
                );
                progress.current_database = database.map(|s| s.to_string());
                progress.current_table = Some(table.clone());
                emit_progress(app_handle, &progress);
            }

            if excel_opts.auto_fit_columns {
                let max_col = columns.len() as u16;
                for col_idx in 0..max_col {
                    worksheet
                        .set_column_width(col_idx, 12.0)
                        .map_err(|e| e.to_string())?;
                }
            }

            *accumulated_processed += local_processed;
            workbook
                .save_to_buffer()
                .map_err(|e| format!("Failed to save Excel to buffer: {}", e))
        }
    }
}

// ── Query builders ───────────────────────────────────────────────

fn build_export_query(schema: &Option<String>, table: &str, columns: &[String]) -> String {
    let schema_prefix = schema
        .as_ref()
        .map(|s| format!("\"{}\".", s))
        .unwrap_or_default();
    let cols = columns
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(", ");

    format!("SELECT {} FROM {}\"{}\"", cols, schema_prefix, table)
}

fn build_count_query(schema: &Option<String>, table: &str) -> String {
    let schema_prefix = schema
        .as_ref()
        .map(|s| format!("\"{}\".", s))
        .unwrap_or_default();
    format!(
        "SELECT COUNT(*) AS count FROM {}\"{}\"",
        schema_prefix, table
    )
}

// ── CSV helpers ──────────────────────────────────────────────────

fn write_csv_header<W: Write>(
    writer: &mut BufWriter<W>,
    columns: &[String],
    delimiter: char,
) -> Result<(), std::io::Error> {
    let header = columns
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(&delimiter.to_string());
    writer.write_all(header.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn write_csv_row<W: Write>(
    writer: &mut BufWriter<W>,
    columns: &[String],
    row: &crate::database::QueryRow,
    opts: &CsvExportOptions,
) -> Result<(), String> {
    let values: Vec<String> = columns
        .iter()
        .map(|col| match row.get(col) {
            Some(QueryValue::Null) => "".to_string(),
            Some(QueryValue::Bool(b)) => b.to_string(),
            Some(QueryValue::Int(n)) => n.to_string(),
            Some(QueryValue::Float(f)) => f.to_string(),
            Some(QueryValue::String(s)) => {
                if opts.quote_all
                    || s.contains(&opts.delimiter.to_string())
                    || s.contains('"')
                    || s.contains('\n')
                {
                    format!("\"{}\"", s.replace('"', "\"\""))
                }
                else {
                    s.clone()
                }
            }
            Some(QueryValue::Bytes(b)) => format!("0x{}", hex::encode(b)),
            Some(QueryValue::DateTime(dt)) => dt.clone(),
            None => "".to_string(),
        })
        .collect();

    let line = values.join(&opts.delimiter.to_string());
    writer
        .write_all(line.as_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(())
}

// ── JSONL helpers ────────────────────────────────────────────────

fn row_to_json_object(row: &crate::database::QueryRow, _date_format: &str) -> JsonValue {
    let mut obj = serde_json::Map::new();
    for (key, value) in row {
        let json_val = match value {
            QueryValue::Null => JsonValue::Null,
            QueryValue::Bool(b) => JsonValue::Bool(*b),
            QueryValue::Int(n) => JsonValue::Number((*n).into()),
            QueryValue::Float(f) => JsonValue::Number(
                serde_json::Number::from_f64(*f).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
            QueryValue::String(s) => JsonValue::String(s.clone()),
            QueryValue::Bytes(b) => JsonValue::String(hex::encode(b)),
            QueryValue::DateTime(dt) => JsonValue::String(dt.clone()),
        };
        obj.insert(key.clone(), json_val);
    }
    JsonValue::Object(obj)
}

// ── SQL helpers ──────────────────────────────────────────────────

fn generate_create_table_sql(
    table: &str,
    _table_info: &crate::database::TableInfo,
    include_drop: bool,
) -> String {
    let mut sql = String::new();

    if include_drop {
        sql.push_str(&format!("DROP TABLE IF EXISTS \"{}\";\n", table));
    }

    sql.push_str(&format!("CREATE TABLE \"{}\" (\n", table));

    sql.push_str(");");
    sql
}

fn generate_insert_sql(
    schema: &Option<String>,
    table: &str,
    columns: &[String],
    rows: &[Vec<QueryValue>],
) -> String {
    let schema_prefix = schema
        .as_ref()
        .map(|s| format!("\"{}\".", s))
        .unwrap_or_default();
    let col_list = columns
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(", ");

    let values_list: Vec<String> = rows
        .iter()
        .map(|row| {
            let vals: Vec<String> = row.iter().map(query_value_to_sql_literal).collect();
            format!("({})", vals.join(", "))
        })
        .collect();

    format!(
        "INSERT INTO {}\"{}\" ({}) VALUES {};",
        schema_prefix,
        table,
        col_list,
        values_list.join(", ")
    )
}

fn query_value_to_sql_literal(value: &QueryValue) -> String {
    match value {
        QueryValue::Null => "NULL".to_string(),
        QueryValue::Bool(b) => b.to_string(),
        QueryValue::Int(n) => n.to_string(),
        QueryValue::Float(f) => f.to_string(),
        QueryValue::String(s) => format!("'{}'", s.replace('\'', "''")),
        QueryValue::Bytes(b) => format!("'{}'", hex::encode(b)),
        QueryValue::DateTime(dt) => format!("'{}'", dt),
    }
}

// ── Preview ──────────────────────────────────────────────────────

/// Generates a preview of export data from the first export source.
pub async fn preview_export<A: DatabaseAdapter>(
    adapter: &A,
    request: ExportRequest,
    preview_rows: u32,
) -> Result<ExportPreview, String> {
    let source = request
        .sources
        .first()
        .ok_or("No export sources specified")?
        .clone();
    let columns = source.columns.clone();
    let table = source.table.clone();
    let schema = request.schema.clone();

    let base_query = build_export_query(&schema, &table, &columns);
    let query = format!("{} LIMIT {}", base_query, preview_rows);

    let result = adapter
        .execute_query(&query)
        .await
        .map_err(|e| e.to_string())?;

    let sample_rows: Vec<Vec<String>> = result
        .rows
        .iter()
        .map(|row| {
            columns
                .iter()
                .map(|col| match row.get(col) {
                    Some(QueryValue::Null) => "".to_string(),
                    Some(QueryValue::Bool(b)) => b.to_string(),
                    Some(QueryValue::Int(n)) => n.to_string(),
                    Some(QueryValue::Float(f)) => f.to_string(),
                    Some(QueryValue::String(s)) => s.clone(),
                    Some(QueryValue::Bytes(b)) => hex::encode(b),
                    Some(QueryValue::DateTime(dt)) => dt.clone(),
                    None => "".to_string(),
                })
                .collect()
        })
        .collect();

    let count_query = build_count_query(&schema, &table);
    let count_result = adapter
        .execute_query(&count_query)
        .await
        .map_err(|e| e.to_string())?;
    let total_rows_estimate = count_result
        .rows
        .first()
        .and_then(|row| row.get("count"))
        .and_then(|v| match v {
            QueryValue::Int(n) => Some(*n as u64),
            _ => None,
        });

    let formatted_preview = format_preview(&request.format, &columns, &sample_rows, preview_rows);

    Ok(ExportPreview {
        columns,
        sample_rows,
        total_rows_estimate,
        formatted_preview,
    })
}

fn format_preview(
    format: &ExportFormat,
    columns: &[String],
    rows: &[Vec<String>],
    _limit: u32,
) -> String {
    match format {
        ExportFormat::Csv => {
            let header = columns.join(",");
            let data_lines: Vec<String> = rows.iter().map(|row| row.join(",")).collect();
            format!("{}\n{}", header, data_lines.join("\n"))
        }
        ExportFormat::Jsonl => {
            let json_lines: Vec<String> = rows
                .iter()
                .map(|row| {
                    let mut obj = serde_json::Map::new();
                    for (col, val) in columns.iter().zip(row.iter()) {
                        obj.insert(col.clone(), JsonValue::String(val.clone()));
                    }
                    serde_json::to_string(&JsonValue::Object(obj)).unwrap_or_default()
                })
                .collect();
            json_lines.join("\n")
        }
        ExportFormat::Sql => {
            let col_list = columns
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ");

            let values_list: Vec<String> = rows
                .iter()
                .map(|row| {
                    let vals: Vec<String> = row
                        .iter()
                        .map(|v| {
                            if v.is_empty() {
                                "NULL".to_string()
                            }
                            else {
                                format!("'{}'", v)
                            }
                        })
                        .collect();
                    format!("({})", vals.join(", "))
                })
                .collect();

            format!(
                "INSERT INTO \"table\" ({}) VALUES {};",
                col_list,
                values_list.join(", ")
            )
        }
        ExportFormat::Excel => {
            let header = columns.join("\t");
            let data_lines: Vec<String> = rows.iter().map(|row| row.join("\t")).collect();
            format!("{}\n{}", header, data_lines.join("\n"))
        }
    }
}

// ── Excel helpers ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_chat2db_filename_pattern() {
        let filename = format_chat2db_filename("users", "csv");
        // Pattern: users_YYYYMMDDHHMMSS.csv
        assert!(filename.starts_with("users_"));
        assert!(filename.ends_with(".csv"));
        // The middle part should be 14 digits (YYYYMMDDHHMMSS)
        let timestamp_part = &filename["users_".len()..filename.len() - ".csv".len()];
        assert_eq!(timestamp_part.len(), 14);
        assert!(timestamp_part.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_format_chat2db_filename_different_extensions() {
        // Should work with any extension
        let name_sql = format_chat2db_filename("orders", "sql");
        assert!(name_sql.starts_with("orders_"));
        assert!(name_sql.ends_with(".sql"));

        let name_xlsx = format_chat2db_filename("orders", "xlsx");
        assert!(name_xlsx.starts_with("orders_"));
        assert!(name_xlsx.ends_with(".xlsx"));
    }

    #[test]
    fn test_format_chat2db_filename_special_chars() {
        // Table names with underscores should still produce valid filenames
        let filename = format_chat2db_filename("my_table", "csv");
        assert!(filename.starts_with("my_table_"));
        assert!(filename.ends_with(".csv"));
        // Should only have one underscore before timestamp
        let after_table = &filename["my_table".len()..];
        assert!(after_table.starts_with("_"));
    }

    #[test]
    fn test_format_extension_csv() {
        assert_eq!(format_extension(&ExportFormat::Csv), "csv");
    }

    #[test]
    fn test_format_extension_jsonl() {
        assert_eq!(format_extension(&ExportFormat::Jsonl), "jsonl");
    }

    #[test]
    fn test_format_extension_sql() {
        assert_eq!(format_extension(&ExportFormat::Sql), "sql");
    }

    #[test]
    fn test_format_extension_excel() {
        assert_eq!(format_extension(&ExportFormat::Excel), "xlsx");
    }

    #[test]
    fn test_format_extension_all_formats() {
        let cases = [
            (ExportFormat::Csv, "csv"),
            (ExportFormat::Jsonl, "jsonl"),
            (ExportFormat::Sql, "sql"),
            (ExportFormat::Excel, "xlsx"),
        ];
        for (format, expected) in &cases {
            assert_eq!(format_extension(format), *expected);
        }
    }
}

fn write_excel_cell(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: &QueryValue,
) -> Result<(), String> {
    match value {
        QueryValue::Null => Ok(()),
        QueryValue::Bool(b) => {
            worksheet
                .write_boolean(row, col, *b)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        QueryValue::Int(n) => {
            worksheet
                .write_number(row, col, *n as f64)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        QueryValue::Float(f) => {
            worksheet
                .write_number(row, col, *f)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        QueryValue::String(s) => {
            worksheet
                .write_string(row, col, s)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        QueryValue::Bytes(b) => {
            worksheet
                .write_string(row, col, hex::encode(b))
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        QueryValue::DateTime(dt) => {
            worksheet
                .write_string(row, col, dt)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}
