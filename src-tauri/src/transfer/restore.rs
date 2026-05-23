use crate::database::{DatabaseAdapter, DatabaseType, DbError, DbResult};
use calamine::{open_workbook, Data, Reader, Xlsx, XlsxError};
use serde::Serialize;
use std::fs;
use std::io::BufReader;

#[derive(Debug, Clone)]
pub struct RestoreOptions {
    pub progress_every: usize,
    pub csv_delimiter: u8,
    pub csv_has_header: bool,
    pub xlsx_sheet_name: Option<String>,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self {
            progress_every: 100,
            csv_delimiter: b',',
            csv_has_header: true,
            xlsx_sheet_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RestoreStats {
    pub statements_total: u64,
    pub statements_succeeded: u64,
    pub rows_inserted: u64,
    pub errors: Vec<String>,
}

const CSV_BATCH_SIZE: usize = 500;

/// Restores a SQL dump by executing statements sequentially.
pub async fn restore_sql_file<A: DatabaseAdapter>(
    adapter: &A,
    path: &str,
    options: &RestoreOptions,
) -> DbResult<RestoreStats> {
    restore_sql_file_with_progress(adapter, path, options, |_, _| {}).await
}

pub(crate) async fn restore_sql_file_with_progress<A: DatabaseAdapter, F: FnMut(u64, u64)>(
    adapter: &A,
    path: &str,
    options: &RestoreOptions,
    mut on_progress: F,
) -> DbResult<RestoreStats> {
    let content = fs::read_to_string(path)?;
    let statements = split_sql_statements(&content);
    let total = statements.len() as u64;

    let mut index = 0usize;
    let mut stats = RestoreStats {
        statements_total: total,
        ..RestoreStats::default()
    };

    while index < statements.len() {
        let statement = statements[index].trim();
        if !statement.is_empty() {
            match adapter.execute_query(statement).await {
                Ok(_) => {
                    stats.statements_succeeded += 1;
                }
                Err(error) => {
                    stats
                        .errors
                        .push(format!("statement {}: {}", index + 1, error));
                }
            }
        }

        let current = index as u64 + 1;
        if options.progress_every > 0 && (index + 1).is_multiple_of(options.progress_every)
            || current == total
        {
            on_progress(current, total);
        }

        index += 1;
    }

    Ok(stats)
}

/// Restores CSV data into a target table in 500-row insert batches.
pub async fn restore_csv_file<A: DatabaseAdapter>(
    adapter: &A,
    path: &str,
    target_schema: Option<&str>,
    target_table: &str,
    options: &RestoreOptions,
) -> DbResult<RestoreStats> {
    restore_csv_file_with_progress(
        adapter,
        path,
        target_schema,
        target_table,
        options,
        |_, _| {},
    )
    .await
}

pub(crate) async fn restore_csv_file_with_progress<A: DatabaseAdapter, F: FnMut(u64, u64)>(
    adapter: &A,
    path: &str,
    target_schema: Option<&str>,
    target_table: &str,
    options: &RestoreOptions,
    mut on_progress: F,
) -> DbResult<RestoreStats> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(options.csv_delimiter)
        .has_headers(options.csv_has_header)
        .from_path(path)
        .map_err(|error| DbError::InvalidQuery(format!("Failed to parse CSV file: {}", error)))?;

    let columns = if options.csv_has_header {
        reader
            .headers()
            .map_err(|error| {
                DbError::InvalidQuery(format!("Failed to read CSV headers: {}", error))
            })?
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    } else {
        let first_record = reader
            .records()
            .next()
            .transpose()
            .map_err(|error| DbError::InvalidQuery(format!("Failed to parse CSV row: {}", error)))?
            .ok_or_else(|| {
                DbError::InvalidQuery("CSV file is empty and no headers were provided".to_string())
            })?;

        (0..first_record.len())
            .map(|index| format!("column_{}", index + 1))
            .collect::<Vec<_>>()
    };

    let mut records_reader = csv::ReaderBuilder::new()
        .delimiter(options.csv_delimiter)
        .has_headers(options.csv_has_header)
        .from_path(path)
        .map_err(|error| DbError::InvalidQuery(format!("Failed to parse CSV file: {}", error)))?;
    let records = records_reader.records();
    let mut current_batch: Vec<Vec<String>> = Vec::new();

    if columns.is_empty() {
        return Err(DbError::InvalidQuery(
            "CSV file has no columns to restore".to_string(),
        ));
    }

    let mut stats = RestoreStats::default();

    for record in records {
        let record = record.map_err(|error| {
            DbError::InvalidQuery(format!("Failed to parse CSV row: {}", error))
        })?;

        current_batch.push(
            record
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        );

        if current_batch.len() >= CSV_BATCH_SIZE {
            let batch_stats = restore_rows_in_batches(
                adapter,
                target_schema,
                target_table,
                &columns,
                &current_batch,
                &mut on_progress,
            )
            .await?;
            merge_stats(&mut stats, batch_stats);
            current_batch.clear();
        }
    }

    if !current_batch.is_empty() {
        let batch_stats = restore_rows_in_batches(
            adapter,
            target_schema,
            target_table,
            &columns,
            &current_batch,
            &mut on_progress,
        )
        .await?;
        merge_stats(&mut stats, batch_stats);
    }

    Ok(stats)
}

/// Restores XLSX rows from one sheet into a target table in 500-row insert batches.
pub async fn restore_xlsx_file<A: DatabaseAdapter>(
    adapter: &A,
    path: &str,
    target_schema: Option<&str>,
    target_table: &str,
    options: &RestoreOptions,
) -> DbResult<RestoreStats> {
    restore_xlsx_file_with_progress(
        adapter,
        path,
        target_schema,
        target_table,
        options,
        |_, _| {},
    )
    .await
}

pub(crate) async fn restore_xlsx_file_with_progress<A: DatabaseAdapter, F: FnMut(u64, u64)>(
    adapter: &A,
    path: &str,
    target_schema: Option<&str>,
    target_table: &str,
    options: &RestoreOptions,
    mut on_progress: F,
) -> DbResult<RestoreStats> {
    let mut workbook: Xlsx<BufReader<std::fs::File>> =
        open_workbook::<Xlsx<BufReader<std::fs::File>>, _>(path)
            .map_err(|error: XlsxError| DbError::InvalidQuery(error.to_string()))?;

    let sheet_name = options
        .xlsx_sheet_name
        .clone()
        .or_else(|| workbook.sheet_names().first().cloned())
        .ok_or_else(|| DbError::InvalidQuery("XLSX file has no worksheets".to_string()))?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|error: XlsxError| DbError::InvalidQuery(error.to_string()))?;

