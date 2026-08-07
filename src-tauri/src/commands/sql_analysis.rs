//! SQL editability analysis.
//!
//! Parses a SELECT statement with sqlparser and decides whether its result rows
//! can be mapped back to a single base table (and therefore edited/deleted).
//! A result is editable only when the query is a plain single-table SELECT
//! (optionally with WHERE/ORDER BY/LIMIT) whose rows map 1:1 to the table.

use serde::{Deserialize, Serialize};
use sqlparser::ast::{
    Expr, GroupByExpr, ObjectNamePart, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
};
use sqlparser::dialect::{GenericDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;

/// Why a query result cannot be edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NonEditableReason {
    /// Statement is not a SELECT (INSERT/UPDATE/DELETE/DDL...).
    NotSelect,
    /// Query starts with WITH (CTE) — row identity cannot be trusted.
    Cte,
    /// UNION/INTERSECT/EXCEPT — rows come from multiple statements.
    SetOperation,
    /// GROUP BY / HAVING / DISTINCT / aggregate functions — rows are aggregated.
    Aggregation,
    /// More than one table source (JOIN or comma-separated).
    MultipleSources,
    /// No FROM clause at all.
    NoTable,
    /// The FROM source is a subquery/table function/parenthesized join.
    ComplexSource,
}

/// Result of analyzing whether a query's rows are editable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlEditability {
    pub editable: bool,
    /// Present only when editable — the single base table the query reads from.
    pub table_name: Option<String>,
    pub schema: Option<String>,
    pub reason: Option<NonEditableReason>,
}

fn dialect_for(db_type: &str) -> Box<dyn sqlparser::dialect::Dialect> {
    match db_type.to_ascii_lowercase().as_str() {
        "postgres" | "postgresql" | "duckdb" | "cockroachdb" | "gbase8c" | "kingbasees" | "yashandb"
        | "xugudb" | "timescaledb" | "redshift" | "yugabytedb" | "opengauss" | "highgo" | "uxdb"
        | "gaussdb" => Box::new(PostgreSqlDialect {}),
        "mysql" | "clickhouse" | "oceanbase" | "mariadb" | "gbase8a" => Box::new(MySqlDialect {}),
        "sqlserver" | "mssql" => Box::new(MsSqlDialect {}),
        _ => Box::new(GenericDialect {}),
    }
}

/// Analyze a SQL statement and report whether its result rows are editable.
///
/// Safe by construction: only plain single-table SELECTs map result rows back
/// to base rows. Aggregations, set operations, CTEs, and multi-source queries
/// are reported as non-editable with a machine-readable reason.
pub fn analyze_sql_editability(sql: &str, db_type: &str) -> SqlEditability {
    let dialect = dialect_for(db_type);
    let Ok(statements) = Parser::parse_sql(&*dialect, sql) else {
        return SqlEditability {
            editable: false,
            table_name: None,
            schema: None,
            reason: Some(NonEditableReason::ComplexSource),
        };
    };

    if statements.len() != 1 {
        return SqlEditability {
            editable: false,
            table_name: None,
            schema: None,
            reason: Some(NonEditableReason::NotSelect),
        };
    }

    let Some(query) = as_select_query(&statements[0]) else {
        return SqlEditability {
            editable: false,
            table_name: None,
            schema: None,
            reason: Some(NonEditableReason::NotSelect),
        };
    };

    // WITH (CTE) — the top-level FROM may reference a CTE instead of a table.
    if query.with.is_some() {
        return non_editable(NonEditableReason::Cte);
    }

    // UNION/INTERSECT/EXCEPT — rows come from multiple statements.
    if !matches!(query.body.as_ref(), SetExpr::Select(_)) {
        return non_editable(NonEditableReason::SetOperation);
    }

    let SetExpr::Select(select) = query.body.as_ref() else {
        return non_editable(NonEditableReason::SetOperation);
    };

    if select_is_aggregated(select) {
        return non_editable(NonEditableReason::Aggregation);
    }

    if select.from.is_empty() {
        return non_editable(NonEditableReason::NoTable);
    }

    // Exactly one FROM source, and it must be a plain table (no subquery,
    // no table function, no parenthesized join).
    if select.from.len() > 1 {
        return non_editable(NonEditableReason::MultipleSources);
    }

    let table_with_joins = &select.from[0];
    if !table_with_joins.joins.is_empty() {
        return non_editable(NonEditableReason::MultipleSources);
    }

    let TableFactor::Table { name, .. } = &table_with_joins.relation else {
        return non_editable(NonEditableReason::ComplexSource);
    };

    // Extract schema (second-to-last part) and table (last part).
    let parts: Vec<&String> = name
        .0
        .iter()
        .filter_map(ObjectNamePart::as_ident)
        .map(|ident| &ident.value)
        .collect();

    let Some(table_name) = parts.last().cloned().cloned() else {
        return non_editable(NonEditableReason::NoTable);
    };

    let schema = if parts.len() >= 2 {
        Some(parts[parts.len() - 2].clone())
    } else {
        None
    };

    SqlEditability {
        editable: true,
        table_name: Some(table_name),
        schema,
        reason: None,
    }
}

