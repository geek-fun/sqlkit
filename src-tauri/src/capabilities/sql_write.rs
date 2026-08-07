//! SQL write-capability tools for the MCP bridge.
//!
//! Splits write operations out of `sqlkit__execute_query` so the policy gate
//! (McpPolicy) can enforce them by risk level: writes are Elevated, deletes
//! and DDL are Destructive (gated by `confirm_destructive`).

use std::sync::Arc;

use serde_json::{json, Value};
use sqlparser::ast::Statement;
use sqlparser::dialect::{GenericDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;

use data_studio_agent::capabilities::registry::CapabilityRegistry;
use data_studio_agent::capabilities::types::{Capability, CapabilityHandler, RiskLevel, SourceKind};

use super::sql::{execute_on_adapter, resolve_adapter};

/// Statement category for risk-gated dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqlKind {
    Read,
    Write,
    Delete,
    Ddl,
    Other,
}

/// Classify a SQL statement by its top-level AST variant, using the dialect
/// that matches the connection's database type.
pub(crate) fn classify_sql(db_type: &str, sql: &str) -> Result<SqlKind, String> {
    let dialect: Box<dyn sqlparser::dialect::Dialect> = match db_type.to_lowercase().as_str() {
        "postgres" | "postgresql" | "duckdb" | "cockroachdb" | "gbase8c" | "kingbasees"
        | "yashandb" | "xugudb" | "oceanbase" | "dameng" => Box::new(PostgreSqlDialect {}),
        "mysql" | "clickhouse" | "gbase8a" => Box::new(MySqlDialect {}),
        "sqlserver" => Box::new(MsSqlDialect {}),
        _ => Box::new(GenericDialect {}),
    };

    let stmts = Parser::parse_sql(&*dialect, sql)
        .map_err(|e| format!("parse error: {}", e))?;
    if stmts.len() > 1 {
        return Err("multiple statements are not supported".to_string());
    }
    let Some(stmt) = stmts.into_iter().next() else {
        return Err("empty query".to_string());
    };
    Ok(classify_statement(&stmt))
}

