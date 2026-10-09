//! Classifies whether a SQL statement returns a result set or only an
//! affected-row count.
//!
//! Adapters use this to route statements to the row-retrieval path versus the
//! execute path. Classification happens in two steps: a fast keyword scan after
//! stripping leading comments and whitespace, then a parser fallback for
//! statements the keyword scan cannot see through (leading parentheses,
//! `VALUES`, DML with `RETURNING`). The fallback only upgrades to the row path
//! when the parse is unambiguous; anything inconclusive stays on the execute
//! path so DML keeps producing its affected-row count.

use sqlparser::ast::Statement;
use sqlparser::dialect::Dialect;
use sqlparser::parser::Parser;

/// Returns whether `sql` should be executed through the row-retrieval path.
///
/// `row_keywords` is the adapter's list of keywords whose statements always
/// produce a result set (e.g. SELECT, SHOW, EXPLAIN). When the fast scan is
/// inconclusive, the statement is parsed with `dialect`: a single `Query`, or
/// DML with RETURNING, also produces rows.
pub(crate) fn statement_returns_rows(
    sql: &str,
    row_keywords: &[&str],
    dialect: &dyn Dialect,
) -> bool {
    if let Some(keyword) = first_executable_keyword(sql) {
        if row_keywords.iter().any(|k| keyword.eq_ignore_ascii_case(k)) {
            return true;
        }
    }
    parsed_statement_returns_rows(sql, dialect)
}

/// First bare word in `sql`, skipping whitespace and leading `--` / `/* */`
/// comments. Returns `None` when the first executable token is not a word
/// (e.g. `(`) or nothing executable remains (unterminated comment) — those
/// cases fall through to the parser-based classification.
fn first_executable_keyword(sql: &str) -> Option<&str> {
    let bytes = sql.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        if bytes[i] == b'-' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 >= bytes.len() {
                return None;
            }
            i += 2;
            continue;
        }

        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        return if i == start {
            None
        } else {
            Some(&sql[start..i])
        };
    }
    None
}

/// Parser-based classification for statements the keyword scan cannot decide.
/// Only a single parsed statement is trusted: multi-statement scripts stay on
/// the execute path (conservative, matching the previous behavior).
fn parsed_statement_returns_rows(sql: &str, dialect: &dyn Dialect) -> bool {
    let Ok(statements) = Parser::parse_sql(dialect, sql) else {
        return false;
    };
    let [statement] = statements.as_slice() else {
        return false;
    };

    match statement {
        Statement::Query(_) => true,
        Statement::Insert(insert) => insert.returning.is_some(),
        Statement::Update(update) => update.returning.is_some(),
        Statement::Delete(delete) => delete.returning.is_some(),
        Statement::Merge(merge) => merge.output.is_some(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::dialect::{GenericDialect, MySqlDialect, PostgreSqlDialect};

    const PG_KEYWORDS: &[&str] = &["SELECT", "WITH", "SHOW", "EXPLAIN", "TABLE"];
    const MYSQL_KEYWORDS: &[&str] = &["SELECT", "SHOW", "DESCRIBE", "EXPLAIN", "WITH"];
    const SQLITE_KEYWORDS: &[&str] = &["SELECT", "PRAGMA", "EXPLAIN", "WITH"];

    #[test]
    fn plain_select_returns_rows() {
        assert!(statement_returns_rows(
            "SELECT 1",
            PG_KEYWORDS,
            &PostgreSqlDialect {}
        ));
        assert!(statement_returns_rows(
            "select 1",
            PG_KEYWORDS,
            &PostgreSqlDialect {}
        ));
        assert!(statement_returns_rows(
            "  \n\t SELECT 1",
            PG_KEYWORDS,
            &PostgreSqlDialect {}
        ));
    }

    #[test]
    fn line_comment_prefixed_select_returns_rows() {
        assert!(statement_returns_rows(
            "-- ① 是否已安装（最直接）\nSELECT extname FROM pg_extension WHERE extname = 'postgis';",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
        assert!(statement_returns_rows(
            "--comment\n--another\nSHOW TABLES",
            MYSQL_KEYWORDS,
            &MySqlDialect {},
        ));
        assert!(statement_returns_rows(
            "-- note\nPRAGMA table_info(t)",
            SQLITE_KEYWORDS,
            &GenericDialect {},
        ));
    }

    #[test]
    fn block_comment_prefixed_select_returns_rows() {
        assert!(statement_returns_rows(
            "/* header comment */ SELECT 1",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
        assert!(statement_returns_rows(
            "/* multi\nline */ WITH cte AS (SELECT 1) SELECT * FROM cte",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
    }

    #[test]
    fn unterminated_block_comment_is_not_rows() {
        assert!(!statement_returns_rows(
            "/* unterminated SELECT 1",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
    }

    #[test]
    fn comment_only_script_is_not_rows() {
        assert!(!statement_returns_rows(
            "-- nothing executable here\n",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
    }

    #[test]
    fn dml_without_returning_is_not_rows() {
        assert!(!statement_returns_rows(
            "INSERT INTO t (a) VALUES (1)",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
        assert!(!statement_returns_rows(
            "UPDATE t SET a = 1",
            PG_KEYWORDS,
            &PostgreSqlDialect {}
        ));
        assert!(!statement_returns_rows(
            "DELETE FROM t",
            PG_KEYWORDS,
            &PostgreSqlDialect {}
        ));
        assert!(!statement_returns_rows(
            "-- c\nUPDATE t SET a = 1",
            MYSQL_KEYWORDS,
            &MySqlDialect {},
        ));
    }

    #[test]
    fn dml_with_returning_is_rows() {
        assert!(statement_returns_rows(
            "-- c\nINSERT INTO t (a) VALUES (1) RETURNING a",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
        assert!(statement_returns_rows(
            "UPDATE t SET a = 1 WHERE id = 2 RETURNING a",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
        assert!(statement_returns_rows(
            "DELETE FROM t WHERE id = 2 RETURNING id",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
    }

    #[test]
    fn parenthesized_query_and_values_are_rows() {
        assert!(statement_returns_rows(
            "(SELECT 1) UNION (SELECT 2)",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
        assert!(statement_returns_rows(
            "VALUES (1), (2)",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
    }

    #[test]
    fn table_keyword_is_rows_for_postgres() {
        assert!(statement_returns_rows(
            "TABLE pg_extension",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
    }

    #[test]
    fn multi_statement_script_follows_first_statement_classification() {
        // The first statement decides — same as the previous prefix behavior.
        assert!(statement_returns_rows(
            "SELECT 1; SELECT 2;",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
        // The parser fallback never upgrades multi-statement scripts.
        assert!(!statement_returns_rows(
            "SET TIME ZONE 'UTC'; SELECT 1;",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
    }

    #[test]
    fn create_statement_is_not_rows() {
        assert!(!statement_returns_rows(
            "-- c\nCREATE EXTENSION IF NOT EXISTS postgis",
            PG_KEYWORDS,
            &PostgreSqlDialect {},
        ));
    }
}
