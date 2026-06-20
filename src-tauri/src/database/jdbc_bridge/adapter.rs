//! JDBC bridge adapter — implements DatabaseAdapter by delegating to a Java subprocess.

use crate::database::{
    adapter::DatabaseAdapter,
    config::ConnectionConfig,
    error::{DbError, DbResult},
    types::{
        ColumnInfo, ConnectionStatus, DatabaseSchema, QueryResult, QueryRow, QueryValue, TableInfo,
    },
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::launcher::JdbcBridgeLauncher;
use super::pool::JdbcBridgePool;
use super::protocol::{
    ConnectionStatusData, JdbcMethod, JdbcRequest, QueryResultData,
};

/// JDBC bridge adapter.
///
/// Spawns a Java subprocess (lazily) and communicates via JSON-RPC
/// over stdin/stdout. The Java side holds a HikariCP connection pool,
/// so connections are reused across queries.
pub struct JdbcBridgeAdapter {
    pub config: ConnectionConfig,
    launcher: Option<Arc<Mutex<JdbcBridgeLauncher>>>,
    conn_id: Option<String>,
}

impl JdbcBridgeAdapter {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            launcher: None,
            conn_id: None,
        }
    }

    /// Ensure all prerequisites are met: JRE, bridge JAR, and JDBC driver.
    /// Uses the fallback chain to try multiple driver versions automatically.
    async fn init_bridge(&mut self) -> DbResult<Arc<Mutex<JdbcBridgeLauncher>>> {
        let db_type = self.config.db_type;

        // Use fallback chain for JDBC-dependent databases (Oracle, DB2, H2, etc.)
        // For non-registry types, fall back to the old single-driver approach
        let (_version, conn_id, launcher) = super::fallback::run_fallback_chain(
            db_type,
            &self.config.host,
            self.config.port,
            self.config.database.as_deref(),
            &self.config.username,
            &self.config.password,
            self.config.oracle_options.as_ref(),
        )
        .await?;

        self.conn_id = Some(conn_id);
        self.launcher = Some(launcher.clone());

        Ok(launcher)
    }

    /// Get the launcher (must be initialized first via `connect`).
    fn launcher(&self) -> DbResult<&Arc<Mutex<JdbcBridgeLauncher>>> {
        self.launcher
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".to_string()))
    }

    async fn send_request(
        launcher: &Arc<Mutex<JdbcBridgeLauncher>>,
        req: JdbcRequest,
    ) -> DbResult<serde_json::Value> {
        let launcher = launcher.clone();
        let resp = tokio::task::spawn_blocking(move || {
            let mut guard = launcher.blocking_lock();
            guard.send_request(&req)
        })
        .await
        .map_err(|e| DbError::Connection(format!("JDBC bridge task panicked: {}", e)))??;
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }

    fn parse_query_result(data: serde_json::Value) -> DbResult<QueryResult> {
        let qr: QueryResultData = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse query result: {}", e)))?;

        let rows: Vec<QueryRow> = qr
            .rows
            .into_iter()
            .map(|row| {
                let mut map: HashMap<String, QueryValue> = HashMap::new();
                for (i, val) in row.into_iter().enumerate() {
                    let col_name = qr.columns.get(i).cloned().unwrap_or_default();
                    let qv = match val {
                        serde_json::Value::Null => QueryValue::Null,
                        serde_json::Value::String(s) => QueryValue::String(s),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                QueryValue::Int(i)
                            } else {
                                QueryValue::String(n.to_string())
                            }
                        }
                        serde_json::Value::Bool(b) => QueryValue::String(b.to_string()),
                        _ => QueryValue::String(String::new()),
                    };
                    map.insert(col_name, qv);
                }
                map
            })
            .collect();

        Ok(QueryResult {
            columns: qr.columns,
            column_types: Vec::new(),
            rows,
            rows_affected: qr.rows_affected,
            execution_time_ms: None,
            truncated: false,
        })
    }

    fn parse_connection_status(data: serde_json::Value) -> DbResult<ConnectionStatus> {
        let cs: ConnectionStatusData = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse status: {}", e)))?;
        Ok(cs.into())
    }
}

#[async_trait]
impl DatabaseAdapter for JdbcBridgeAdapter {
    type Pool = JdbcBridgePool;