fn classify_statement(stmt: &Statement) -> SqlKind {
    match stmt {
        Statement::Query(_)
        | Statement::Explain { .. }
        | Statement::ExplainTable { .. }
        | Statement::ShowVariable { .. }
        | Statement::ShowVariables { .. }
        | Statement::ShowTables { .. }
        | Statement::ShowColumns { .. }
        | Statement::ShowViews { .. }
        | Statement::ShowSchemas { .. }
        | Statement::ShowDatabases { .. }
        | Statement::ShowFunctions { .. }
        | Statement::ShowStatus { .. }
        | Statement::ShowCollation { .. }
        | Statement::ShowCreate { .. }
        | Statement::ShowObjects(_) => SqlKind::Read,
        Statement::Insert(_) | Statement::Update { .. } | Statement::Merge { .. } => SqlKind::Write,
        Statement::Delete(_) | Statement::Truncate { .. } => SqlKind::Delete,
        Statement::CreateTable(_)
        | Statement::CreateIndex(_)
        | Statement::CreateDatabase { .. }
        | Statement::CreateSchema { .. }
        | Statement::CreateView { .. }
        | Statement::CreateFunction(_)
        | Statement::CreateProcedure { .. }
        | Statement::CreateTrigger { .. }
        | Statement::CreateSequence { .. }
        | Statement::CreateType { .. }
        | Statement::CreateExtension { .. }
        | Statement::CreateRole { .. }
        | Statement::CreatePolicy { .. }
        | Statement::CreateMacro { .. }
        | Statement::CreateStage { .. }
        | Statement::CreateSecret { .. }
        | Statement::CreateVirtualTable { .. }
        | Statement::CreateConnector(_)
        | Statement::AlterTable { .. }
        | Statement::AlterIndex { .. }
        | Statement::AlterView { .. }
        | Statement::AlterRole { .. }
        | Statement::AlterType(_)
        | Statement::AlterSession { .. }
        | Statement::AlterPolicy { .. }
        | Statement::AlterConnector { .. }
        | Statement::Drop { .. }
        | Statement::DropFunction { .. }
        | Statement::DropProcedure { .. }
        | Statement::DropTrigger { .. }
        | Statement::DropExtension { .. }
        | Statement::DropPolicy { .. }
        | Statement::DropSecret { .. }
        | Statement::DropConnector { .. }
        | Statement::RenameTable(_)
        | Statement::AttachDatabase { .. }
        | Statement::AttachDuckDBDatabase { .. }
        | Statement::DetachDuckDBDatabase { .. }
        | Statement::Unload { .. }
        | Statement::Load { .. }
        | Statement::LoadData { .. }
        | Statement::Cache { .. }
        | Statement::UNCache { .. }
        | Statement::OptimizeTable { .. }
        | Statement::Analyze { .. }
        | Statement::Msck { .. }
        | Statement::Grant { .. }
        | Statement::Revoke { .. }
        | Statement::Comment { .. }
        | Statement::LockTables { .. }
        | Statement::UnlockTables { .. }
        | Statement::SetVariable { .. }
        | Statement::SetNames { .. }
        | Statement::SetNamesDefault { .. }
        | Statement::SetRole { .. }
        | Statement::SetSessionParam(_)
        | Statement::SetTimeZone { .. }
        | Statement::SetTransaction { .. }
        | Statement::Commit { .. }
        | Statement::Rollback { .. }
        | Statement::Savepoint { .. }
        | Statement::ReleaseSavepoint { .. }
        | Statement::StartTransaction { .. }
        | Statement::Declare { .. }
        | Statement::Prepare { .. }
        | Statement::Execute { .. }
        | Statement::Deallocate { .. }
        | Statement::Call(_)
        | Statement::Copy { .. }
        | Statement::CopyIntoSnowflake { .. }
        | Statement::Kill { .. }
        | Statement::Flush { .. }
        | Statement::Pragma { .. }
        | Statement::Use(_)
        | Statement::Install { .. }
        | Statement::RaisError { .. }
        | Statement::Fetch { .. }
        | Statement::Close { .. }
        | Statement::Discard { .. }
        | Statement::Assert { .. }
        | Statement::Directory { .. }
        | Statement::Remove(_)
        | Statement::List(_)
        | Statement::LISTEN { .. }
        | Statement::NOTIFY { .. }
        | Statement::UNLISTEN { .. } => SqlKind::Ddl,
        // Future sqlparser variants fall here rather than failing to compile.
        #[allow(unreachable_patterns)]
        _ => SqlKind::Other,
    }
}

/// Guard a capability against the statement category it is allowed to run.
/// `execute_query` is read-only; anything else returns an actionable error.
pub(crate) fn ensure_read_only(db_type: &str, sql: &str) -> Result<(), String> {
    match classify_sql(db_type, sql)? {
        SqlKind::Read => Ok(()),
        SqlKind::Write => Err(
            "Only SELECT/SHOW/EXPLAIN statements are allowed in sqlkit__execute_query. \
             Use sqlkit__execute_write for INSERT/UPDATE/MERGE."
                .to_string(),
        ),
        SqlKind::Delete => Err(
            "DELETE/TRUNCATE must use sqlkit__execute_delete (destructive, requires \
             Full Access with Confirm Destructive in Settings → MCP Bridge)."
                .to_string(),
        ),
        SqlKind::Ddl => Err(
            "DDL statements (CREATE/ALTER/DROP) must use sqlkit__execute_ddl (destructive, \
             requires Full Access with Confirm Destructive in Settings → MCP Bridge)."
                .to_string(),
        ),
        SqlKind::Other => Err(
            "Statement type is not recognized as read-only. Use sqlkit__execute_write, \
             sqlkit__execute_delete, or sqlkit__execute_ddl as appropriate."
                .to_string(),
        ),
    }
}

