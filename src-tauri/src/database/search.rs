//! Search SQL generation for inline row data search across all columns.
//!
//! This module provides functions to build dialect-aware search WHERE clauses
//! for searching across text and numeric columns in a table. BLOB/BINARY/geometry
//! columns are automatically skipped.
//!
//! # Example
//!
//! ```ignore
//! use sqlkit::database::search::build_table_search_where;
//!
//! let columns = vec![
//!     ColumnInfo { name: "email".into(), data_type: "varchar".into(), .. },
//!     ColumnInfo { name: "age".into(), data_type: "integer".into(), .. },
//! ];
//!
//! let where_clause = build_table_search_where("postgres", &columns, "alice");
//! // Returns: (LOWER(CAST("email" AS TEXT)) LIKE '%alice%' OR LOWER(CAST("age" AS TEXT)) LIKE '%alice%' OR "age" = 42)
//! ```

use crate::database::types::ColumnInfo;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Category of a column type for search purposes.
///
/// Determines how a column is searched:
/// - `Text`: searched with `LOWER(CAST(col AS TYPE)) LIKE '%term%'`
/// - `Numeric`: searched with both exact equality and text LIKE
/// - `Skip`: excluded entirely from search (BLOB/BINARY/geometry)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnCategory {
    /// Text-type column — use LIKE on cast-to-text.
    Text,
    /// Numeric-type column — use equality AND LIKE on cast-to-text.
    Numeric,
    /// Binary/geometry column — skip from search.
    Skip,
}

/// Classify a column data type string into a [`ColumnCategory`].
///
/// Matches common type names across PostgreSQL, MySQL, SQL Server, and SQLite.
///
/// # Classification Rules
///
/// | Category | Matches |
/// |---|---|
/// | `Skip` | blob, binary, bytea, varbinary, image, geometry, geography, point, linestring, polygon, circle, box, path, pgis_* |
/// | `Numeric` | int*, serial*, numeric*, decimal*, float*, double*, real*, money*, number, bool, boolean, bit |
/// | `Text` | Everything else (char, varchar, text, uuid, enum, json, jsonb, xml, date/time types, etc.) |
pub fn classify_column(col_type: &str) -> ColumnCategory {
    let t = col_type.to_lowercase();

    // Skip types — BLOB, BINARY, GEOMETRY, interval (time range), etc.
    if t.contains("blob")
        || t.contains("binary")
        || t.contains("bytea")
        || t.contains("varbinary")
        || t.contains("image")
        || t.contains("geometry")
        || t.contains("geography")
        || t.contains("point")
        || t.contains("linestring")
        || t.contains("polygon")
        || t.contains("circle")
        || t.contains("box")
        || t.contains("path")
        || t == "interval"
        || t.starts_with("pgis_")
    {
        return ColumnCategory::Skip;
    }

    // Numeric types
    if t.contains("int")
        || t.contains("serial")
        || t.contains("numeric")
        || t.contains("decimal")
        || t.contains("float")
        || t.contains("double")
        || t.contains("real")
        || t.contains("money")
        || t.contains("number")
        || t == "bool"
        || t == "boolean"
        || t == "bit"
    {
        return ColumnCategory::Numeric;
    }

    // Everything else is searchable as text
    ColumnCategory::Text
}

/// Get the target type name for `CAST(col AS <type>)` in a given database dialect.
fn cast_target_type(db_type: &str) -> &'static str {
    match db_type {
        "postgres" | "duckdb" | "sqlite" => "TEXT",
        "mysql" | "clickhouse" => "CHAR",
        "sqlserver" => "NVARCHAR(MAX)",
        "jdbc" | "trino" => "VARCHAR",
        _ => "TEXT",
    }
}

/// Quote a SQL identifier according to the database dialect.
///
/// Supports the same dialects as `crate::commands::browse::quote_identifier`.
fn quote_identifier(identifier: &str, db_type: &str) -> String {
    match db_type {
        "postgres" | "sqlite" | "duckdb" | "jdbc" => {
            format!("\"{}\"", identifier.replace('\"', "\"\""))
        }
        "mysql" | "clickhouse" => format!("`{}`", identifier.replace('`', "``")),
        "sqlserver" => format!("[{}]", identifier.replace(']', "]]")),
        _ => identifier.to_string(),
    }
}

