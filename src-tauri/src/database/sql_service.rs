//! SQL construction service — the single source of truth for building queries.
//!
//! All SQL wrapping, validation, filtering, sorting, pagination, and schema
//! injection goes through this module. The frontend never constructs SQL strings.
//!
//! # Design
//!
//! Every function returns a `SqlBuildResult` with structured success/failure
//! so the caller can surface clear error messages instead of cryptic database errors.

use serde::{Deserialize, Serialize};
use sqlparser::ast::{
    Expr, Ident, ObjectNamePart, Query, SelectItem, SelectItemQualifiedWildcardKind, SetExpr,
    Statement, TableFactor, TableWithJoins, Value,
};
use sqlparser::dialect::{GenericDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;

// ── Public types ──────────────────────────────────────────────────────────

/// Outcome of a SQL build operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlBuildResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl SqlBuildResult {
    fn ok(sql: String) -> Self {
        Self { ok: true, sql: Some(sql), reason: None }
    }

    fn err(reason: impl Into<String>) -> Self {
        Self { ok: false, sql: None, reason: Some(reason.into()) }
    }
}

/// A single sort rule (column + direction).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortRule {
    pub column: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    ASC,
    DESC,
}

/// A single filter rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterRule {
    pub column: String,
    pub operator: FilterOperator,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value2: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Eq,
    Neq,
    Like,
    Gt,
    Lt,
    Gte,
    Lte,
    Between,
}

/// Options for wrapping a user's SQL with sort/filter/pagination.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapQueryOptions {
    /// The raw SQL from the editor.
    pub original_sql: String,
    /// Database type string ("postgres", "mysql", "sqlserver", "sqlite", etc.)
    /// Automatically derived from the active connection when sent from the frontend.
    #[serde(default)]
    pub database_type: String,
    /// Optional schema context for qualifying unqualified table references.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_context: Option<String>,
    /// Sort rules.
    #[serde(default)]
    pub sort: Vec<SortRule>,
    /// Filter rules.
    #[serde(default)]
    pub filters: Vec<FilterRule>,
    /// Optional limit (None = no limit change).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// Optional offset (None = no offset change).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

// ── Public API ─────────────────────────────────────────────────────────────

/// Build a wrapped SQL query with sort, filter, and pagination applied.
///
/// This is the main entry point. It:
/// 1. Strips trailing semicolons and comments from the original SQL
/// 2. Parses it with sqlparser to validate it's a single SELECT/WITH statement
/// 3. Injects schema qualification if a schema_context is provided
/// 4. Wraps it in a derived table if sort/filter/pagination are needed
/// 5. Applies WHERE, ORDER BY, and LIMIT/OFFSET clauses
pub fn wrap_query(options: WrapQueryOptions) -> SqlBuildResult {
    // ── Step 1: Clean the SQL ──
    let cleaned = strip_trailing(&options.original_sql);
    if cleaned.is_empty() {
        return SqlBuildResult::err("empty query");
    }

    // ── Step 2: Parse & extract a single statement ──
    let dialect: Box<dyn sqlparser::dialect::Dialect> = match options.database_type.as_str() {
        "postgres" | "duckdb" | "cockroachdb" => Box::new(PostgreSqlDialect {}),
        "mysql" | "clickhouse" => Box::new(MySqlDialect {}),
        "sqlserver" => Box::new(MsSqlDialect {}),
        _ => Box::new(GenericDialect {}),
    };

    let stmts = match Parser::parse_sql(&*dialect, &cleaned) {
        Ok(s) => s,
        Err(e) => return SqlBuildResult::err(format!("parse error: {}", e)),
    };

    if stmts.is_empty() {
        return SqlBuildResult::err("empty query after parsing");
    }
    if stmts.len() > 1 {
        return SqlBuildResult::err("multiple statements are not supported for sort/filter");
    }

    let mut stmt = stmts.into_iter().next().unwrap();

    // ── Step 3: Validate ──
    if !is_select_statement(&stmt) {
        return SqlBuildResult::err(
            "only SELECT queries can be sorted or filtered",
        );
    }

    // ── Step 4: Inject schema qualification ──
    if let Some(ref schema) = options.schema_context {
        if !schema.is_empty() {
            inject_schema(&mut stmt, schema, &options.database_type);
        }
    }

    // ── Step 5: Determine if we need wrapping ──
    let needs_wrapping = !options.sort.is_empty()
        || !options.filters.is_empty()
        || options.limit.is_some()
        || options.offset.is_some();

    if !needs_wrapping {
        // No sort/filter/pagination — return the (potentially schema-qualified) SQL as-is
        return SqlBuildResult::ok(stmt_to_string(&stmt, &options.database_type));
    }

    // ── Step 6: Wrap in derived table ──
    let inner_sql = stmt_to_string(&stmt, &options.database_type);
    let alias = "_sqlkit_grid";
    let quoted_alias = quote_ident(alias, &options.database_type);
    let base = format!("SELECT * FROM ({}) AS {}", inner_sql, quoted_alias);

    // ── Step 7: Apply filters ──
    let base = if !options.filters.is_empty() {
        let where_clause = build_filter_where(&options.filters, &options.database_type);
        format!("{} WHERE {}", base, where_clause)
    } else {
        base
    };

    // ── Step 8: Extract SELECT columns for alias resolution ──
    let select_columns = extract_select_columns(&stmt);
    let has_expression_columns =
        select_columns.iter().any(|(name, _)| name.contains(' ') || name.contains('('));

    // ── Step 9: Apply sorting ──
    let base = if !options.sort.is_empty() {
        let order_by = build_order_by(
            &options.sort,
            &options.database_type,
            &select_columns,
            has_expression_columns,
        );
        format!("{} ORDER BY {}", base, order_by)
    } else {
        base
    };

    // ── Step 10: Apply pagination ──
    let base = build_pagination(&base, options.limit, options.offset, &options.database_type);

    SqlBuildResult::ok(base)
}