    let mut rows_iter = range.rows();
    let columns = rows_iter
        .next()
        .map(|row| {
            row.iter()
                .map(|cell: &Data| cell.to_string())
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| DbError::InvalidQuery("XLSX sheet is empty".to_string()))?;

    let mut stats = RestoreStats::default();
    let mut current_batch: Vec<Vec<String>> = Vec::new();

    for row in rows_iter {
        current_batch.push(
            row.iter()
                .map(|cell: &Data| cell.to_string())
                .collect::<Vec<_>>(),
        );

        if current_batch.len() >= CSV_BATCH_SIZE {
            let batch_stats = restore_rows_in_batches(
                adapter,
                target_schema,
                target_table,
                &columns,
                &current_batch,
                &mut on_progress,
            )
            .await?;
            merge_stats(&mut stats, batch_stats);
            current_batch.clear();
        }
    }

    if !current_batch.is_empty() {
        let batch_stats = restore_rows_in_batches(
            adapter,
            target_schema,
            target_table,
            &columns,
            &current_batch,
            &mut on_progress,
        )
        .await?;
        merge_stats(&mut stats, batch_stats);
    }

    Ok(stats)
}

pub(crate) fn split_sql_statements(content: &str) -> Vec<String> {
    fn read_dollar_tag(chars: &[char], start: usize) -> Option<(String, usize)> {
        if chars.get(start) != Some(&'$') {
            return None;
        }

        let mut end = start + 1;
        while end < chars.len() {
            if chars[end] == '$' {
                let tag = chars[start + 1..end].iter().collect::<String>();
                let valid = if tag.is_empty() {
                    true
                } else {
                    let mut tag_chars = tag.chars();
                    matches!(tag_chars.next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_')
                        && tag_chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                };

                return if valid { Some((tag, end + 1)) } else { None };
            }
            end += 1;
        }

        None
    }

    let chars = content.chars().collect::<Vec<_>>();
    let mut current = String::new();
    let mut statements: Vec<String> = Vec::new();

    let mut index = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut in_dollar: Option<String> = None;

    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();

        if let Some(dollar_tag) = in_dollar.as_ref() {
            if ch == '$' {
                if let Some((tag, next_index)) = read_dollar_tag(&chars, index) {
                    if &tag == dollar_tag {
                        current.push_str(&chars[index..next_index].iter().collect::<String>());
                        in_dollar = None;
                        index = next_index;
                        continue;
                    }
                }
            }

            current.push(ch);
            index += 1;
            continue;
        }

        if in_line_comment {
            current.push(ch);
            if ch == '\n' {
                in_line_comment = false;
            }
            index += 1;
            continue;
        }

        if in_block_comment {
            current.push(ch);
            if ch == '*' && next == Some('/') {
                current.push('/');
                in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if !in_single && !in_double && ch == '-' && next == Some('-') {
            current.push(ch);
            current.push('-');
            in_line_comment = true;
            index += 2;
            continue;
        }

        if !in_single && !in_double && ch == '/' && next == Some('*') {
            current.push(ch);
            current.push('*');
            in_block_comment = true;
            index += 2;
            continue;
        }

        if !in_single && !in_double && ch == '$' {
            if let Some((tag, next_index)) = read_dollar_tag(&chars, index) {
                current.push_str(&chars[index..next_index].iter().collect::<String>());
                in_dollar = Some(tag);
                index = next_index;
                continue;
            }
        }

        if ch == '\'' && !in_double {
            if in_single && next == Some('\'') {
                current.push(ch);
                current.push('\'');
                index += 2;
                continue;
            }
            in_single = !in_single;
            current.push(ch);
            index += 1;
            continue;
        }

        if ch == '"' && !in_single {
            in_double = !in_double;
            current.push(ch);
            index += 1;
            continue;
        }

        if !in_single && !in_double && ch == ';' {
            current.push(ch);
            let statement = current.trim().trim_end_matches(';').trim().to_string();
            if !statement.is_empty() {
                statements.push(statement);
            }
            current.clear();
            index += 1;
            continue;
        }

        current.push(ch);
        index += 1;
    }

    let trailing = current.trim().trim_end_matches(';').trim().to_string();
    if !trailing.is_empty() {
        statements.push(trailing);
    }

    statements
}

async fn restore_rows_in_batches<A: DatabaseAdapter, F: FnMut(u64, u64)>(
    adapter: &A,
    target_schema: Option<&str>,
    target_table: &str,
    columns: &[String],
    rows: &[Vec<String>],
    on_progress: &mut F,
) -> DbResult<RestoreStats> {
    if target_table.trim().is_empty() {
        return Err(DbError::InvalidQuery(
            "Target table is required for tabular restore".to_string(),
        ));
    }

    let db_type = adapter.get_config().db_type;
    let total_batches = if rows.is_empty() {
        0
    } else {
        rows.len().div_ceil(CSV_BATCH_SIZE)
    } as u64;

    let mut batch_index = 0usize;
    let mut stats = RestoreStats {
        statements_total: total_batches,
        ..RestoreStats::default()
    };

    while batch_index < total_batches as usize {
        let start = batch_index * CSV_BATCH_SIZE;
        let end = ((batch_index + 1) * CSV_BATCH_SIZE).min(rows.len());
        let batch = &rows[start..end];
        let statement = build_insert_statement(db_type, target_schema, target_table, columns);

        let batch_result: DbResult<u64> = async {
            let mut inserted = 0u64;
            let mut row_index = 0usize;
            while row_index < batch.len() {
                inserted += adapter
                    .execute_batch_with_params(
                        &statement,
                        columns.len(),
                        vec![batch[row_index].clone()],
                    )
                    .await?;
                row_index += 1;
            }
            Ok(inserted)
        }
        .await;

        match batch_result {
            Ok(inserted) => {
                stats.statements_succeeded += 1;
                stats.rows_inserted += inserted;
            }
            Err(error) => {
                stats
                    .errors
                    .push(format!("batch {}: {}", batch_index + 1, error));
            }
        }

        batch_index += 1;
        on_progress(batch_index as u64, total_batches);
    }

    Ok(stats)
}

fn merge_stats(target: &mut RestoreStats, source: RestoreStats) {
    target.statements_total += source.statements_total;
    target.statements_succeeded += source.statements_succeeded;
    target.rows_inserted += source.rows_inserted;
    target.errors.extend(source.errors);
}

pub(crate) fn quote_identifier(name: &str, db_type: DatabaseType) -> String {
    match db_type {
        DatabaseType::MySQL => format!("`{}`", name.replace('`', "``")),
        DatabaseType::SqlServer => format!("[{}]", name.replace(']', "]]")),
        _ => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

fn qualified_table(db_type: DatabaseType, schema: Option<&str>, table: &str) -> String {
    schema
        .map(|schema_name| {
            format!(
                "{}.{}",
                quote_identifier(schema_name, db_type),
                quote_identifier(table, db_type)
            )
        })
        .unwrap_or_else(|| quote_identifier(table, db_type))
}

fn build_insert_statement(
    db_type: DatabaseType,
    target_schema: Option<&str>,
    target_table: &str,
    columns: &[String],
) -> String {
    let quoted_table = qualified_table(db_type, target_schema, target_table);
    let quoted_columns = columns
        .iter()
        .map(|column| quote_identifier(column, db_type))
        .collect::<Vec<_>>()
        .join(", ");

    let placeholders = match db_type {
        DatabaseType::PostgreSQL => (1..=columns.len())
            .map(|index| format!("${}", index))
            .collect::<Vec<_>>()
            .join(", "),
        DatabaseType::SqlServer => (1..=columns.len())
            .map(|index| format!("@P{}", index))
            .collect::<Vec<_>>()
            .join(", "),
        _ => std::iter::repeat_n("?", columns.len())
            .collect::<Vec<_>>()
            .join(", "),
    };

    format!(
        "INSERT INTO {} ({}) VALUES ({})",
        quoted_table, quoted_columns, placeholders
    )
}

#[cfg(test)]
mod tests {
    use super::split_sql_statements;
    use csv::ReaderBuilder;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn split_sql_statements_ignores_semicolons_in_strings() {
        let sql = "INSERT INTO t VALUES ('a; b');\nSELECT 1;\n";
        let statements = split_sql_statements(sql);

        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0], "INSERT INTO t VALUES ('a; b')");
        assert_eq!(statements[1], "SELECT 1");
    }

    #[test]
    fn split_sql_statements_ignores_semicolons_in_line_comment() {
        let sql = "SELECT 1 -- keep ; in comment\n;\nSELECT 2;\n";
        let statements = split_sql_statements(sql);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].starts_with("SELECT 1"));
        assert_eq!(statements[1], "SELECT 2");
    }

    #[test]
    fn split_sql_statements_ignores_semicolons_in_block_comment() {
        let sql = "/* ;;; */\nSELECT 1;\n/* x ; y */\nSELECT 2;\n";
        let statements = split_sql_statements(sql);

        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("SELECT 1"));
        assert!(statements[1].contains("SELECT 2"));
    }