    async fn connect(&mut self) -> DbResult<()> {
        self.init_bridge().await?;
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        if let Some(launcher) = &self.launcher {
            let mut guard = launcher.lock().await;
            guard.shutdown();
        }
        self.launcher = None;
        self.conn_id = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(
                JdbcMethod::TestConnection,
                serde_json::json!({
                    "conn_id": self.conn_id,
                }),
            ),
        )
        .await?;
        Self::parse_connection_status(data)
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let launcher = self.launcher()?;
        let statements = split_sql_statements(query);
        if statements.is_empty() {
            return Ok(QueryResult {
                columns: Vec::new(),
                column_types: Vec::new(),
                rows: Vec::new(),
                rows_affected: Some(0),
                execution_time_ms: None,
                truncated: false,
            });
        }
        // Execute each statement individually; return the last non-error result.
        // Many JDBC drivers (notably Dameng) reject multiple DDL/DML statements
        // sent as a single string, so splitting is required.
        let mut last_result = None;
        for stmt in &statements {
            let data = Self::send_request(
                &launcher,
                JdbcRequest::new(
                    JdbcMethod::ExecuteQuery,
                    serde_json::json!({
                        "conn_id": self.conn_id,
                        "sql": stmt,
                    }),
                ),
            )
            .await?;
            last_result = Some(Self::parse_query_result(data)?);
        }
        last_result.ok_or_else(|| DbError::Connection("No statements executed".to_string()))
    }

    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(
                JdbcMethod::ListDatabases,
                serde_json::json!({
                    "conn_id": self.conn_id,
                }),
            ),
        )
        .await?;
        let names: Vec<String> = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse database list: {}", e)))?;
        Ok(names
            .into_iter()
            .map(|name| DatabaseSchema {
                name,
                description: None,
                is_system: false,
                metadata: HashMap::new(),
            })
            .collect())
    }

    async fn list_schemas(&self, database: Option<&str>) -> DbResult<Vec<String>> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(
                JdbcMethod::ListSchemas,
                serde_json::json!({
                    "conn_id": self.conn_id,
                    "database": database,
                }),
            ),
        )
        .await?;
        serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse schema list: {}", e)))
    }

    async fn list_tables(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(
                JdbcMethod::ListTables,
                serde_json::json!({
                    "conn_id": self.conn_id,
                    "database": database,
                    "schema": schema,
                }),
            ),
        )
        .await?;
        let tables: Vec<serde_json::Value> = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse table list: {}", e)))?;
        Ok(tables
            .into_iter()
            .filter_map(|t| {
                Some(TableInfo {
                    schema: t.get("schema")?.as_str().map(|s| s.to_string()),
                    name: t.get("name")?.as_str()?.to_string(),
                    table_type: t
                        .get("table_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("TABLE")
                        .to_string(),
                    row_count: t.get("row_count").and_then(|v| v.as_u64()),
                    size_bytes: None,
                    description: None,
                    metadata: HashMap::new(),
                })
            })
            .collect())
    }

    async fn list_columns(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<Vec<ColumnInfo>> {
        let launcher = self.launcher()?;
        let data = Self::send_request(
            &launcher,
            JdbcRequest::new(
                JdbcMethod::ListColumns,
                serde_json::json!({
                    "conn_id": self.conn_id,
                    "database": database,
                    "schema": schema,
                    "table": table,
                }),
            ),
        )
        .await?;
        let cols: Vec<serde_json::Value> = serde_json::from_value(data)
            .map_err(|e| DbError::Connection(format!("Failed to parse column list: {}", e)))?;
        Ok(cols
            .into_iter()
            .filter_map(|c| {
                Some(ColumnInfo {
                    name: c.get("name")?.as_str()?.to_string(),
                    data_type: c
                        .get("data_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    nullable: c.get("nullable").and_then(|v| v.as_bool()).unwrap_or(true),
                    default_value: c
                        .get("default_value")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    is_primary_key: c
                        .get("is_primary_key")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    is_auto_increment: c
                        .get("is_auto_increment")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    max_length: c
                        .get("max_length")
                        .and_then(|v| v.as_u64().map(|x| x as u32)),
                    precision: c
                        .get("precision")
                        .and_then(|v| v.as_u64().map(|x| x as u32)),
                    scale: c.get("scale").and_then(|v| v.as_u64().map(|x| x as u32)),
                    description: None,
                    metadata: HashMap::new(),
                })
            })
            .collect())
    }

    async fn get_table_info(
        &self,
        database: Option<&str>,
        schema: Option<&str>,
        table: &str,
    ) -> DbResult<TableInfo> {
        let tables = self.list_tables(database, schema).await?;
        tables
            .into_iter()
            .find(|t| t.name == table)
            .ok_or_else(|| DbError::Connection(format!("Table '{}' not found", table)))
    }

    fn get_pool(&self) -> Option<Arc<Self::Pool>> {
        None
    }

    fn get_config(&self) -> &ConnectionConfig {
        &self.config
    }
}

/// Split a SQL string into individual statements separated by `;`.
///
/// Handles:
/// - Single-quoted string literals (`'...'`)
/// - Double-quoted identifiers (`"..."`)
/// - `--` single-line comments
/// - `/* ... */` block comments
/// - Trailing semicolons and whitespace
/// - Empty statements are skipped
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Single-quoted string literal
        if c == '\'' {
            current.push(c);
            i += 1;
            while i < chars.len() {
                current.push(chars[i]);
                if chars[i] == '\'' {
                    // Check for escaped single quote ''
                    if i + 1 < chars.len() && chars[i + 1] == '\'' {
                        i += 1;
                        current.push(chars[i]);
                    } else {
                        break;
                    }
                }
                i += 1;
            }
        }
        // Double-quoted identifier
        else if c == '"' {
            current.push(c);
            i += 1;
            while i < chars.len() {
                current.push(chars[i]);
                if chars[i] == '"' {
                    // Check for escaped double quote ""
                    if i + 1 < chars.len() && chars[i + 1] == '"' {
                        i += 1;
                        current.push(chars[i]);
                    } else {
                        break;
                    }
                }
                i += 1;
            }
        }
        // Single-line comment
        else if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        }
        // Block comment
        else if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() {
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
        }
        // Statement separator
        else if c == ';' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
        } else {
            current.push(c);
        }

        i += 1;
    }

    // Last statement (after final semicolon or no trailing semicolon)
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }

    statements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_single_statement() {
        let stmts = split_sql_statements("SELECT * FROM users");
        assert_eq!(stmts, vec!["SELECT * FROM users"]);
    }

    #[test]
    fn test_split_with_trailing_semicolon() {
        let stmts = split_sql_statements("SELECT * FROM users;");
        assert_eq!(stmts, vec!["SELECT * FROM users"]);
    }

    #[test]
    fn test_split_multi_statement() {
        let stmts = split_sql_statements(
            "CREATE TABLE t (id INT); INSERT INTO t VALUES (1); SELECT * FROM t",
        );
        assert_eq!(
            stmts,
            vec![
                "CREATE TABLE t (id INT)",
                "INSERT INTO t VALUES (1)",
                "SELECT * FROM t",
            ]
        );
    }

    #[test]
    fn test_split_comment_statement() {
        let stmts = split_sql_statements(
            "CREATE TABLE t (id INT);\nCOMMENT ON TABLE t IS 'hello';",
        );
        assert_eq!(
            stmts,
            vec![
                "CREATE TABLE t (id INT)",
                "COMMENT ON TABLE t IS 'hello'",
            ]
        );
    }

    #[test]
    fn test_split_with_semicolon_in_string() {
        let stmts = split_sql_statements("SELECT 'hello;world' AS x");
        assert_eq!(stmts, vec!["SELECT 'hello;world' AS x"]);
    }

    #[test]
    fn test_skip_empty_statements() {
        let stmts = split_sql_statements(";;SELECT 1;;;");
        assert_eq!(stmts, vec!["SELECT 1"]);
    }

    #[test]
    fn test_split_complex_ddl() {
        let sql = "CREATE TABLE SYSDBA.CLASSES (\n    class_id INT PRIMARY KEY\n);\nCOMMENT ON TABLE SYSDBA.CLASSES IS '班级信息表';\nCOMMENT ON COLUMN SYSDBA.CLASSES.class_id IS '班级ID';";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 3);
        assert!(stmts[0].starts_with("CREATE TABLE"));
        assert!(stmts[1].starts_with("COMMENT ON TABLE"));
        assert!(stmts[2].starts_with("COMMENT ON COLUMN"));
    }

    #[test]
    fn test_split_dollar_sign_is_not_comment() {
        let stmts = split_sql_statements("SELECT * FROM t WHERE x = 1; -- comment\nSELECT 2");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "SELECT * FROM t WHERE x = 1");
        assert_eq!(stmts[1], "SELECT 2");
    }
}