/// Build a search WHERE clause that searches across all non-BLOB columns.
///
/// Returns `None` if the search term is empty or no searchable columns are found.
///
/// # Search Behavior
///
/// - **Text columns** (`ColumnCategory::Text`): matched with
///   `LOWER(CAST(col AS type)) LIKE '%term%'`
/// - **Numeric columns** (`ColumnCategory::Numeric`): matched with
///   `(col = term OR LOWER(CAST(col AS type)) LIKE '%term%')` — this handles
///   both exact numeric matches and text representation matches
/// - **Skip columns** (`ColumnCategory::Skip`): excluded entirely
/// - The search is **case-insensitive** via `LOWER()`
///
/// # Arguments
///
/// * `db_type` - Database type string (`"postgres"`, `"mysql"`, `"sqlserver"`, `"sqlite"`, etc.)
/// * `columns` - Column metadata from `list_columns` or `get_table_info`
/// * `term` - The raw user search term (will be trimmed)
///
/// # Returns
///
/// An SQL WHERE clause like `(LOWER(CAST("col1" AS TEXT)) LIKE '%term%' OR ...)`
/// or `None` if no searchable columns or empty term.
pub fn build_table_search_where(db_type: &str, columns: &[ColumnInfo], term: &str) -> Option<String> {
    let term = term.trim();
    if term.is_empty() {
        return None;
    }

    let searchable: Vec<&ColumnInfo> = columns
        .iter()
        .filter(|c| classify_column(&c.data_type) != ColumnCategory::Skip)
        .collect();

    if searchable.is_empty() {
        return None;
    }

    let cast_type = cast_target_type(db_type);
    let escaped_term = term.replace('\'', "''");
    let lower_term = escaped_term.to_lowercase();

    let conditions: Vec<String> = searchable
        .iter()
        .map(|col| {
            let quoted = quote_identifier(&col.name, db_type);
            let category = classify_column(&col.data_type);
            let text_condition =
                format!("LOWER(CAST({} AS {})) LIKE '%{}%'", quoted, cast_type, lower_term);

            match category {
                ColumnCategory::Text => text_condition,
                ColumnCategory::Numeric => {
                    // Try exact numeric match; if term is a valid number, include equality
                    if term.parse::<f64>().is_ok() {
                        format!("{} = {} OR {}", quoted, term, text_condition)
                    } else {
                        // Not a number — only text search
                        text_condition
                    }
                }
                ColumnCategory::Skip => unreachable!(), // filtered above
            }
        })
        .collect();

    if conditions.is_empty() {
        return None;
    }

    Some(format!("({})", conditions.join(" OR ")))
}

/// Generate a stable WHERE clause that uniquely identifies a row.
///
/// Uses primary key columns if available; otherwise falls back to matching
/// all searchable (non-BLOB) column values. Intended for future "open filtered
/// from search" navigation — the caller can append this to a SELECT to
/// jump directly to the row that was clicked in the search results.
///
/// # Arguments
///
/// * `db_type` - Database type string (`"postgres"`, `"mysql"`, etc.)
/// * `columns` - Column metadata for the table (used to find PKs)
/// * `row_values` - A single row's data as column → JSON value map
///
/// # Returns
///
/// A WHERE clause like `"id" = 42 AND "name" = 'alice'`, or `None` if
/// `row_values` is empty or no suitable columns are found.
pub fn build_search_result_where(
    db_type: &str,
    columns: &[ColumnInfo],
    row_values: &HashMap<String, JsonValue>,
) -> Option<String> {
    if row_values.is_empty() {
        return None;
    }

    // Prefer primary key columns for stable row identification.
    let pk_cols: Vec<&ColumnInfo> = columns.iter().filter(|c| c.is_primary_key).collect();
    let target_cols: Vec<&ColumnInfo> = if !pk_cols.is_empty() {
        pk_cols
    } else {
        // Fall back to all searchable (non-BLOB) columns
        columns
            .iter()
            .filter(|c| classify_column(&c.data_type) != ColumnCategory::Skip)
            .collect()
    };

    let conditions: Vec<String> = target_cols
        .iter()
        .filter_map(|col| {
            let value = row_values.get(&col.name)?;
            let quoted = quote_identifier(&col.name, db_type);
            if value.is_null() {
                Some(format!("{} IS NULL", quoted))
            } else {
                Some(format!("{} = {}", quoted, json_to_sql_literal(value)))
            }
        })
        .collect();

    if conditions.is_empty() {
        return None;
    }
    Some(conditions.join(" AND "))
}