fn non_editable(reason: NonEditableReason) -> SqlEditability {
    SqlEditability {
        editable: false,
        table_name: None,
        schema: None,
        reason: Some(reason),
    }
}

fn as_select_query(statement: &Statement) -> Option<&Query> {
    match statement {
        Statement::Query(query) => Some(query),
        _ => None,
    }
}

/// True when the SELECT is aggregated: GROUP BY/HAVING/DISTINCT or an
/// aggregate function in the projection. Aggregated rows do not map 1:1 to
/// base-table rows.
fn select_is_aggregated(select: &Select) -> bool {
    if select.distinct.is_some() {
        return true;
    }
    if matches!(select.group_by, GroupByExpr::Expressions(ref exprs, _) if !exprs.is_empty()) {
        return true;
    }
    if select.having.is_some() {
        return true;
    }
    select.projection.iter().any(projection_has_aggregate)
}

fn projection_has_aggregate(item: &SelectItem) -> bool {
    match item {
        SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => {
            expr_has_aggregate(expr)
        }
        _ => false,
    }
}

fn expr_has_aggregate(expr: &Expr) -> bool {
    match expr {
        Expr::Function(function) => {
            if is_aggregate_function(&function.name.to_string()) {
                return true;
            }
            expr_function_has_aggregate_arg(&function.args)
                || function.filter.as_deref().is_some_and(expr_has_aggregate)
        }
        Expr::BinaryOp { left, right, .. } => expr_has_aggregate(left) || expr_has_aggregate(right),
        Expr::Nested(inner) | Expr::IsNull(inner) | Expr::IsNotNull(inner) => expr_has_aggregate(inner),
        Expr::UnaryOp { expr: inner, .. } => expr_has_aggregate(inner),
        Expr::Case { operand, conditions, else_result, .. } => {
            operand.as_deref().is_some_and(expr_has_aggregate)
                || conditions
                    .iter()
                    .any(|cond| expr_has_aggregate(&cond.condition) || expr_has_aggregate(&cond.result))
                || else_result.as_deref().is_some_and(expr_has_aggregate)
        }
        Expr::Subquery(query) | Expr::Exists { subquery: query, .. } => {
            let mut found = false;
            if let SetExpr::Select(select) = query.body.as_ref() {
                found = select.projection.iter().any(projection_has_aggregate);
            }
            found
        }
        _ => false,
    }
}

fn expr_function_has_aggregate_arg(args: &sqlparser::ast::FunctionArguments) -> bool {
    match args {
        sqlparser::ast::FunctionArguments::List(list) => list
            .args
            .iter()
            .any(|arg| match arg {
                sqlparser::ast::FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(e)) => {
                    expr_has_aggregate(e)
                }
                sqlparser::ast::FunctionArg::Named { arg, .. }
                | sqlparser::ast::FunctionArg::ExprNamed { arg, .. } => match arg {
                    sqlparser::ast::FunctionArgExpr::Expr(e) => expr_has_aggregate(e),
                    _ => false,
                },
                _ => false,
            }),
        sqlparser::ast::FunctionArguments::Subquery(query) => {
            let mut found = false;
            if let SetExpr::Select(select) = query.body.as_ref() {
                found = select.projection.iter().any(projection_has_aggregate);
            }
            found
        }
        sqlparser::ast::FunctionArguments::None => false,
    }
}

fn is_aggregate_function(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "STDDEV" | "STDDEV_POP" | "STDDEV_SAMP" | "VARIANCE"
            | "VAR_POP" | "VAR_SAMP" | "ARRAY_AGG" | "STRING_AGG" | "JSON_AGG" | "JSONB_AGG"
            | "BOOL_AND" | "BOOL_OR" | "EVERY"
    )
}