    #[test]
    fn split_sql_statements_handles_same_line_semicolons() {
        let statements = split_sql_statements("SELECT 1; SELECT 2;");
        assert_eq!(statements, vec!["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn split_sql_statements_handles_dollar_quoted_function() {
        let sql = "CREATE FUNCTION demo() RETURNS void AS $$ BEGIN; END; $$ LANGUAGE plpgsql;";
        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 1);
    }

    #[test]
    fn split_sql_statements_handles_tagged_dollar() {
        let sql =
            "CREATE FUNCTION demo() RETURNS void AS $body$ BEGIN; PERFORM 1; END; $body$ LANGUAGE plpgsql;";
        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 1);
    }

    #[test]
    fn split_sql_statements_handles_nested_unmatched_dollar() {
        let sql = "SELECT $tag1$ one $tag2$ two $tag1$; SELECT 2;";
        let statements = split_sql_statements(sql);
        assert_eq!(
            statements,
            vec!["SELECT $tag1$ one $tag2$ two $tag1$", "SELECT 2"]
        );
    }

    #[test]
    fn csv_parser_handles_quoted_commas() {
        let file = NamedTempFile::new().expect("temp file");
        let path = file.path().to_path_buf();
        let content = "name,note\n\"Jane\",\"hello, world\"\n";
        fs::write(&path, content).expect("write csv");

        let mut reader = ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(true)
            .from_path(path)
            .expect("csv reader");

        let headers = reader
            .headers()
            .expect("headers")
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        let record = reader
            .records()
            .next()
            .expect("record")
            .expect("valid record");

        assert_eq!(headers, vec!["name".to_string(), "note".to_string()]);
        assert_eq!(record.get(0).unwrap_or_default(), "Jane");
        assert_eq!(record.get(1).unwrap_or_default(), "hello, world");
    }
}
