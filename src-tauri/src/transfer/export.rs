//! Export implementation for CSV, JSONL, SQL, and Excel formats.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use rust_xlsxwriter::{Workbook, Worksheet};
use serde_json::Value as JsonValue;

use super::defaults::*;
use super::progress::*;
use super::types::*;
use crate::database::{DatabaseAdapter, QueryValue};

/// Executes a data export operation.
pub async fn execute_export<A: DatabaseAdapter>(
    adapter: &A,
    request: ExportRequest,
    app_handle: &tauri::AppHandle,
) -> Result<TransferResult, String> {
    let start_time = Instant::now();
    let _operation_id = uuid::Uuid::new_v4().to_string();

    let columns = request.source.columns.clone();
    let table = request.source.table.clone();
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

    let base_query = build_export_query(&schema, &table, &columns, &request.source);

    let count_query = build_count_query(&schema, &table, &request.source.where_clause);
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
    let batch_size = 1000u64;

    match request.format {
        ExportFormat::Csv => {
            if csv_opts.include_header {
                write_csv_header(&mut writer, &columns, csv_opts.delimiter)
                    .map_err(|e| e.to_string())?;
            }

            let mut offset = 0u64;
            while offset < total_rows {
                let query = format!("{} LIMIT {} OFFSET {}", base_query, batch_size, offset);
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

                offset += batch_size;
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
                let query = format!("{} LIMIT {} OFFSET {}", base_query, batch_size, offset);
                let result = adapter
                    .execute_query(&query)
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    let json_obj = row_to_json_object(row, &jsonl_opts.date_format);
                    let json_line = serde_json::to_string(&json_obj).map_err(|e| e.to_string())?;
                    writer
                        .write_all(json_line.as_bytes())
                        .map_err(|e| e.to_string())?;
                    writer.write_all(b"\n").map_err(|e| e.to_string())?;
                    processed_rows += 1;
                }

                offset += batch_size;
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
                let query = format!("{} LIMIT {} OFFSET {}", base_query, batch_size, offset);
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

                offset += batch_size;
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
                let query = format!("{} LIMIT {} OFFSET {}", base_query, batch_size, offset);
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

                offset += batch_size;
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

fn build_export_query(
    schema: &Option<String>,
    table: &str,
    columns: &[String],
    source: &ExportSource,
) -> String {
    let schema_prefix = schema
        .as_ref()
        .map(|s| format!("\"{}\".", s))
        .unwrap_or_default();
    let cols = columns
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(", ");

    let mut query = format!("SELECT {} FROM {}\"{}\"", cols, schema_prefix, table);

    if let Some(ref where_clause) = source.where_clause {
        query.push_str(&format!(" WHERE {}", where_clause));
    }

    if let Some(ref order_by) = source.order_by {
        query.push_str(&format!(" ORDER BY {}", order_by));
    }

    query
}

fn build_count_query(
    schema: &Option<String>,
    table: &str,
    where_clause: &Option<String>,
) -> String {
    let schema_prefix = schema
        .as_ref()
        .map(|s| format!("\"{}\".", s))
        .unwrap_or_default();
    let mut query = format!(
        "SELECT COUNT(*) AS count FROM {}\"{}\"",
        schema_prefix, table
    );

    if let Some(ref where_clause) = where_clause {
        query.push_str(&format!(" WHERE {}", where_clause));
    }

    query
}

fn write_csv_header(
    writer: &mut BufWriter<File>,
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

fn write_csv_row(
    writer: &mut BufWriter<File>,
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
                } else {
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

/// Generates a preview of export data.
pub async fn preview_export<A: DatabaseAdapter>(
    adapter: &A,
    request: ExportRequest,
    preview_rows: u32,
) -> Result<ExportPreview, String> {
    let columns = request.source.columns.clone();
    let table = request.source.table.clone();
    let schema = request.schema.clone();

    let base_query = build_export_query(&schema, &table, &columns, &request.source);
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

    let count_query = build_count_query(&schema, &table, &request.source.where_clause);
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
                            } else {
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