/// Tauri command: analyze whether a query's result rows are editable.
///
/// Returns the single base table (with optional schema) when the query is a
/// plain single-table SELECT; otherwise a machine-readable non-editable reason
/// the frontend can surface to the user.
#[tauri::command]
pub fn analyze_sql_editability_command(sql: String, database_type: Option<String>) -> SqlEditability {
    analyze_sql_editability(&sql, database_type.as_deref().unwrap_or("generic"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editable(sql: &str) -> bool {
        analyze_sql_editability(sql, "postgres").editable
    }

    fn reason(sql: &str) -> Option<NonEditableReason> {
        analyze_sql_editability(sql, "postgres").reason
    }

    #[test]
    fn plain_select_star_is_editable() {
        let result = analyze_sql_editability("SELECT * FROM apps", "postgres");
        assert!(result.editable);
        assert_eq!(result.table_name.as_deref(), Some("apps"));
        assert_eq!(result.schema, None);
    }

    #[test]
    fn select_with_qualifier_returns_schema() {
        let result = analyze_sql_editability("SELECT * FROM public.apps", "postgres");
        assert!(result.editable);
        assert_eq!(result.table_name.as_deref(), Some("apps"));
        assert_eq!(result.schema.as_deref(), Some("public"));
    }

    #[test]
    fn select_with_where_order_limit_is_editable() {
        assert!(editable("SELECT id, name FROM customers WHERE id > 10 ORDER BY name LIMIT 100"));
    }

    #[test]
    fn quoted_table_name_is_editable() {
        let result = analyze_sql_editability("SELECT * FROM \"My Table\"", "postgres");
        assert!(result.editable);
        assert_eq!(result.table_name.as_deref(), Some("My Table"));
    }

    #[test]
    fn join_is_not_editable() {
        assert!(!editable("SELECT a.*, b.* FROM a JOIN b ON a.id = b.id"));
        assert_eq!(reason("SELECT a.*, b.* FROM a JOIN b ON a.id = b.id"), Some(NonEditableReason::MultipleSources));
    }

    #[test]
    fn comma_separated_sources_is_not_editable() {
        assert_eq!(reason("SELECT * FROM a, b"), Some(NonEditableReason::MultipleSources));
    }

    #[test]
    fn count_aggregation_is_not_editable() {
        assert_eq!(reason("SELECT COUNT(*) FROM apps"), Some(NonEditableReason::Aggregation));
    }

    #[test]
    fn group_by_is_not_editable() {
        assert_eq!(reason("SELECT name, COUNT(*) FROM customers GROUP BY name"), Some(NonEditableReason::Aggregation));
    }

    #[test]
    fn distinct_is_not_editable() {
        assert_eq!(reason("SELECT DISTINCT name FROM customers"), Some(NonEditableReason::Aggregation));
    }

    #[test]
    fn union_is_not_editable() {
        assert_eq!(reason("SELECT * FROM a UNION SELECT * FROM b"), Some(NonEditableReason::SetOperation));
    }

    #[test]
    fn cte_is_not_editable() {
        assert_eq!(
            reason("WITH t AS (SELECT * FROM apps) SELECT * FROM t"),
            Some(NonEditableReason::Cte)
        );
    }

    #[test]
    fn subquery_source_is_not_editable() {
        assert_eq!(
            reason("SELECT * FROM (SELECT * FROM apps) t"),
            Some(NonEditableReason::ComplexSource)
        );
    }

    #[test]
    fn non_select_statement_is_not_editable() {
        assert_eq!(reason("UPDATE apps SET name = 'x'"), Some(NonEditableReason::NotSelect));
        assert_eq!(reason("DELETE FROM apps"), Some(NonEditableReason::NotSelect));
        assert_eq!(reason("INSERT INTO apps (name) VALUES ('x')"), Some(NonEditableReason::NotSelect));
    }

    #[test]
    fn multiple_statements_is_not_editable() {
        assert_eq!(
            reason("SELECT * FROM apps; SELECT * FROM orders"),
            Some(NonEditableReason::NotSelect)
        );
    }

    #[test]
    fn mysql_backtick_table_is_editable() {
        let result = analyze_sql_editability("SELECT * FROM `customers`", "mysql");
        assert!(result.editable);
        assert_eq!(result.table_name.as_deref(), Some("customers"));
    }

    #[test]
    fn no_from_is_not_editable() {
        assert_eq!(reason("SELECT 1"), Some(NonEditableReason::NoTable));
    }
}
