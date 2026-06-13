//! Import implementation for CSV, JSONL, SQL, and Excel formats.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

use calamine::{open_workbook, Reader, Xlsx};

use super::defaults::*;
use super::progress::*;
use super::types::*;

/// Executes a data import operation.
/// Supports Tables scope (import to specific tables), Database scope (create database if needed),
/// and Server scope (execute SQL directly at connection level).
pub async fn execute_import<A: crate::database::DatabaseAdapter>(
    adapter: &A,
    request: ImportRequest,
    app_handle: &tauri::AppHandle,
) -> Result<TransferResult, String> {
    let start_time = Instant::now();
    let mut total_processed: u64 = 0;
    let mut total_skipped: u64 = 0;
    let mut total_errors: Vec<TransferError> = Vec::new();

    emit_progress(
        app_handle,
        &create_progress("import", "preparing", 0, None, 0),
    );

    match request.scope {
        TransferScope::Server => {
            // Server scope: execute SQL file directly at connection level
            // SQL files may contain CREATE DATABASE statements
            if let Some(target) = request.tables.first() {
                if target.format == ImportFormat::Sql {
                    let target_result =
                        import_sql_at_server_level(adapter, target, app_handle, &start_time)
                            .await?;
                    total_processed += target_result.processed_rows;
                    total_skipped += target_result.skipped_rows;
                    total_errors.extend(target_result.errors);
                } else {
                    return Err("Server scope import only supports SQL files".to_string());
                }
            }
        }
        TransferScope::Database => {
            // Database scope: create database if needed, then import tables
            if request.create_database_if_not_exists.unwrap_or(false) {
                if let Some(ref db_name) = request.database {
                    let db_exists = adapter
                        .list_databases()
                        .await
                        .map_err(|e| e.to_string())?
                        .iter()
                        .any(|d| &d.name == db_name);
                    if !db_exists {
                        adapter
                            .execute_query(&format!("CREATE DATABASE \"{}\"", db_name))
                            .await
                            .map_err(|e| format!("Failed to create database: {}", e))?;
                    }
                }
            }
            for target in &request.tables {
                let target_result =
                    import_target(adapter, &request, target, app_handle, &start_time).await?;
                total_processed += target_result.processed_rows;
                total_skipped += target_result.skipped_rows;
                total_errors.extend(target_result.errors);
            }
        }
        TransferScope::Tables => {
            // Tables scope: import to specific tables (current behavior)
            for target in &request.tables {
                let target_result =
                    import_target(adapter, &request, target, app_handle, &start_time).await?;
                total_processed += target_result.processed_rows;
                total_skipped += target_result.skipped_rows;
                total_errors.extend(target_result.errors);
            }
        }
    }

    emit_progress(
        app_handle,
        &create_progress(
            "import",
            "finalizing",
            total_processed,
            None,
            start_time.elapsed().as_millis() as u64,
        ),
    );

    Ok(TransferResult {
        success: total_errors.is_empty(),
        total_rows: total_processed + total_skipped,
        processed_rows: total_processed,
        skipped_rows: total_skipped,
        error_count: total_errors.len() as u64,
        duration_ms: start_time.elapsed().as_millis() as u64,
        output_path: None,
        output_size_bytes: None,
        errors: total_errors,
    })
}

/// Import SQL file at server level (can contain CREATE DATABASE statements).
async fn import_sql_at_server_level<A: crate::database::DatabaseAdapter>(
    adapter: &A,
    target: &ImportTarget,
    app_handle: &tauri::AppHandle,
    start_time: &Instant,
) -> Result<TransferResult, String> {
    let file_path = Path::new(&target.file_path);
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read SQL file: {}", e))?;

    let mut processed_rows: u64 = 0;
    let mut skipped_rows: u64 = 0;
    let mut errors: Vec<TransferError> = Vec::new();

    // Split by semicolon and execute each statement
    for (idx, stmt) in content
        .split(';')
        .filter(|s| !s.trim().is_empty())
        .enumerate()
    {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        let result = adapter.execute_query(stmt).await;
        match result {
            Ok(_) => {
                processed_rows += 1;
            }
            Err(e) => {
                errors.push(TransferError {
                    row_number: None,
                    statement_number: Some(idx as u64 + 1),
                    message: e.to_string(),
                    sql: Some(stmt.to_string()),
                });
                skipped_rows += 1;
            }
        }

        emit_progress(
            app_handle,
            &create_progress(
                "import",
                "processing",
                processed_rows,
                None,
                start_time.elapsed().as_millis() as u64,
            ),
        );
    }

    Ok(TransferResult {
        success: errors.is_empty(),
        total_rows: processed_rows + skipped_rows,
        processed_rows,
        skipped_rows,
        error_count: errors.len() as u64,
        duration_ms: start_time.elapsed().as_millis() as u64,
        output_path: None,
        output_size_bytes: None,
        errors,
    })
}