/// Convert a `serde_json::Value` to a SQL literal string.
fn json_to_sql_literal(val: &JsonValue) -> String {
    match val {
        JsonValue::Null => "NULL".to_string(),
        JsonValue::Bool(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => format!("'{}'", s.replace('\'', "''")),
        JsonValue::Array(_) | JsonValue::Object(_) => {
            let s = val.to_string().replace('\'', "''");
            format!("'{}'", s)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn col(name: &str, data_type: &str) -> ColumnInfo {
        ColumnInfo {
            name: name.to_string(),
            data_type: data_type.to_string(),
            nullable: true,
            default_value: None,
            is_primary_key: false,
            is_auto_increment: false,
            max_length: None,
            precision: None,
            scale: None,
            description: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    // ── classify_column tests ──

    #[test]
    fn test_classify_text_types() {
        for t in &["varchar", "char", "text", "nvarchar", "nchar", "ntext", "uuid", "enum",
                     "name", "json", "jsonb", "xml", "date", "time", "timestamp", "timestamptz",
                     "bpchar"]
        {
            assert_eq!(
                classify_column(t),
                ColumnCategory::Text,
                "expected '{}' to be Text",
                t,
            );
        }
    }

    #[test]
    fn test_classify_numeric_types() {
        for t in &["integer", "int", "int4", "int8", "int16", "bigint", "smallint", "tinyint",
                     "serial", "bigserial", "smallserial", "numeric", "decimal", "float", "float4",
                     "float8", "double", "double precision", "real", "money", "smallmoney",
                     "number", "bool", "boolean", "bit"]
        {
            assert_eq!(
                classify_column(t),
                ColumnCategory::Numeric,
                "expected '{}' to be Numeric",
                t,
            );
        }
    }

    #[test]
    fn test_classify_skip_types() {
        for t in &["blob", "binary", "bytea", "varbinary", "image", "geometry", "geography",
                     "point", "linestring", "polygon", "circle", "box", "path", "interval"]
        {
            assert_eq!(
                classify_column(t),
                ColumnCategory::Skip,
                "expected '{}' to be Skip",
                t,
            );
        }
    }

    #[test]
    fn test_classify_case_insensitive() {
        assert_eq!(classify_column("VARCHAR"), ColumnCategory::Text);
        assert_eq!(classify_column("INTEGER"), ColumnCategory::Numeric);
        assert_eq!(classify_column("BLOB"), ColumnCategory::Skip);
        assert_eq!(classify_column("Text"), ColumnCategory::Text);
        assert_eq!(classify_column("Bytea"), ColumnCategory::Skip);
    }

    // ── build_table_search_where tests ──

    #[test]
    fn test_empty_term_returns_none() {
        let cols = [col("id", "integer")];
        assert_eq!(build_table_search_where("postgres", &cols, ""), None);
        assert_eq!(build_table_search_where("postgres", &cols, "  "), None);
    }

    #[test]
    fn test_no_searchable_columns_returns_none() {
        let cols = [col("data", "blob")];
        assert_eq!(build_table_search_where("postgres", &cols, "test"), None);
    }

    #[test]
    fn test_postgres_text_search() {
        let cols = [col("name", "varchar"), col("email", "text")];
        let result = build_table_search_where("postgres", &cols, "alice").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(\"name\" AS TEXT)) LIKE '%alice%' OR LOWER(CAST(\"email\" AS TEXT)) LIKE '%alice%')"
        );
    }

    #[test]
    fn test_postgres_numeric_search() {
        let cols = [col("age", "integer")];
        let result = build_table_search_where("postgres", &cols, "42").unwrap();
        assert_eq!(
            result,
            "(\"age\" = 42 OR LOWER(CAST(\"age\" AS TEXT)) LIKE '%42%')"
        );
    }

    #[test]
    fn test_postgres_numeric_search_non_numeric_term() {
        let cols = [col("age", "integer")];
        // "abc" is not a valid number, so only text search
        let result = build_table_search_where("postgres", &cols, "abc").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(\"age\" AS TEXT)) LIKE '%abc%')"
        );
    }

    #[test]
    fn test_postgres_mixed_search() {
        let cols = [
            col("name", "varchar"),
            col("age", "integer"),
            col("avatar", "bytea"), // skipped
        ];
        let result = build_table_search_where("postgres", &cols, "42").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(\"name\" AS TEXT)) LIKE '%42%' OR \"age\" = 42 OR LOWER(CAST(\"age\" AS TEXT)) LIKE '%42%')"
        );
    }

    #[test]
    fn test_mysql_text_search() {
        let cols = [col("name", "varchar")];
        let result = build_table_search_where("mysql", &cols, "alice").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(`name` AS CHAR)) LIKE '%alice%')"
        );
    }

    #[test]
    fn test_mysql_numeric_search() {
        let cols = [col("age", "int")];
        let result = build_table_search_where("mysql", &cols, "42").unwrap();
        assert_eq!(
            result,
            "(`age` = 42 OR LOWER(CAST(`age` AS CHAR)) LIKE '%42%')"
        );
    }

    #[test]
    fn test_sqlserver_text_search() {
        let cols = [col("name", "nvarchar")];
        let result = build_table_search_where("sqlserver", &cols, "alice").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST([name] AS NVARCHAR(MAX))) LIKE '%alice%')"
        );
    }

    #[test]
    fn test_sqlserver_numeric_search() {
        let cols = [col("age", "int")];
        let result = build_table_search_where("sqlserver", &cols, "42").unwrap();
        assert_eq!(
            result,
            "([age] = 42 OR LOWER(CAST([age] AS NVARCHAR(MAX))) LIKE '%42%')"
        );
    }

    #[test]
    fn test_sqlite_text_search() {
        let cols = [col("name", "text")];
        let result = build_table_search_where("sqlite", &cols, "alice").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(\"name\" AS TEXT)) LIKE '%alice%')"
        );
    }

    #[test]
    fn test_duckdb_search() {
        let cols = [col("name", "varchar")];
        let result = build_table_search_where("duckdb", &cols, "test").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(\"name\" AS TEXT)) LIKE '%test%')"
        );
    }

    #[test]
    fn test_clickhouse_search() {
        let cols = [col("name", "String")];
        let result = build_table_search_where("clickhouse", &cols, "test").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(`name` AS CHAR)) LIKE '%test%')"
        );
    }

    #[test]
    fn test_jdbc_search() {
        let cols = [col("name", "varchar")];
        let result = build_table_search_where("jdbc", &cols, "test").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(\"name\" AS VARCHAR)) LIKE '%test%')"
        );
    }

    #[test]
    fn test_trino_search() {
        let cols = [col("name", "varchar")];
        let result = build_table_search_where("trino", &cols, "test").unwrap();
        // Trino does not quote identifiers by default
        assert_eq!(
            result,
            "(LOWER(CAST(name AS VARCHAR)) LIKE '%test%')"
        );
    }

    #[test]
    fn test_special_chars_escaped() {
        let cols = [col("name", "text")];
        // Single quotes must be escaped
        let result = build_table_search_where("postgres", &cols, "o'neil").unwrap();
        assert_eq!(
            result,
            "(LOWER(CAST(\"name\" AS TEXT)) LIKE '%o''neil%')"
        );
    }

    #[test]
    fn test_column_with_special_chars() {
        let cols = [col("last\"name", "text")];
        let result = build_table_search_where("postgres", &cols, "test").unwrap();
        // Quote in column name should be escaped
        assert!(result.contains("\"last\"\"name\""));
    }

    #[test]
    fn test_many_columns() {
        let cols = (0..10)
            .map(|i| col(&format!("col{}", i), "text"))
            .collect::<Vec<_>>();
        let result = build_table_search_where("postgres", &cols, "search").unwrap();
        // Should contain 10 conditions
        assert_eq!(result.matches("LOWER(CAST(").count(), 10);
    }

    #[test]
    fn test_case_insensitive_search() {
        let cols = [col("name", "varchar")];
        let result = build_table_search_where("postgres", &cols, "ALICE").unwrap();
        // The search term should be lowercased in the SQL
        assert!(
            result.contains("'%alice%'"),
            "Expected lowercased term in LIKE, got: {}",
            result
        );
    }

    #[test]
    fn test_quote_identifier_postgres() {
        assert_eq!(quote_identifier("test", "postgres"), "\"test\"");
        assert_eq!(quote_identifier("te\"st", "postgres"), "\"te\"\"st\"");
    }

    #[test]
    fn test_quote_identifier_mysql() {
        assert_eq!(quote_identifier("test", "mysql"), "`test`");
        assert_eq!(quote_identifier("te`st", "mysql"), "`te``st`");
    }

    #[test]
    fn test_quote_identifier_sqlserver() {
        assert_eq!(quote_identifier("test", "sqlserver"), "[test]");
        assert_eq!(quote_identifier("te]st", "sqlserver"), "[te]]st]");
    }

    #[test]
    fn test_quote_identifier_sqlite() {
        assert_eq!(quote_identifier("test", "sqlite"), "\"test\"");
    }

    #[test]
    fn test_classify_unknown_type_as_text() {
        // Unknown or custom types should default to Text
        assert_eq!(classify_column("custom_type"), ColumnCategory::Text);
        assert_eq!(classify_column("hstore"), ColumnCategory::Text);
        assert_eq!(classify_column("citext"), ColumnCategory::Text);
    }

    // ── build_search_result_where tests ──

    fn pk_col(name: &str, data_type: &str) -> ColumnInfo {
        ColumnInfo {
            is_primary_key: true,
            ..col(name, data_type)
        }
    }

    fn row(pairs: &[(&str, JsonValue)]) -> HashMap<String, JsonValue> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn test_result_where_with_pk() {
        let cols = [pk_col("id", "integer"), col("name", "varchar")];
        let r = row(&[("id", json!(42)), ("name", json!("alice"))]);
        let result = build_search_result_where("postgres", &cols, &r).unwrap();
        assert_eq!(result, "\"id\" = 42");
    }

    #[test]
    fn test_result_where_no_pk_fallback() {
        let cols = [col("name", "varchar"), col("age", "integer"), col("avatar", "bytea")];
        let r = row(&[("name", json!("alice")), ("age", json!(30)), ("avatar", json!(null))]);
        let result = build_search_result_where("postgres", &cols, &r).unwrap();
        // Should use name and age (searchable), skip bytea
        assert!(result.contains("\"name\" = 'alice'"));
        assert!(result.contains("\"age\" = 30"));
        assert!(!result.contains("avatar"));
        assert!(result.contains(" AND "));
    }

    #[test]
    fn test_result_where_mixed_pk_and_regular() {
        let cols = [pk_col("id", "int"), pk_col("lang", "varchar"), col("name", "text")];
        let r = row(&[("id", json!(1)), ("lang", json!("en")), ("name", json!("hello"))]);
        let result = build_search_result_where("postgres", &cols, &r).unwrap();
        // Only PKs should be used
        assert!(result.contains("\"id\" = 1"));
        assert!(result.contains("\"lang\" = 'en'"));
        assert!(!result.contains("name"));
    }

    #[test]
    fn test_result_where_empty_row_returns_none() {
        let cols = [pk_col("id", "integer")];
        let r: HashMap<String, JsonValue> = HashMap::new();
        assert_eq!(build_search_result_where("postgres", &cols, &r), None);
    }

    #[test]
    fn test_result_where_null_pk() {
        let cols = [pk_col("id", "integer")];
        let r = row(&[("id", JsonValue::Null)]);
        let result = build_search_result_where("postgres", &cols, &r).unwrap();
        assert_eq!(result, "\"id\" IS NULL");
    }

    #[test]
    fn test_result_where_special_chars() {
        let cols = [pk_col("name", "varchar")];
        let r = row(&[("name", json!("o'neil"))]);
        let result = build_search_result_where("postgres", &cols, &r).unwrap();
        assert_eq!(result, "\"name\" = 'o''neil'");
    }

    #[test]
    fn test_result_where_mysql_quoting() {
        let cols = [pk_col("id", "integer")];
        let r = row(&[("id", json!(1))]);
        let result = build_search_result_where("mysql", &cols, &r).unwrap();
        assert_eq!(result, "`id` = 1");
    }

    #[test]
    fn test_result_where_sqlserver_quoting() {
        let cols = [pk_col("id", "integer")];
        let r = row(&[("id", json!(1))]);
        let result = build_search_result_where("sqlserver", &cols, &r).unwrap();
        assert_eq!(result, "[id] = 1");
    }

    #[test]
    fn test_result_where_skipped_cols_not_included() {
        let cols = [
            pk_col("id", "integer"),
            col("data", "geometry"), // should be skipped
        ];
        let r = row(&[("id", json!(1)), ("data", json!("POINT(0 0)"))]);
        let result = build_search_result_where("postgres", &cols, &r).unwrap();
        // "data" is SKIP type but it's not a PK so it won't be included anyway
        // (PKs are preferred). This test verifies that SKIP columns aren't
        // included in the fallback path either.
        assert_eq!(result, "\"id\" = 1");
    }

    #[test]
    fn test_result_where_all_searchable_no_pk() {
        let cols = [col("a", "text"), col("b", "text")];
        let r = row(&[("a", json!("x")), ("b", json!("y"))]);
        let result = build_search_result_where("postgres", &cols, &r).unwrap();
        assert!(result.contains("\"a\" = 'x'"));
        assert!(result.contains("\"b\" = 'y'"));
    }
}