/// Strip trailing semicolons and SQL comments.
pub fn strip_trailing(sql: &str) -> String {
    let s = sql.trim();
    // Strip single-line comments (-- ...) that trail after the main statement
    let s = strip_trailing_line_comments(s);
    // Strip trailing semicolons
    s.trim_end().trim_end_matches(';').trim_end().to_string()
}

/// Remove trailing `-- line comments` and `/* block comments */`
fn strip_trailing_line_comments(sql: &str) -> &str {
    let sql = sql.trim_end();
    // Handle multi-line: find last `--` or `/*` that starts a trailing comment
    // We only strip comments that appear after the last meaningful token.
    // Simple approach: find `--` and `/*` at the end
    if let Some(pos) = sql.rfind("--") {
        // Make sure the `--` is not inside a string literal (rough check)
        let before = &sql[..pos];
        if !before.contains('\'') || count_unmatched_quotes(before) % 2 == 0 {
            return before.trim_end();
        }
    }
    if let Some(pos) = sql.rfind("/*") {
        let before = &sql[..pos];
        if !before.contains('\'') || count_unmatched_quotes(before) % 2 == 0 {
            if let Some(end) = sql[pos..].find("*/") {
                let end_pos = pos + end + 2;
                if end_pos >= sql.len() {
                    return before.trim_end();
                }
            }
        }
    }
    sql
}

fn count_unmatched_quotes(s: &str) -> usize {
    let mut count = 0;
    let mut in_string = false;
    let mut prev_escape = false;
    for ch in s.chars() {
        if prev_escape {
            prev_escape = false;
            continue;
        }
        if ch == '\\' {
            prev_escape = true;
            continue;
        }
        if ch == '\'' {
            in_string = !in_string;
            if !in_string {
                count += 2; // matched pair
            }
        }
    }
    if in_string {
        count + 1
    } else {
        count
    }
    // Return odd if there's an unmatched quote
}

// ── SQL building helpers ──

fn build_filter_where(filters: &[FilterRule], db_type: &str) -> String {
    let clauses: Vec<String> = filters
        .iter()
        .map(|f| {
            let col = quote_ident(&f.column, db_type);
            let esc = |v: &str| v.replace('\'', "''");
            match f.operator {
                FilterOperator::Eq => format!("{} = '{}'", col, esc(&f.value)),
                FilterOperator::Neq => format!("{} != '{}'", col, esc(&f.value)),
                FilterOperator::Like => {
                    format!("{} LIKE '%{}%'", col, esc(&f.value))
                }
                FilterOperator::Gt => format!("{} > '{}'", col, esc(&f.value)),
                FilterOperator::Lt => format!("{} < '{}'", col, esc(&f.value)),
                FilterOperator::Gte => format!("{} >= '{}'", col, esc(&f.value)),
                FilterOperator::Lte => format!("{} <= '{}'", col, esc(&f.value)),
                FilterOperator::Between => {
                    let v2 = f.value2.as_deref().unwrap_or("");
                    format!(
                        "{} >= '{}' AND {} <= '{}'",
                        col,
                        esc(&f.value),
                        col,
                        esc(v2)
                    )
                }
            }
        })
        .collect();

    clauses.join(" AND ")
}

