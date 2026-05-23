//! Cross-engine data migration implementation.

use std::time::Instant;

use crate::database::types::ColumnInfo;
use crate::database::DatabaseType;
use crate::database::{DatabaseAdapter, QueryValue};

use super::paginate_clause;
use super::progress::*;
use super::types::*;

fn quote_ident(name: &str, db_type: DatabaseType) -> String {
    match db_type {
        DatabaseType::MySQL => format!("`{}`", name.replace('`', "``")),
        DatabaseType::SqlServer => format!("[{}]", name.replace(']', "]]")),
        _ => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

fn source_table_reference(
    request: &MigrationRequest,
    db_type: DatabaseType,
    table: &str,
) -> String {
    match db_type {
        DatabaseType::MySQL => {
            let database_prefix = request
                .source_database
                .as_ref()
                .map(|db| format!("{}.", quote_ident(db, db_type)))
                .unwrap_or_default();
            format!("{}{}", database_prefix, quote_ident(table, db_type))
        }
        _ => {
            let schema_prefix = request
                .source_schema
                .as_ref()
                .map(|schema| format!("{}.", quote_ident(schema, db_type)))
                .unwrap_or_default();
            format!("{}{}", schema_prefix, quote_ident(table, db_type))
        }
    }
}

fn target_table_reference(
    request: &MigrationRequest,
    db_type: DatabaseType,
    table: &str,
) -> String {
    match db_type {
        DatabaseType::MySQL => {
            let database_prefix = request
                .target_database
                .as_ref()
                .map(|db| format!("{}.", quote_ident(db, db_type)))
                .unwrap_or_default();
            format!("{}{}", database_prefix, quote_ident(table, db_type))
        }
        _ => {
            let schema_prefix = request
                .target_schema
                .as_ref()
                .map(|schema| format!("{}.", quote_ident(schema, db_type)))
                .unwrap_or_default();
            format!("{}{}", schema_prefix, quote_ident(table, db_type))
        }
    }
}

pub async fn execute_migration<A1: DatabaseAdapter, A2: DatabaseAdapter>(
    source_adapter: &A1,
    target_adapter: &A2,
    request: MigrationRequest,
    app_handle: &tauri::AppHandle,
) -> Result<TransferResult, String> {
    let start_time = Instant::now();
    let mut total_processed: u64 = 0;
    let mut total_skipped: u64 = 0;
    let mut total_errors: Vec<TransferError> = Vec::new();

    emit_progress(
        app_handle,
        &create_progress("migration", "preparing", 0, None, 0),
    );

    for (table_idx, table_plan) in request.table_plans.iter().enumerate() {
        emit_progress(
            app_handle,
            &TransferProgress {
                operation: "migration".to_string(),
                phase: "processing".to_string(),
                current_table: Some(table_plan.source_table.clone()),
                total_rows: None,
                processed_rows: total_processed,
                skipped_rows: total_skipped,
                error_count: total_errors.len() as u64,
                percent: 0.0,
                elapsed_ms: start_time.elapsed().as_millis() as u64,
                estimated_remaining_ms: None,
                message: Some(format!(
                    "Migrating table {} of {}",
                    table_idx + 1,
                    request.table_plans.len()
                )),
            },
        );

        let table_result = migrate_table(
            source_adapter,
            target_adapter,
            &request,
            table_plan,
            app_handle,
            start_time,
        )
        .await;

        match table_result {
            Ok(result) => {
                total_processed += result.processed_rows;
                total_skipped += result.skipped_rows;
                if !result.success {
                    total_errors.extend(result.errors);
                }
            }
            Err(e) => {
                total_errors.push(TransferError {
                    row_number: None,
                    statement_number: None,
                    message: format!("Table {} failed: {}", table_plan.source_table, e),
                    sql: None,
                });
                if request.on_error == MigrationErrorStrategy::Abort {
                    break;
                }
            }
        }
    }

    emit_progress(
        app_handle,
        &create_progress(
            "migration",
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

async fn migrate_table<A1: DatabaseAdapter, A2: DatabaseAdapter>(
    source_adapter: &A1,
    target_adapter: &A2,
    request: &MigrationRequest,
    table_plan: &MigrationTablePlan,
    app_handle: &tauri::AppHandle,
    start_time: Instant,
) -> Result<TransferResult, String> {
    let mut processed_rows: u64 = 0;
    let mut skipped_rows: u64 = 0;
    let mut errors: Vec<TransferError> = Vec::new();

    let source_columns: Vec<String> = table_plan
        .column_mappings
        .iter()
        .map(|m| m.source_column.clone())
        .collect();

    let target_columns: Vec<String> = table_plan
        .column_mappings
        .iter()
        .map(|m| m.target_column.clone())
        .collect();

    let source_db_type = source_adapter.get_config().db_type;
    let source_table_ref =
        source_table_reference(request, source_db_type, &table_plan.source_table);

    let count_query = format!("SELECT COUNT(*) AS count FROM {}", source_table_ref);

    let count_result = source_adapter
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

    let col_list = source_columns
        .iter()
        .map(|column| quote_ident(column, source_db_type))
        .collect::<Vec<_>>()
        .join(", ");

    let base_query = format!("SELECT {} FROM {}", col_list, source_table_ref);

    let batch_size = request.batch_size as u64;
    let mut offset = 0u64;

    while offset < total_rows {
        let query = format!(
            "{} {}",
            base_query,
            paginate_clause(source_db_type, offset as usize, batch_size as usize, false)
        );
        let result = source_adapter
            .execute_query(&query)
            .await
            .map_err(|e| e.to_string())?;

        let insert_result = insert_batch_to_target(
            target_adapter,
            request,
            table_plan,
            &result.rows,
            &source_columns,
            &target_columns,
        )
        .await;

        match insert_result {
            Ok(count) => processed_rows += count,
            Err(e) => {
                errors.push(TransferError {
                    row_number: Some(processed_rows + 1),
                    statement_number: None,
                    message: e,
                    sql: None,
                });
                skipped_rows += result.rows.len() as u64;
            }
        }

        offset += batch_size;

        emit_progress(
            app_handle,
            &TransferProgress {
                operation: "migration".to_string(),
                phase: "processing".to_string(),
                current_table: Some(table_plan.source_table.clone()),
                total_rows: Some(total_rows),
                processed_rows,
                skipped_rows,
                error_count: errors.len() as u64,
                percent: if total_rows > 0 {
                    (processed_rows as f32 / total_rows as f32) * 100.0
                } else {
                    0.0
                },
                elapsed_ms: start_time.elapsed().as_millis() as u64,
                estimated_remaining_ms: None,
                message: None,
            },
        );
    }

    Ok(TransferResult {
        success: errors.is_empty(),
        total_rows,
        processed_rows,
        skipped_rows,
        error_count: errors.len() as u64,
        duration_ms: start_time.elapsed().as_millis() as u64,
        output_path: None,
        output_size_bytes: None,
        errors,
    })
}

async fn insert_batch_to_target<A: DatabaseAdapter>(
    target_adapter: &A,
    request: &MigrationRequest,
    table_plan: &MigrationTablePlan,
    rows: &[crate::database::QueryRow],
    source_columns: &[String],
    target_columns: &[String],
) -> Result<u64, String> {
    if rows.is_empty() {
        return Ok(0);
    }

    let target_db_type = target_adapter.get_config().db_type;
    let target_table_ref =
        target_table_reference(request, target_db_type, &table_plan.target_table);

    let col_list = target_columns
        .iter()
        .map(|column| quote_ident(column, target_db_type))
        .collect::<Vec<_>>()
        .join(", ");

    let values_list: Vec<String> = rows
        .iter()
        .map(|row| {
            let vals: Vec<String> = source_columns
                .iter()
                .map(|col| match row.get(col) {
                    Some(QueryValue::Null) => "NULL".to_string(),
                    Some(QueryValue::Bool(b)) => b.to_string(),
                    Some(QueryValue::Int(n)) => n.to_string(),
                    Some(QueryValue::Float(f)) => f.to_string(),
                    Some(QueryValue::String(s)) => format!("'{}'", s.replace('\'', "''")),
                    Some(QueryValue::Bytes(b)) => format!("'{}'", hex::encode(b)),
                    Some(QueryValue::DateTime(dt)) => format!("'{}'", dt),
                    None => "NULL".to_string(),
                })
                .collect();
            format!("({})", vals.join(", "))
        })
        .collect();

    let sql = format!(
        "INSERT INTO {} ({}) VALUES {}",
        target_table_ref,
        col_list,
        values_list.join(", ")
    );

    let result = target_adapter
        .execute_query(&sql)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.rows_affected.unwrap_or(rows.len() as u64))
}

pub async fn preview_migration<A: DatabaseAdapter>(
    source_adapter: &A,
    request: &MigrationRequest,
) -> Result<MigrationPreview, String> {
    let mut tables: Vec<MigrationTablePreview> = Vec::new();
    let mut total_rows: u64 = 0;
    let mut type_conversions: u64 = 0;

    for table_plan in &request.table_plans {
        let source_db_type = source_adapter.get_config().db_type;
        let source_table_ref =
            source_table_reference(request, source_db_type, &table_plan.source_table);

        let count_query = format!("SELECT COUNT(*) AS count FROM {}", source_table_ref);

        let count_result = source_adapter
            .execute_query(&count_query)
            .await
            .map_err(|e| e.to_string())?;
        let row_count = count_result
            .rows
            .first()
            .and_then(|row| row.get("count"))
            .and_then(|v| match v {
                QueryValue::Int(n) => Some(*n as u64),
                _ => None,
            })
            .unwrap_or(0);

        let conversions = table_plan
            .column_mappings
            .iter()
            .filter(|m| m.conversion != MigrationConversion::Direct)
            .count() as u64;

        tables.push(MigrationTablePreview {
            source_table: table_plan.source_table.clone(),
            target_table: table_plan.target_table.clone(),
            row_count,
            column_count: table_plan.column_mappings.len() as u64,
            mappings: table_plan.column_mappings.clone(),
        });

        total_rows += row_count;
        type_conversions += conversions;
    }

    Ok(MigrationPreview {
        tables,
        total_rows,
        type_conversions,
    })
}

pub fn auto_map_columns(
    source_columns: &[ColumnInfo],
    target_engine: DatabaseType,
) -> Vec<MigrationMapping> {
    source_columns
        .iter()
        .map(|col| {
            let target_type = map_type_to_engine(&col.data_type, target_engine);
            let conversion = if target_type == col.data_type {
                MigrationConversion::Direct
            } else {
                MigrationConversion::Mapped
            };

            MigrationMapping {
                source_column: col.name.clone(),
                source_type: col.data_type.clone(),
                target_column: col.name.clone(),
                target_type,
                conversion,
            }
        })
        .collect()
}

fn map_type_to_engine(source_type: &str, target_engine: DatabaseType) -> String {
    let source_lower = source_type.to_lowercase();

    match target_engine {
        DatabaseType::PostgreSQL => match source_lower.as_str() {
            "int" | "integer" => "INTEGER".to_string(),
            "bigint" => "BIGINT".to_string(),
            "smallint" => "SMALLINT".to_string(),
            "tinyint" => "SMALLINT".to_string(),
            "varchar" | "char" | "text" => "VARCHAR(255)".to_string(),
            "datetime" | "timestamp" => "TIMESTAMP".to_string(),
            "date" => "DATE".to_string(),
            "boolean" | "bool" | "tinyint(1)" => "BOOLEAN".to_string(),
            "float" | "double" => "DOUBLE PRECISION".to_string(),
            "decimal" | "numeric" => "NUMERIC".to_string(),
            "json" => "JSONB".to_string(),
            "blob" | "binary" => "BYTEA".to_string(),
            _ => source_type.to_string(),
        },
        DatabaseType::MySQL => match source_lower.as_str() {
            "int" | "integer" | "serial" => "INT".to_string(),
            "bigint" => "BIGINT".to_string(),
            "smallint" => "SMALLINT".to_string(),
            "boolean" | "bool" => "TINYINT(1)".to_string(),
            "varchar" | "text" => "VARCHAR(255)".to_string(),
            "datetime" | "timestamp" => "DATETIME".to_string(),
            "date" => "DATE".to_string(),
            "float" | "double precision" => "DOUBLE".to_string(),
            "decimal" | "numeric" => "DECIMAL".to_string(),
            "jsonb" => "JSON".to_string(),
            "bytea" => "BLOB".to_string(),
            _ => source_type.to_string(),
        },
        DatabaseType::SQLite => match source_lower.as_str() {
            "int" | "integer" | "bigint" | "smallint" | "tinyint" => "INTEGER".to_string(),
            "varchar" | "char" | "text" => "TEXT".to_string(),
            "datetime" | "timestamp" | "date" => "TEXT".to_string(),
            "boolean" | "bool" | "tinyint(1)" => "INTEGER".to_string(),
            "float" | "double" | "decimal" | "numeric" => "REAL".to_string(),
            "blob" | "binary" | "bytea" => "BLOB".to_string(),
            _ => source_type.to_string(),
        },
        DatabaseType::SqlServer => match source_lower.as_str() {
            "int" | "integer" | "serial" => "INT".to_string(),
            "bigint" => "BIGINT".to_string(),
            "smallint" => "SMALLINT".to_string(),
            "tinyint" => "TINYINT".to_string(),
            "varchar" | "text" => "NVARCHAR(255)".to_string(),
            "datetime" | "timestamp" => "DATETIME2".to_string(),
            "date" => "DATE".to_string(),
            "boolean" | "bool" | "tinyint(1)" => "BIT".to_string(),
            "float" | "double precision" => "FLOAT".to_string(),
            "decimal" | "numeric" => "DECIMAL".to_string(),
            "json" | "jsonb" => "NVARCHAR(MAX)".to_string(),
            "blob" | "binary" | "bytea" => "VARBINARY(MAX)".to_string(),
            _ => source_type.to_string(),
        },
        _ => source_type.to_string(),
    }
}