async fn import_target<A: crate::database::DatabaseAdapter>(
    adapter: &A,
    request: &ImportRequest,
    target: &ImportTarget,
    app_handle: &tauri::AppHandle,
    start_time: &Instant,
) -> Result<TransferResult, String> {
    let csv_opts = target
        .csv_options
        .clone()
        .unwrap_or_else(csv_import_defaults);

    let file_path = Path::new(&target.file_path);
    let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;

    let mut processed_rows: u64 = 0;
    let mut skipped_rows: u64 = 0;
    let mut errors: Vec<TransferError> = Vec::new();

    match target.format {
        ImportFormat::Csv => {
            let reader = BufReader::new(file);
            let delimiter = csv_opts.delimiter;

            let mut lines = reader.lines().peekable();

            let _header_line = if csv_opts.has_header {
                lines
                    .next()
                    .transpose()
                    .map_err(|e| format!("Failed to read header: {}", e))?
                    .unwrap_or_default()
            } else {
                target
                    .column_mappings
                    .iter()
                    .map(|m| m.source_column.clone())
                    .collect::<Vec<_>>()
                    .join(&delimiter.to_string())
            };

            let header_columns: Vec<String> = if csv_opts.has_header {
                parse_csv_line(&_header_line, delimiter)
            } else {
                target
                    .column_mappings
                    .iter()
                    .map(|m| m.source_column.clone())
                    .collect()
            };

            let mut batch_values: Vec<Vec<String>> = Vec::new();

            for (row_num, line_result) in lines.enumerate() {
                let line = line_result
                    .map_err(|e| format!("Failed to read line {}: {}", row_num + 2, e))?;

                if line.trim().is_empty() {
                    continue;
                }

                let values = parse_csv_line(&line, delimiter);

                let mapped_values: Vec<String> = header_columns
                    .iter()
                    .enumerate()
                    .filter_map(|(i, col)| {
                        let mapping = target
                            .column_mappings
                            .iter()
                            .find(|m| m.source_column == *col);
                        if mapping.is_none()
                            || mapping.and_then(|m| m.target_column.as_ref()).is_none()
                        {
                            None
                        } else {
                            Some(values.get(i).cloned().unwrap_or_default())
                        }
                    })
                    .collect();

                batch_values.push(mapped_values);
                processed_rows += 1;

                if batch_values.len() >= request.batch_size as usize {
                    let insert_result = execute_batch_insert_for_target(
                        adapter,
                        request,
                        target,
                        &batch_values,
                        processed_rows - batch_values.len() as u64,
                    )
                    .await;

                    match insert_result {
                        Ok(count) => {
                            processed_rows = processed_rows - batch_values.len() as u64 + count;
                        }
                        Err(e) => {
                            errors.push(TransferError {
                                row_number: Some(processed_rows - batch_values.len() as u64 + 1),
                                statement_number: None,
                                message: e,
                                sql: None,
                            });
                            skipped_rows += batch_values.len() as u64;
                        }
                    }

                    batch_values.clear();
                    emit_progress(
                        app_handle,
                        &create_progress(
                            "import",
                            "processing",
                            processed_rows,
                            None,
                            start_time.elapsed().as_millis() as u64,
                        ),
                    );
                }
            }

            if !batch_values.is_empty() {
                let insert_result = execute_batch_insert_for_target(
                    adapter,
                    request,
                    target,
                    &batch_values,
                    processed_rows - batch_values.len() as u64,
                )
                .await;

                match insert_result {
                    Ok(count) => {
                        processed_rows = processed_rows - batch_values.len() as u64 + count;
                    }
                    Err(e) => {
                        errors.push(TransferError {
                            row_number: Some(processed_rows - batch_values.len() as u64 + 1),
                            statement_number: None,
                            message: e,
                            sql: None,
                        });
                        skipped_rows += batch_values.len() as u64;
                    }
                }
            }
        }

        ImportFormat::Jsonl => {
            let reader = BufReader::new(file);
            let mut batch_values: Vec<Vec<String>> = Vec::new();
            let _target_columns: Vec<String> = target
                .column_mappings
                .iter()
                .filter_map(|m| m.target_column.clone())
                .collect();

            for (row_num, line_result) in reader.lines().enumerate() {
                let line = line_result
                    .map_err(|e| format!("Failed to read line {}: {}", row_num + 1, e))?;

                if line.trim().is_empty() {
                    continue;
                }

                let json_obj: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                    errors.push(TransferError {
                        row_number: Some(row_num as u64 + 1),
                        statement_number: None,
                        message: format!("JSON parse error: {}", e),
                        sql: None,
                    });
                    String::new()
                })?;

                if !json_obj.is_object() {
                    skipped_rows += 1;
                    continue;
                }

                let obj = json_obj.as_object().unwrap();
                let values: Vec<String> = target
                    .column_mappings
                    .iter()
                    .filter_map(|m| {
                        let _target_col = m.target_column.as_ref()?;
                        let source_val = obj.get(&m.source_column);
                        match source_val {
                            Some(serde_json::Value::Null) => Some(String::new()),
                            Some(serde_json::Value::Bool(b)) => Some(b.to_string()),
                            Some(serde_json::Value::Number(n)) => Some(n.to_string()),
                            Some(serde_json::Value::String(s)) => Some(s.clone()),
                            Some(serde_json::Value::Array(arr)) => {
                                Some(serde_json::to_string(arr).unwrap_or_default())
                            }
                            Some(serde_json::Value::Object(obj)) => {
                                Some(serde_json::to_string(obj).unwrap_or_default())
                            }
                            None => Some(String::new()),
                        }
                    })
                    .collect();

                batch_values.push(values);
                processed_rows += 1;

                if batch_values.len() >= request.batch_size as usize {
                    let insert_result = execute_batch_insert_for_target(
                        adapter,
                        request,
                        target,
                        &batch_values,
                        processed_rows - batch_values.len() as u64,
                    )
                    .await;

                    match insert_result {
                        Ok(count) => {
                            processed_rows = processed_rows - batch_values.len() as u64 + count;
                        }
                        Err(e) => {
                            errors.push(TransferError {
                                row_number: Some(processed_rows - batch_values.len() as u64 + 1),
                                statement_number: None,
                                message: e,
                                sql: None,
                            });
                            skipped_rows += batch_values.len() as u64;
                        }
                    }

                    batch_values.clear();
                    emit_progress(
                        app_handle,
                        &create_progress(
                            "import",
                            "processing",
                            processed_rows,
                            None,
                            start_time.elapsed().as_millis() as u64,
                        ),
                    );
                }
            }

            if !batch_values.is_empty() {
                execute_batch_insert_for_target(
                    adapter,
                    request,
                    target,
                    &batch_values,
                    processed_rows - batch_values.len() as u64,
                )
                .await?;
            }
        }

        ImportFormat::Sql => {
            let reader = BufReader::new(file);
            let mut current_statement = String::new();
            let mut statement_count: u64 = 0;

            for line_result in reader.lines() {
                let line = line_result.map_err(|e| format!("Failed to read line: {}", e))?;

                if line.trim().is_empty() {
                    continue;
                }

                current_statement.push_str(&line);
                current_statement.push('\n');

                if line.trim().ends_with(';') {
                    let sql = current_statement.trim();
                    if !sql.is_empty() {
                        statement_count += 1;
                        if request.dry_run {
                            processed_rows += 1;
                        } else {
                            match adapter.execute_query(sql).await {
                                Ok(_) => processed_rows += 1,
                                Err(e) => {
                                    errors.push(TransferError {
                                        row_number: None,
                                        statement_number: Some(statement_count),
                                        message: e.to_string(),
                                        sql: Some(sql.to_string()),
                                    });
                                    skipped_rows += 1;
                                }
                            }
                        }
                    }
                    current_statement.clear();

                    emit_progress(
                        app_handle,
                        &create_progress(
                            "import",
                            "processing",
                            processed_rows,
                            None,
                            start_time.elapsed().as_millis() as u64,
                        ),
                    );
                }
            }

            if !current_statement.trim().is_empty() {
                statement_count += 1;
                if request.dry_run {
                    processed_rows += 1;
                } else {
                    match adapter.execute_query(current_statement.trim()).await {
                        Ok(_) => processed_rows += 1,
                        Err(e) => {
                            errors.push(TransferError {
                                row_number: None,
                                statement_number: Some(statement_count),
                                message: e.to_string(),
                                sql: Some(current_statement.trim().to_string()),
                            });
                            skipped_rows += 1;
                        }
                    }
                }
            }
        }

        ImportFormat::Excel => {
            let mut workbook: Xlsx<_> = open_workbook(file_path)
                .map_err(|e| format!("Failed to open Excel file: {}", e))?;

            let sheet_name = target
                .excel_options
                .as_ref()
                .map(|o| o.sheet_name.clone())
                .unwrap_or_else(|| "Sheet1".to_string());

            let range = workbook
                .worksheet_range(&sheet_name)
                .ok_or_else(|| format!("Sheet '{}' not found", sheet_name))?
                .map_err(|e| format!("Failed to read sheet '{}': {:?}", sheet_name, e))?;

            let has_header = target
                .excel_options
                .as_ref()
                .map(|o| o.has_header)
                .unwrap_or(true);

            let mut rows_iter = range.rows();
            let header_row: Vec<String> = if has_header {
                rows_iter
                    .next()
                    .map(|row| {
                        row.iter()
                            .map(|c: &calamine::DataType| c.to_string())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                target
                    .column_mappings
                    .iter()
                    .map(|m| m.source_column.clone())
                    .collect()
            };

            let mut batch_values: Vec<Vec<String>> = Vec::new();

            for row in range.rows() {
                let values: Vec<String> = header_row
                    .iter()
                    .enumerate()
                    .filter_map(|(col_idx, col)| {
                        let mapping = target
                            .column_mappings
                            .iter()
                            .find(|m| m.source_column == *col);
                        if mapping.is_none()
                            || mapping.and_then(|m| m.target_column.as_ref()).is_none()
                        {
                            None
                        } else {
                            Some(
                                row.get(col_idx)
                                    .map(|c: &calamine::DataType| c.to_string())
                                    .unwrap_or_default(),
                            )
                        }
                    })
                    .collect();

                batch_values.push(values);
                processed_rows += 1;

                if batch_values.len() >= request.batch_size as usize {
                    let insert_result = execute_batch_insert_for_target(
                        adapter,
                        request,
                        target,
                        &batch_values,
                        processed_rows - batch_values.len() as u64,
                    )
                    .await;

                    match insert_result {
                        Ok(count) => {
                            processed_rows = processed_rows - batch_values.len() as u64 + count;
                        }
                        Err(e) => {
                            errors.push(TransferError {
                                row_number: Some(processed_rows - batch_values.len() as u64 + 1),
                                statement_number: None,
                                message: e,
                                sql: None,
                            });
                            skipped_rows += batch_values.len() as u64;
                        }
                    }

                    batch_values.clear();
                    emit_progress(
                        app_handle,
                        &create_progress(
                            "import",
                            "processing",
                            processed_rows,
                            None,
                            start_time.elapsed().as_millis() as u64,
                        ),
                    );
                }
            }

            if !batch_values.is_empty() {
                let insert_result = execute_batch_insert_for_target(
                    adapter,
                    request,
                    target,
                    &batch_values,
                    processed_rows - batch_values.len() as u64,
                )
                .await;

                match insert_result {
                    Ok(count) => {
                        processed_rows = processed_rows - batch_values.len() as u64 + count;
                    }
                    Err(e) => {
                        errors.push(TransferError {
                            row_number: Some(processed_rows - batch_values.len() as u64 + 1),
                            statement_number: None,
                            message: e,
                            sql: None,
                        });
                        skipped_rows += batch_values.len() as u64;
                    }
                }
            }
        }
    }

    Ok(TransferResult {
        success: errors.is_empty(),
        total_rows: processed_rows + skipped_rows,
        processed_rows,
        skipped_rows,
        error_count: errors.len() as u64,
        duration_ms: start_time.elapsed().as_millis() as u64,
        output_path: None,
        output_size_bytes: None,
        errors,
    })
}

fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
    let mut values: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars = line.chars().peekable();

    for ch in chars {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == delimiter && !in_quotes {
            values.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    values.push(current.trim().to_string());
    values
}

async fn execute_batch_insert_for_target<A: crate::database::DatabaseAdapter>(
    adapter: &A,
    request: &ImportRequest,
    target: &ImportTarget,
    batch: &[Vec<String>],
    _start_row: u64,
) -> Result<u64, String> {
    if batch.is_empty() {
        return Ok(0);
    }

    let schema_prefix = request
        .schema
        .as_ref()
        .map(|s| format!("\"{}\".", s))
        .unwrap_or_default();
    let target_columns: Vec<String> = target
        .column_mappings
        .iter()
        .filter_map(|m| m.target_column.clone())
        .collect();

    let col_list = target_columns
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(", ");

    let values_list: Vec<String> = batch
        .iter()
        .map(|row| {
            let vals: Vec<String> = row
                .iter()
                .map(|v| {
                    if v.is_empty() {
                        "NULL".to_string()
                    } else {
                        format!("'{}'", v.replace('\'', "''"))
                    }
                })
                .collect();
            format!("({})", vals.join(", "))
        })
        .collect();

    let sql = format!(
        "INSERT INTO {}\"{}\" ({}) VALUES {}",
        schema_prefix,
        target.table,
        col_list,
        values_list.join(", ")
    );

    if request.dry_run {
        return Ok(batch.len() as u64);
    }

    let result = adapter
        .execute_query(&sql)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.rows_affected.unwrap_or(batch.len() as u64))
}