fn build_order_by(
    sort: &[SortRule],
    db_type: &str,
    select_columns: &[(String, usize)],
    has_expression_columns: bool,
) -> String {
    sort.iter()
        .map(|s| {
            let dir = match s.direction {
                SortDirection::ASC => "ASC",
                SortDirection::DESC => "DESC",
            };
            // If the column is an expression in the SELECT list (not a bare column),
            // use its positional alias
            let col_ref = if has_expression_columns {
                if let Some((_, idx)) = select_columns.iter().find(|(name, _)| name == &s.column) {
                    // Column index is 1-based in SQL
                    format!("{}", idx + 1)
                } else {
                    quote_ident(&s.column, db_type)
                }
            } else {
                quote_ident(&s.column, db_type)
            };
            format!("{} {}", col_ref, dir)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_pagination(sql: &str, limit: Option<u64>, offset: Option<u64>, db_type: &str) -> String {
    let limit = limit.unwrap_or(0);
    let offset = offset.unwrap_or(0);

    if limit == 0 && offset == 0 {
        return sql.to_string();
    }

    match db_type {
        "sqlserver" => {
            if limit > 0 {
                format!(
                    "{} OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
                    sql, offset, limit
                )
            } else {
                format!("{} OFFSET {} ROWS", sql, offset)
            }
        }
        _ => {
            // PostgreSQL, MySQL, SQLite, etc.
            if limit > 0 {
                format!("{} LIMIT {} OFFSET {}", sql, limit, offset)
            } else {
                format!("{} LIMIT -1 OFFSET {}", sql, offset)
            }
        }
    }
}

/// Quote a SQL identifier per database dialect.
pub fn quote_ident(ident: &str, db_type: &str) -> String {
    match db_type {
        "postgres" | "sqlite" | "duckdb" | "jdbc" | "cockroachdb" => {
            format!("\"{}\"", ident.replace('\"', "\"\""))
        }
        "mysql" | "clickhouse" => format!("`{}`", ident.replace('`', "``")),
        "sqlserver" => format!("[{}]", ident.replace(']', "]]")),
        _ => ident.to_string(),
    }
}

// ── Schema injection ──

/// Inject schema qualification into all unqualified table references.
fn inject_schema(stmt: &mut Statement, schema: &str, db_type: &str) {
    match stmt {
        Statement::Query(query) => {
            inject_schema_in_query(query, schema, db_type);
        }
        _ => {}
    }
}

fn inject_schema_in_query(query: &mut Query, schema: &str, db_type: &str) {
    inject_schema_in_query_body(query.body.as_mut(), schema, db_type);
    // Also handle CTEs recursively
    if let Some(ref mut with) = query.with {
        for cte in with.cte_tables.iter_mut() {
            inject_schema_in_query(cte.query.as_mut(), schema, db_type);
        }
    }
}

fn inject_schema_in_query_body(body: &mut SetExpr, schema: &str, db_type: &str) {
    match body {
        SetExpr::Select(select) => {
            inject_schema_in_tables(&mut select.from, schema, db_type);
        }
        SetExpr::SetOperation { left, right, .. } => {
            inject_schema_in_query_body(left.as_mut(), schema, db_type);
            inject_schema_in_query_body(right.as_mut(), schema, db_type);
        }
        _ => {}
    }
}

fn inject_schema_in_tables(tables: &mut Vec<TableWithJoins>, schema: &str, db_type: &str) {
    for table in tables.iter_mut() {
        inject_schema_in_table_factor(&mut table.relation, schema, db_type);
        for join in table.joins.iter_mut() {
            inject_schema_in_table_factor(&mut join.relation, schema, db_type);
        }
    }
}

fn inject_schema_in_table_factor(factor: &mut TableFactor, schema: &str, db_type: &str) {
    match factor {
        TableFactor::Table { ref mut name, .. } => {
            // Only qualify if no schema is already present
            if name.0.len() == 1 {
                name.0
                    .insert(0, ObjectNamePart::Identifier(Ident::new(schema)));
            }
        }
        TableFactor::Derived { .. } | TableFactor::TableFunction { .. } => {
            // Skip subqueries and table functions — they have their own scope
        }
        TableFactor::NestedJoin { table_with_joins, .. } => {
            inject_schema_in_table_factor(&mut table_with_joins.relation, schema, db_type);
            for join in table_with_joins.joins.iter_mut() {
                inject_schema_in_table_factor(&mut join.relation, schema, db_type);
            }
        }
        _ => {}
    }
}

// ── AST helpers ──

fn is_select_statement(stmt: &Statement) -> bool {
    match stmt {
        Statement::Query(query) => {
            // SELECT, WITH, set operations (UNION, INTERSECT, EXCEPT) containing SELECT
            match query.body.as_ref() {
                SetExpr::Select(_) | SetExpr::SetOperation { .. } => true,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Extract (column_name_or_alias, position) from SELECT list.
fn extract_select_columns(stmt: &Statement) -> Vec<(String, usize)> {
    match stmt {
        Statement::Query(query) => match query.body.as_ref() {
            SetExpr::Select(select) => extract_select_items(&select.projection),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn extract_select_items(items: &[SelectItem]) -> Vec<(String, usize)> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let name = match item {
                SelectItem::UnnamedExpr(expr) => expr_to_name(expr),
                SelectItem::ExprWithAlias { alias, .. } => alias.value.clone(),
                SelectItem::QualifiedWildcard(kind, _) => match kind {
                    SelectItemQualifiedWildcardKind::ObjectName(obj) => obj.to_string() + ".*",
                    SelectItemQualifiedWildcardKind::Expr(e) => format!("({}).*", expr_to_name(e)),
                },
                SelectItem::Wildcard(_) => "*".to_string(),
            };
            (name, i)
        })
        .collect()
}

fn expr_to_name(expr: &Expr) -> String {
    match expr {
        Expr::Identifier(ident) => ident.value.clone(),
        Expr::CompoundIdentifier(idents) => {
            idents.iter().map(|i| i.value.clone()).collect::<Vec<_>>().join(".")
        }
        Expr::Function(f) => f.name.to_string(),
        Expr::BinaryOp { left, op, right } => {
            format!("{} {} {}", expr_to_name(left), op, expr_to_name(right))
        }
        Expr::Cast { expr, data_type, .. } => {
            format!("CAST({} AS {})", expr_to_name(expr), data_type)
        }
        Expr::Value(vws) => match &vws.value {
            Value::Number(n, _) => n.to_string(),
            Value::SingleQuotedString(s) => format!("'{}'", s),
            other => format!("{:?}", other),
        },
        _ => format!("{:?}", expr),
    }
}

/// Convert a Statement back to SQL string.
fn stmt_to_string(stmt: &Statement, _db_type: &str) -> String {
    // Use the Display implementation which is dialect-aware via the parser dialect
    stmt.to_string()
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_trailing_semicolon() {
        assert_eq!(strip_trailing("SELECT 1;"), "SELECT 1");
        assert_eq!(strip_trailing("SELECT 1;;;"), "SELECT 1");
        assert_eq!(strip_trailing("  SELECT 1;  "), "SELECT 1");
        assert_eq!(strip_trailing("SELECT 1"), "SELECT 1");
        assert_eq!(strip_trailing(""), "");
    }

    #[test]
    fn test_strip_trailing_comment() {
        assert_eq!(strip_trailing("SELECT 1 -- comment"), "SELECT 1");
        assert_eq!(strip_trailing("SELECT 1;-- comment"), "SELECT 1");
        assert_eq!(
            strip_trailing("SELECT 1 /* block comment */"),
            "SELECT 1"
        );
    }

    #[test]
    fn test_strip_trailing_comment_and_semicolons() {
        assert_eq!(
            strip_trailing("SELECT * FROM users ;-- WHERE id = '123'"),
            "SELECT * FROM users"
        );
        assert_eq!(
            strip_trailing("SELECT id, name FROM users; -- filter"),
            "SELECT id, name FROM users"
        );
    }

    #[test]
    fn test_wrap_query_no_sort_filter() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT * FROM users".to_string(),
            database_type: "postgres".to_string(),
            schema_context: None,
            sort: vec![],
            filters: vec![],
            limit: None,
            offset: None,
        });
        assert!(result.ok);
        assert_eq!(result.sql.unwrap(), "SELECT * FROM users");
    }

    #[test]
    fn test_wrap_query_trailing_semicolon_comment() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT * FROM users ;-- WHERE id = 'abc'".to_string(),
            database_type: "postgres".to_string(),
            schema_context: None,
            sort: vec![],
            filters: vec![],
            limit: None,
            offset: None,
        });
        assert!(result.ok, "should handle trailing semicolons: {:?}", result.reason);
    }

    #[test]
    fn test_wrap_query_with_filter() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT * FROM users".to_string(),
            database_type: "postgres".to_string(),
            schema_context: None,
            sort: vec![],
            filters: vec![FilterRule {
                column: "id".to_string(),
                operator: FilterOperator::Eq,
                value: "42".to_string(),
                value2: None,
            }],
            limit: None,
            offset: None,
        });
        assert!(result.ok, "filter should succeed: {:?}", result.reason);
        let sql = result.sql.unwrap();
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("\"id\" = '42'"));
    }

    #[test]
    fn test_wrap_query_with_sort() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT id, name FROM users".to_string(),
            database_type: "postgres".to_string(),
            schema_context: None,
            sort: vec![SortRule {
                column: "name".to_string(),
                direction: SortDirection::ASC,
            }],
            filters: vec![],
            limit: None,
            offset: None,
        });
        assert!(result.ok, "sort should succeed: {:?}", result.reason);
        let sql = result.sql.unwrap();
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("\"name\" ASC"));
    }

    #[test]
    fn test_wrap_query_rejects_multi_statement() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT 1; DROP TABLE users".to_string(),
            database_type: "postgres".to_string(),
            schema_context: None,
            sort: vec![],
            filters: vec![],
            limit: None,
            offset: None,
        });
        assert!(!result.ok, "multi-statement should be rejected");
        assert!(result.reason.unwrap().contains("multiple"));
    }

    #[test]
    fn test_wrap_query_rejects_dml() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "DELETE FROM users".to_string(),
            database_type: "postgres".to_string(),
            schema_context: None,
            sort: vec![],
            filters: vec![],
            limit: None,
            offset: None,
        });
        assert!(!result.ok, "DML should be rejected");
        assert!(result.reason.unwrap().contains("only SELECT"));
    }

    #[test]
    fn test_wrap_query_with_pagination() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT * FROM users".to_string(),
            database_type: "postgres".to_string(),
            schema_context: None,
            sort: vec![],
            filters: vec![],
            limit: Some(50),
            offset: Some(10),
        });
        assert!(result.ok, "pagination should succeed: {:?}", result.reason);
        let sql = result.sql.unwrap();
        assert!(sql.contains("LIMIT 50"));
        assert!(sql.contains("OFFSET 10"));
    }

    #[test]
    fn test_schema_injection() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT * FROM users".to_string(),
            database_type: "postgres".to_string(),
            schema_context: Some("public".to_string()),
            sort: vec![],
            filters: vec![],
            limit: None,
            offset: None,
        });
        assert!(result.ok, "schema injection should succeed: {:?}", result.reason);
        let sql = result.sql.unwrap();
        // The table should now be qualified as "public"."users"
        assert!(sql.contains("public") && sql.contains("users"));
    }

    #[test]
    fn test_wrap_query_all_together() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT id, name, email FROM users".to_string(),
            database_type: "postgres".to_string(),
            schema_context: Some("public".to_string()),
            sort: vec![
                SortRule { column: "name".to_string(), direction: SortDirection::ASC },
            ],
            filters: vec![
                FilterRule {
                    column: "email".to_string(),
                    operator: FilterOperator::Like,
                    value: "test".to_string(),
                    value2: None,
                },
            ],
            limit: Some(100),
            offset: Some(0),
        });
        assert!(result.ok, "all combined: {:?}", result.reason);
        let sql = result.sql.unwrap();
        // Schema is injected as unqualified table ref → "public.users" or "public"."users"
        assert!(
            sql.contains("public.") || sql.contains("\"public\""),
            "schema not found in: {}",
            sql
        );
        assert!(sql.contains("WHERE"), "where: {}", sql);
        assert!(sql.contains("ORDER BY"), "order: {}", sql);
        assert!(sql.contains("LIMIT 100"), "limit: {}", sql);
    }

    #[test]
    fn test_mysql_dialect() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT * FROM users ORDER BY name".to_string(),
            database_type: "mysql".to_string(),
            schema_context: None,
            sort: vec![SortRule {
                column: "name".to_string(),
                direction: SortDirection::DESC,
            }],
            filters: vec![],
            limit: Some(10),
            offset: None,
        });
        assert!(result.ok, "mysql: {:?}", result.reason);
        let sql = result.sql.unwrap();
        // MySQL uses backtick quoting
        assert!(sql.contains('`'), "mysql should use backticks: {}", sql);
    }

    #[test]
    fn test_sqlserver_dialect() {
        let result = wrap_query(WrapQueryOptions {
            original_sql: "SELECT * FROM users".to_string(),
            database_type: "sqlserver".to_string(),
            schema_context: None,
            sort: vec![SortRule {
                column: "name".to_string(),
                direction: SortDirection::ASC,
            }],
            filters: vec![],
            limit: Some(10),
            offset: Some(0),
        });
        assert!(result.ok, "sqlserver: {:?}", result.reason);
        let sql = result.sql.unwrap();
        assert!(sql.contains("OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY"));
    }
}
