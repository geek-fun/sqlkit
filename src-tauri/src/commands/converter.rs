//! Data conversion utilities for database responses.
//!
//! This module provides utilities to convert database-specific types
//! to JSON-friendly formats for frontend consumption.

use crate::database::types::{QueryResult, QueryRow, QueryValue, TableInfo};
use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};

/// Convert a QueryValue to a JSON Value.
///
/// This handles special cases like binary data (converts to base64)
/// and ensures proper JSON representation of database values.
pub fn convert_query_value_to_json(value: &QueryValue) -> Value {
    match value {
        QueryValue::Null => Value::Null,
        QueryValue::Bool(b) => json!(b),
        QueryValue::Int(i) => json!(i),
        QueryValue::Float(f) => {
            // Handle special float values
            if f.is_nan() {
                json!("NaN")
            } else if f.is_infinite() {
                if f.is_sign_positive() {
                    json!("Infinity")
                } else {
                    json!("-Infinity")
                }
            } else {
                json!(f)
            }
        }
        QueryValue::String(s) => json!(s),
        QueryValue::Bytes(b) => {
            // Convert binary data to base64 string
            json!(general_purpose::STANDARD.encode(b))
        }
        QueryValue::DateTime(dt) => json!(dt),
    }
}

/// Convert a QueryRow to a JSON object.
pub fn convert_query_row_to_json(row: &QueryRow) -> Value {
    let obj: serde_json::Map<String, Value> = row
        .iter()
        .map(|(k, v)| (k.clone(), convert_query_value_to_json(v)))
        .collect();
    Value::Object(obj)
}

/// Convert a QueryResult to a JSON-friendly structure.
///
/// This ensures all special values are properly handled before
/// being sent to the frontend.
pub fn convert_query_result_to_json(result: &QueryResult) -> Value {
    json!({
        "columns": result.columns,
        "rows": result.rows.iter().map(convert_query_row_to_json).collect::<Vec<_>>(),
        "rows_affected": result.rows_affected,
        "execution_time_ms": result.execution_time_ms,
    })
}

/// Convert a vector of TableInfo to JSON array.
///
/// This is a pass-through for now but provides a place for future
/// transformations if needed.
pub fn convert_table_list_to_json(tables: &[TableInfo]) -> Value {
    json!(tables)
}

/// Convert database name list to JSON array.
pub fn convert_database_list_to_json(databases: &[String]) -> Value {
    json!(databases)
}

/// Convert schema name list to JSON array.
pub fn convert_schema_list_to_json(schemas: &[String]) -> Value {
    json!(schemas)
}

/// Parse a JSON value into a QueryValue based on context.
///
/// Used when converting frontend data back to database values.
pub fn json_to_query_value(value: &Value) -> Option<QueryValue> {
    match value {
        Value::Null => Some(QueryValue::Null),
        Value::Bool(b) => Some(QueryValue::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(QueryValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                Some(QueryValue::Float(f))
            } else {
                None
            }
        }
        Value::String(s) => {
            // Try to decode base64 as binary data if it looks like base64
            // Otherwise treat as string
            if let Ok(bytes) = general_purpose::STANDARD.decode(s) {
                // Check if it's likely binary data (contains non-printable chars)
                if bytes.iter().any(|&b: &u8| b < 32 && b != b'\n' && b != b'\r' && b != b'\t') {
                    Some(QueryValue::Bytes(bytes))
                } else {
                    Some(QueryValue::String(s.clone()))
                }
            } else {
                Some(QueryValue::String(s.clone()))
            }
        }
        Value::Array(_) | Value::Object(_) => {
            // Complex types are serialized as JSON strings
            Some(QueryValue::String(value.to_string()))
        }
    }
}

/// Convert a JSON object to a QueryRow.
pub fn json_to_query_row(obj: &serde_json::Map<String, Value>) -> QueryRow {
    obj.iter()
        .filter_map(|(k, v)| json_to_query_value(v).map(|qv| (k.clone(), qv)))
        .collect()
}

/// Parse a string value into the appropriate QueryValue type.
///
/// Attempts to detect booleans and numbers, defaults to String.
pub fn parse_string_to_query_value(value: &str) -> QueryValue {
    if value.eq_ignore_ascii_case("true") {
        QueryValue::Bool(true)
    } else if value.eq_ignore_ascii_case("false") {
        QueryValue::Bool(false)
    } else if value.eq_ignore_ascii_case("null") {
        QueryValue::Null
    } else if let Ok(i) = value.parse::<i64>() {
        QueryValue::Int(i)
    } else if let Ok(f) = value.parse::<f64>() {
        QueryValue::Float(f)
    } else {
        QueryValue::String(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_query_value_null() {
        let value = QueryValue::Null;
        let json = convert_query_value_to_json(&value);
        assert_eq!(json, Value::Null);
    }

    #[test]
    fn test_convert_query_value_bool() {
        let value = QueryValue::Bool(true);
        let json = convert_query_value_to_json(&value);
        assert_eq!(json, json!(true));
    }

    #[test]
    fn test_convert_query_value_int() {
        let value = QueryValue::Int(42);
        let json = convert_query_value_to_json(&value);
        assert_eq!(json, json!(42));
    }

    #[test]
    fn test_convert_query_value_float() {
        let value = QueryValue::Float(3.14);
        let json = convert_query_value_to_json(&value);
        assert_eq!(json, json!(3.14));
    }

    #[test]
    fn test_convert_query_value_string() {
        let value = QueryValue::String("hello".to_string());
        let json = convert_query_value_to_json(&value);
        assert_eq!(json, json!("hello"));
    }

    #[test]
    fn test_convert_query_value_bytes() {
        let value = QueryValue::Bytes(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello" in bytes
        let json = convert_query_value_to_json(&value);
        assert_eq!(json, json!("SGVsbG8="));
    }

    #[test]
    fn test_convert_query_value_special_floats() {
        let nan = QueryValue::Float(f64::NAN);
        let inf = QueryValue::Float(f64::INFINITY);
        let neg_inf = QueryValue::Float(f64::NEG_INFINITY);

        assert_eq!(convert_query_value_to_json(&nan), json!("NaN"));
        assert_eq!(convert_query_value_to_json(&inf), json!("Infinity"));
        assert_eq!(convert_query_value_to_json(&neg_inf), json!("-Infinity"));
    }

    #[test]
    fn test_parse_string_to_query_value() {
        assert!(matches!(parse_string_to_query_value("true"), QueryValue::Bool(true)));
        assert!(matches!(parse_string_to_query_value("false"), QueryValue::Bool(false)));
        assert!(matches!(parse_string_to_query_value("null"), QueryValue::Null));
        assert!(matches!(parse_string_to_query_value("42"), QueryValue::Int(42)));
        assert!(matches!(parse_string_to_query_value("3.14"), QueryValue::Float(_)));
        assert!(matches!(parse_string_to_query_value("hello"), QueryValue::String(_)));
    }

    #[test]
    fn test_convert_query_row() {
        let mut row = QueryRow::new();
        row.insert("id".to_string(), QueryValue::Int(1));
        row.insert("name".to_string(), QueryValue::String("Alice".to_string()));
        row.insert("active".to_string(), QueryValue::Bool(true));

        let json = convert_query_row_to_json(&row);
        assert!(json.is_object());
        
        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), &json!(1));
        assert_eq!(obj.get("name").unwrap(), &json!("Alice"));
        assert_eq!(obj.get("active").unwrap(), &json!(true));
    }
}