/// Detects file format and metadata.
pub fn detect_file(file_path: &str) -> Result<FileDetectionResult, String> {
    let path = Path::new(file_path);
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;

    let file_size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    let format = match extension.as_deref() {
        Some("csv") => ImportFormat::Csv,
        Some("jsonl") | Some("json") => ImportFormat::Jsonl,
        Some("sql") => ImportFormat::Sql,
        Some("xlsx") | Some("xls") => ImportFormat::Excel,
        _ => return Err("Unknown file format".to_string()),
    };

    let reader = BufReader::new(file);
    let mut lines_iter = reader.lines();

    let first_line = lines_iter
        .next()
        .transpose()
        .map_err(|e| format!("Failed to read file: {}", e))?
        .unwrap_or_default();

    let (columns, csv_delimiter, has_header) = match format {
        ImportFormat::Csv => {
            let delimiter = detect_csv_delimiter(&first_line);
            let cols = parse_csv_line(&first_line, delimiter);
            let has_header = cols
                .iter()
                .all(|c| !c.is_empty() && !c.chars().all(char::is_numeric));
            (cols, Some(delimiter), Some(has_header))
        }
        ImportFormat::Jsonl => {
            let json_obj: serde_json::Value = serde_json::from_str(&first_line)
                .map_err(|_| "Invalid JSONL format".to_string())?;
            let cols = json_obj
                .as_object()
                .map(|obj| obj.keys().cloned().collect())
                .unwrap_or_default();
            (cols, None, None)
        }
        ImportFormat::Sql => (Vec::new(), None, None),
        ImportFormat::Excel => {
            let mut workbook: Xlsx<_> = open_workbook(file_path)
                .map_err(|e| format!("Failed to open Excel file for detection: {}", e))?;
            let range = workbook
                .worksheet_range("Sheet1")
                .ok_or("Sheet 'Sheet1' not found")?
                .map_err(|e| format!("Failed to read sheet: {:?}", e))?;
            let cols = range
                .rows()
                .next()
                .map(|row| {
                    row.iter()
                        .map(|c: &calamine::DataType| c.to_string())
                        .collect()
                })
                .unwrap_or_default();
            (cols, None, Some(true))
        }
    };

    let estimated_rows = estimate_row_count(file_path, file_size_bytes, &format);

    Ok(FileDetectionResult {
        format,
        encoding: "UTF-8".to_string(),
        estimated_rows,
        file_size_bytes,
        columns,
        csv_delimiter,
        has_header,
    })
}