fn connection_id_from(config: Option<&Value>) -> Result<String, String> {
    super::sql::get_connection_id(config)
}

fn sql_from(args: &Value) -> Result<String, String> {
    args.get("sql")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'sql' argument".to_string())
}

/// Guard that the statement belongs to the allowed categories, then execute.
async fn run_classified(
    allowed: &[SqlKind],
    args: &Value,
    connection_config: Option<&Value>,
) -> Result<String, String> {
    let conn_id = connection_id_from(connection_config)?;
    let sql = sql_from(args)?;
    let adapter = resolve_adapter(&conn_id).await?;

    let db_type = adapter_db_type(&adapter);
    let kind = classify_sql(&db_type, &sql)?;
    if !allowed.contains(&kind) {
        return Err(format!(
            "sqlkit__execute_{} does not accept {:?} statements. {:?} statements go to {}.",
            match allowed[0] {
                SqlKind::Write => "write",
                SqlKind::Delete => "delete",
                _ => "ddl",
            },
            kind,
            kind,
            match kind {
                SqlKind::Read => "sqlkit__execute_query",
                SqlKind::Write => "sqlkit__execute_write",
                SqlKind::Delete => "sqlkit__execute_delete",
                SqlKind::Ddl => "sqlkit__execute_ddl",
                SqlKind::Other => "an explicit tool",
            },
        ));
    }

    let result = execute_on_adapter(&adapter, &sql).await?;
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

pub(crate) fn adapter_db_type(adapter: &crate::state::ActiveConnection) -> String {
    match adapter {
        crate::state::ActiveConnection::Postgres(_) => "postgres".to_string(),
        crate::state::ActiveConnection::MySQL(_) => "mysql".to_string(),
        crate::state::ActiveConnection::SQLite(_) => "sqlite".to_string(),
        crate::state::ActiveConnection::SQLServer(_) => "sqlserver".to_string(),
        crate::state::ActiveConnection::ClickHouse(_) => "clickhouse".to_string(),
        crate::state::ActiveConnection::JdbcBridge(_) => "generic".to_string(),
        crate::state::ActiveConnection::HttpSql(_) => "generic".to_string(),
        crate::state::ActiveConnection::Rqlite(_) => "generic".to_string(),
        crate::state::ActiveConnection::Turso(_) => "generic".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

struct ExecuteWriteHandler;
struct ExecuteDeleteHandler;
struct ExecuteDdlHandler;

#[async_trait::async_trait]
#[async_trait::async_trait]
impl CapabilityHandler for ExecuteWriteHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        run_classified(&[SqlKind::Write], args, connection_config).await
    }
}

#[async_trait::async_trait]
#[async_trait::async_trait]
impl CapabilityHandler for ExecuteDeleteHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        run_classified(&[SqlKind::Delete], args, connection_config).await
    }
}

#[async_trait::async_trait]
#[async_trait::async_trait]
impl CapabilityHandler for ExecuteDdlHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        run_classified(&[SqlKind::Ddl], args, connection_config).await
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

fn connection_id_schema() -> Value {
    json!({
        "type": "string",
        "description": "The connection alias to use (e.g. 'mac-postgresql'). Use sqlkit__list_connections to see available connections."
    })
}