fn detect_csv_delimiter(line: &str) -> char {
    let comma_count = line.chars().filter(|&c| c == ',').count();
    let tab_count = line.chars().filter(|&c| c == '\t').count();
    let semicolon_count = line.chars().filter(|&c| c == ';').count();

    if tab_count > comma_count && tab_count > semicolon_count {
        '\t'
    } else if semicolon_count > comma_count {
        ';'
    } else {
        ','
    }
}

fn estimate_row_count(file_path: &str, file_size: u64, _format: &ImportFormat) -> Option<u64> {
    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    let sample_lines: Vec<String> = reader.lines().take(100).filter_map(|l| l.ok()).collect();

    if sample_lines.is_empty() {
        return None;
    }

    let avg_line_size = sample_lines.iter().map(|l| l.len()).sum::<usize>() / sample_lines.len();

    if avg_line_size == 0 {
        return None;
    }

    Some(file_size / avg_line_size as u64)
}

/// Generates a preview of import data.
pub fn preview_import(
    file_path: &str,
    format: ImportFormat,
    preview_rows: u32,
) -> Result<ExportPreview, String> {
    let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);

    let mut columns: Vec<String> = Vec::new();
    let mut sample_rows: Vec<Vec<String>> = Vec::new();

    match format {
        ImportFormat::Csv => {
            let mut lines_iter = reader.lines();

            let first_line = lines_iter
                .next()
                .transpose()
                .map_err(|e| format!("Failed to read file: {}", e))?
                .unwrap_or_default();

            let delimiter = detect_csv_delimiter(&first_line);
            columns = parse_csv_line(&first_line, delimiter);

            for line_result in lines_iter.take(preview_rows as usize) {
                let line = line_result.map_err(|e| format!("Failed to read line: {}", e))?;
                sample_rows.push(parse_csv_line(&line, delimiter));
            }
        }
        ImportFormat::Jsonl => {
            for (i, line_result) in reader.lines().enumerate() {
                if i > preview_rows as usize {
                    break;
                }

                let line = line_result.map_err(|e| format!("Failed to read line: {}", e))?;
                let json_obj: serde_json::Value =
                    serde_json::from_str(&line).map_err(|e| format!("JSON parse error: {}", e))?;

                if i == 0 && json_obj.is_object() {
                    columns = json_obj.as_object().unwrap().keys().cloned().collect();
                }

                if json_obj.is_object() {
                    let obj = json_obj.as_object().unwrap();
                    let row: Vec<String> = columns
                        .iter()
                        .map(|col| match obj.get(col) {
                            Some(serde_json::Value::Null) => String::new(),
                            Some(serde_json::Value::Bool(b)) => b.to_string(),
                            Some(serde_json::Value::Number(n)) => n.to_string(),
                            Some(serde_json::Value::String(s)) => s.clone(),
                            Some(v) => v.to_string(),
                            None => String::new(),
                        })
                        .collect();
                    sample_rows.push(row);
                }
            }
        }
        ImportFormat::Sql => {
            let mut statements: Vec<String> = Vec::new();
            let mut current_statement = String::new();

            for line_result in reader.lines().take(50) {
                let line = line_result.map_err(|e| format!("Failed to read line: {}", e))?;
                if line.trim().is_empty() {
                    continue;
                }
                current_statement.push_str(&line);
                current_statement.push('\n');
                if line.trim().ends_with(';') {
                    statements.push(current_statement.trim().to_string());
                    current_statement.clear();
                }
            }

            columns = vec!["statement".to_string()];
            sample_rows = statements
                .iter()
                .take(preview_rows as usize)
                .map(|s| vec![s.clone()])
                .collect();
        }
        ImportFormat::Excel => {
            let mut workbook: Xlsx<_> = open_workbook(file_path)
                .map_err(|e| format!("Failed to open Excel file: {}", e))?;

            let sheet_name = "Sheet1";
            let range = workbook
                .worksheet_range(sheet_name)
                .ok_or_else(|| format!("Sheet '{}' not found", sheet_name))?
                .map_err(|e| format!("Failed to read sheet: {:?}", e))?;

            let has_header = true;

            if has_header {
                columns = range
                    .rows()
                    .next()
                    .map(|row| {
                        row.iter()
                            .map(|c: &calamine::DataType| c.to_string())
                            .collect()
                    })
                    .unwrap_or_default();
            }

            for (row_idx, row) in range.rows().enumerate() {
                if row_idx == 0 && has_header {
                    continue;
                }
                if sample_rows.len() >= preview_rows as usize {
                    break;
                }
                let row_values: Vec<String> = row
                    .iter()
                    .map(|cell: &calamine::DataType| cell.to_string())
                    .collect();
                sample_rows.push(row_values);
            }
        }
    }

    Ok(ExportPreview {
        columns,
        sample_rows,
        total_rows_estimate: None,
        formatted_preview: String::new(),
    })
}