pub(crate) fn register_write_tools(reg: &mut CapabilityRegistry) {
    reg.register(Capability {
        name: "sqlkit__execute_write",
        description: "Execute a data-modifying SQL statement: INSERT, UPDATE, or MERGE. Use sqlkit__execute_query for reads and sqlkit__execute_delete / sqlkit__execute_ddl for destructive statements.",
        handler: Arc::new(ExecuteWriteHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "sql": {"type": "string", "description": "The INSERT/UPDATE/MERGE SQL statement"}
        }, "required": ["connection_id", "sql"]}),
        risk_level: RiskLevel::Elevated,
        required_permission: "create",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: false,
    });

    reg.register(Capability {
        name: "sqlkit__execute_delete",
        description: "Execute a destructive DELETE or TRUNCATE statement. DESTRUCTIVE: permanently removes data. Requires Full Access with Confirm Destructive in Settings → MCP Bridge.",
        handler: Arc::new(ExecuteDeleteHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "sql": {"type": "string", "description": "The DELETE or TRUNCATE SQL statement"}
        }, "required": ["connection_id", "sql"]}),
        risk_level: RiskLevel::Destructive,
        required_permission: "delete",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: false,
    });

    reg.register(Capability {
        name: "sqlkit__execute_ddl",
        description: "Execute a DDL statement (CREATE, ALTER, DROP, and other schema changes). DESTRUCTIVE for DROP/ALTER: permanently changes schema. Requires Full Access with Confirm Destructive in Settings → MCP Bridge.",
        handler: Arc::new(ExecuteDdlHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "sql": {"type": "string", "description": "The DDL SQL statement"}
        }, "required": ["connection_id", "sql"]}),
        risk_level: RiskLevel::Destructive,
        required_permission: "delete",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_select_as_read() {
        assert_eq!(classify_sql("postgres", "SELECT * FROM users").unwrap(), SqlKind::Read);
        assert_eq!(classify_sql("postgres", "WITH x AS (SELECT 1) SELECT * FROM x").unwrap(), SqlKind::Read);
    }

    #[test]
    fn classifies_show_as_read() {
        assert_eq!(classify_sql("mysql", "SHOW TABLES").unwrap(), SqlKind::Read);
        assert_eq!(classify_sql("sqlserver", "SHOW VARIABLES").unwrap(), SqlKind::Read);
    }

    #[test]
    fn classifies_insert_update_merge_as_write() {
        assert_eq!(
            classify_sql("postgres", "INSERT INTO users (id) VALUES (1)").unwrap(),
            SqlKind::Write
        );
        assert_eq!(
            classify_sql("postgres", "UPDATE users SET name = 'x' WHERE id = 1").unwrap(),
            SqlKind::Write
        );
        assert_eq!(
            classify_sql("sqlserver", "MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN UPDATE SET x = s.x").unwrap(),
            SqlKind::Write
        );
    }

    #[test]
    fn classifies_delete_truncate_as_delete() {
        assert_eq!(
            classify_sql("postgres", "DELETE FROM users WHERE id = 1").unwrap(),
            SqlKind::Delete
        );
        assert_eq!(
            classify_sql("postgres", "TRUNCATE TABLE users").unwrap(),
            SqlKind::Delete
        );
    }

    #[test]
    fn classifies_ddl_as_ddl() {
        assert_eq!(
            classify_sql("postgres", "CREATE TABLE t (id int)").unwrap(),
            SqlKind::Ddl
        );
        assert_eq!(
            classify_sql("postgres", "ALTER TABLE t ADD COLUMN c int").unwrap(),
            SqlKind::Ddl
        );
        assert_eq!(classify_sql("postgres", "DROP TABLE t").unwrap(), SqlKind::Ddl);
    }

    #[test]
    fn rejects_multiple_statements() {
        assert!(classify_sql("postgres", "SELECT 1; SELECT 2").is_err());
    }

    #[test]
    fn read_guard_rejects_write() {
        let err = ensure_read_only("postgres", "INSERT INTO t VALUES (1)").unwrap_err();
        assert!(err.contains("execute_write"), "got: {}", err);
    }

    #[test]
    fn read_guard_rejects_delete() {
        let err = ensure_read_only("postgres", "DELETE FROM t").unwrap_err();
        assert!(err.contains("execute_delete"), "got: {}", err);
    }

    #[test]
    fn read_guard_rejects_ddl() {
        let err = ensure_read_only("postgres", "DROP TABLE t").unwrap_err();
        assert!(err.contains("execute_ddl"), "got: {}", err);
    }

    #[test]
    fn read_guard_allows_select() {
        assert!(ensure_read_only("postgres", "SELECT 1").is_ok());
    }
}
