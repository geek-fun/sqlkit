//! Live coverage for docs/sql-test; load 01_schema + 02_seed before running.
//! See docs/sql-test/AUTOMATED_TESTING.md for setup and connection settings.
use sqlkit_lib::database::config::OracleConnectionOptions;
use sqlkit_lib::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, JdbcBridgeAdapter, MySQLAdapter, PoolConfig,
    PostgresAdapter, SqlServerAdapter, SslMode,
};
use sqlparser::{
    dialect::PostgreSqlDialect,
    tokenizer::{Token, Tokenizer},
};
use std::{env, path::PathBuf};

fn config(
    kind: DatabaseType,
    prefix: &str,
    port: u16,
    user: &str,
    password: &str,
    db: &str,
) -> ConnectionConfig {
    ConnectionConfig::new(
        kind,
        env::var(format!("{prefix}_HOST")).unwrap_or_else(|_| "127.0.0.1".into()),
        env::var(format!("{prefix}_PORT"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(port),
        env::var(format!("{prefix}_USER")).unwrap_or_else(|_| user.into()),
    )
    .with_password(env::var(format!("{prefix}_PASSWORD")).unwrap_or_else(|_| password.into()))
    .with_database(env::var(format!("{prefix}_DB")).unwrap_or_else(|_| db.into()))
    .with_ssl_mode(SslMode::Disable)
    .with_pool_config(PoolConfig {
        min_connections: 1,
        max_connections: 1,
        ..Default::default()
    })
}

fn fixture(engine: &str, file: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../docs/sql-test")
            .join(engine)
            .join(file),
    )
    .unwrap()
}

fn sections(sql: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut title = String::new();
    let mut body = String::new();
    for line in sql.lines() {
        if line
            .strip_prefix("-- ── ")
            .and_then(|s| s.chars().next())
            .is_some_and(|c| c.is_ascii_uppercase())
        {
            if !title.is_empty() {
                sections.push((title, body));
            }
            title = line.to_string();
            body = String::new();
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    if !title.is_empty() {
        sections.push((title, body));
    }
    sections
}

fn pg_statements(sql: &str) -> Vec<String> {
    let tokens = Tokenizer::new(&PostgreSqlDialect {}, sql)
        .with_unescape(false)
        .tokenize()
        .unwrap();
    tokens
        .split(|t| *t == Token::SemiColon)
        .map(|tokens| tokens.iter().map(ToString::to_string).collect::<String>())
        .filter(|s| {
            s.lines()
                .any(|l| !l.trim().is_empty() && !l.trim().starts_with("--"))
        })
        .collect()
}

fn mysql_statements(sql: &str) -> Vec<String> {
    let mut delimiter = ";";
    let mut pending = String::new();
    let mut result = Vec::new();
    for line in sql.lines() {
        if let Some(value) = line.strip_prefix("DELIMITER ") {
            delimiter = value.trim();
            continue;
        }
        pending.push_str(line);
        pending.push('\n');
        if pending.trim_end().ends_with(delimiter) {
            result.push(
                pending
                    .trim_end()
                    .strip_suffix(delimiter)
                    .unwrap()
                    .to_string(),
            );
            pending.clear();
        }
    }
    if pending
        .lines()
        .any(|l| !l.trim().is_empty() && !l.trim().starts_with("--"))
    {
        result.push(pending);
    }
    result
}

async fn metadata<A: DatabaseAdapter>(adapter: &A, schema: &str, failures: &mut Vec<String>) {
    macro_rules! check {
        ($label:expr, $call:expr) => {
            match $call.await {
                Ok(_) => {
                    println!("{}: ok", $label);
                }
                Err(error) => failures.push(format!("{}: {}", $label, error)),
            }
        };
    }
    check!("schemas", adapter.list_schemas(None));
    check!("views", adapter.list_views(None, Some(schema)));
    check!("procedures", adapter.list_procedures(None, Some(schema)));
    check!("functions", adapter.list_functions(None, Some(schema)));
    let tables = adapter
        .list_tables(None, Some(schema))
        .await
        .expect("list tables");
    assert!(tables.iter().any(|t| t.name.eq_ignore_ascii_case("users")));
    for table in tables
        .iter()
        .filter(|t| t.table_type == "TABLE" || t.table_type == "BASE TABLE")
    {
        let name = &table.name;
        let columns = adapter
            .list_columns(None, Some(schema), name)
            .await
            .unwrap();
        let unique: std::collections::HashSet<_> = columns.iter().map(|c| &c.name).collect();
        assert_eq!(
            unique.len(),
            columns.len(),
            "duplicate columns in {schema}.{name}"
        );
        check!(
            format!("columns {name}"),
            adapter.list_columns(None, Some(schema), name)
        );
        check!(
            format!("info {name}"),
            adapter.get_table_info(None, Some(schema), name)
        );
        check!(
            format!("indexes {name}"),
            adapter.list_indexes(None, Some(schema), name)
        );
        check!(
            format!("foreign keys {name}"),
            adapter.list_foreign_keys(None, Some(schema), name)
        );
        check!(
            format!("triggers {name}"),
            adapter.list_triggers(None, Some(schema), name)
        );
    }
    for (name, kind) in [
        ("users", "TABLE"),
        ("orders", "TABLE"),
        ("active_products", "VIEW"),
    ] {
        let name = if schema == "sqlkit-demo" && name == "active_products" {
            "v_active_products"
        } else {
            name
        };
        let name = if schema == "SQLKIT" {
            name.to_uppercase()
        } else {
            name.to_string()
        };
        if kind == "TABLE" && schema != "SQLKIT" {
            let columns = adapter
                .list_columns(None, Some(schema), &name)
                .await
                .unwrap();
            let options = sqlkit_lib::transfer::types::DdlOptions {
                include_create_table: true,
                include_primary_keys: true,
                include_foreign_keys: true,
                include_indexes: true,
                include_constraints: true,
                include_comments: true,
                include_storage_options: false,
                include_drop_if_exists: false,
                include_if_not_exists: false,
                include_data: false,
                target_engine: None,
            };
            let kind = match schema {
                "store" => DatabaseType::PostgreSQL,
                "sqlkit" => DatabaseType::SqlServer,
                _ => DatabaseType::MySQL,
            };
            let ddl = sqlkit_lib::transfer::ddl::generate_ddl_for_engine(
                kind,
                Some(schema),
                &name,
                &columns,
                &options,
            );
            assert!(ddl.contains("CREATE TABLE"));
            continue;
        }
        match adapter
            .get_object_ddl(None, Some(schema), &name, kind)
            .await
        {
            Ok(ddl) if ddl.trim().len() > 20 => {}
            value => failures.push(format!("DDL {name}: {value:?}")),
        }
    }
}

async fn probes<A: DatabaseAdapter>(adapter: &A, engine: &str, schema: &str) {
    use sqlkit_lib::database::QueryValue;
    let quoted = if engine == "mysql" {
        format!("`{schema}`")
    } else {
        format!("\"{schema}\"")
    };
    let table = if engine == "oracle" { "USERS" } else { "users" };
    for (name, expected) in [
        ("users", 400),
        ("products", 60),
        ("orders", 500),
        ("order_items", 1250),
    ] {
        let name = if engine == "oracle" {
            name.to_uppercase()
        } else {
            name.into()
        };
        let counts = adapter
            .execute_query(&format!("SELECT COUNT(*) AS n FROM {quoted}.{name}"))
            .await
            .unwrap();
        let count = match counts.rows[0].values().next().unwrap() {
            QueryValue::Int(n) => *n,
            QueryValue::String(s) => s.parse().unwrap(),
            other => panic!("unexpected count: {other:?}"),
        };
        assert_eq!(count, expected, "{engine} {name} seed count");
    }
    let columns = adapter
        .list_columns(None, Some(schema), table)
        .await
        .unwrap();
    let balance = columns
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case("balance"))
        .unwrap();
    assert_eq!(balance.precision, Some(12));
    assert_eq!(balance.scale, Some(2));
    let empty = adapter
        .execute_query(&format!("SELECT * FROM {quoted}.{table} WHERE 1 = 0"))
        .await
        .unwrap();
    assert!(empty.rows.is_empty());
    assert!(empty.columns.len() > 10);
    let all = adapter
        .execute_query(&format!("SELECT * FROM {quoted}.{table}"))
        .await
        .unwrap();
    assert_eq!(all.rows.len(), 400);
    if engine == "mysql" {
        adapter
            .execute_query("SET @sqlkit_session_probe = 17")
            .await
            .unwrap();
        adapter.list_tables(None, Some(schema)).await.unwrap();
        let session = adapter
            .execute_query("SELECT CAST(@sqlkit_session_probe AS SIGNED) AS n")
            .await
            .unwrap();
        assert_eq!(session.rows[0]["n"], QueryValue::Int(17));
    }
    let scratch = if engine == "oracle" {
        "SQLKIT_STRESS_PROBE"
    } else {
        "sqlkit_stress_probe"
    };
    let renamed = if engine == "oracle" {
        "SQLKIT_STRESS_RENAMED"
    } else {
        "sqlkit_stress_renamed"
    };
    adapter
        .execute_query(&format!("CREATE TABLE {quoted}.{scratch} (id INT)"))
        .await
        .unwrap();
    if engine == "sqlserver" {
        adapter.execute_query("SET NOCOUNT OFF").await.unwrap();
        let insert = adapter
            .execute_query(&format!("INSERT INTO {quoted}.{scratch} VALUES (1), (2)"))
            .await
            .unwrap();
        assert_eq!(insert.rows_affected, Some(2));
        let batch = adapter
            .execute_query("SELECT 1 AS probe_first; SELECT 2 AS probe_last")
            .await
            .unwrap();
        assert_eq!(batch.rows[0]["probe_last"], QueryValue::Int(2));
        let empty = adapter
            .execute_query("SELECT 1 AS probe_first; SELECT 2 AS probe_last WHERE 1 = 0")
            .await
            .unwrap();
        assert_eq!(empty.columns, ["probe_last"]);
        assert!(empty.rows.is_empty());
        adapter.execute_query("SET NOCOUNT ON").await.unwrap();
    }
    adapter
        .rename_object(None, Some(schema), scratch, "TABLE", renamed)
        .await
        .unwrap();
    adapter
        .drop_object(None, Some(schema), renamed, "TABLE")
        .await
        .unwrap();
    if engine == "postgres" {
        let columns = adapter
            .list_columns(None, Some(schema), "users")
            .await
            .unwrap();
        assert!(
            columns
                .iter()
                .find(|c| c.name == "id")
                .unwrap()
                .is_auto_increment
        );
        let indexes = adapter
            .list_indexes(None, Some(schema), "users")
            .await
            .unwrap();
        assert!(indexes
            .iter()
            .any(|i| i.is_primary && i.is_unique && i.columns == ["id"]));
        assert!(indexes.iter().any(|i| i.index_type == "GIN"));
        assert!(indexes.iter().any(|i| i.index_type == "BRIN"));
        let fks = adapter
            .list_foreign_keys(None, Some(schema), "order_items")
            .await
            .unwrap();
        let fk = fks
            .iter()
            .find(|fk| fk.constraint_name == "fk_items_order")
            .unwrap();
        assert_eq!(fk.columns, ["order_id", "order_date"]);
        assert_eq!(fk.referenced_columns, ["id", "order_date"]);
        assert_eq!(fk.referenced_table, "orders");
        let values = adapter.execute_query("SELECT signup_ip AS ip, host(signup_ip) AS expected_ip, wallet AS wallet, wallet::text AS expected_wallet, geo AS geo, geo::text AS expected_geo, true AS flag, 7 AS n, NULL::inet AS missing FROM store.users ORDER BY id LIMIT 1").await.unwrap();
        let row = &values.rows[0];
        for key in ["ip", "wallet", "geo"] {
            assert_eq!(row[key], row[&format!("expected_{key}")]);
        }
        assert_eq!(row["flag"], QueryValue::Bool(true));
        assert_eq!(row["n"], QueryValue::Int(7));
        assert_eq!(row["missing"], QueryValue::Null);
        let large = adapter.execute_query("SELECT 1234567890123456789012345678901234567890.123456789::numeric AS n, ARRAY['10.0.0.1'::inet, NULL] AS ips").await.unwrap();
        assert_eq!(
            large.rows[0]["n"],
            QueryValue::String("1234567890123456789012345678901234567890.123456789".into())
        );
        assert_eq!(
            large.rows[0]["ips"],
            QueryValue::String("{10.0.0.1,NULL}".into())
        );
        let batch = adapter.execute_query("BEGIN; CREATE TEMP TABLE sqlkit_batch_probe (n int) ON COMMIT DROP; INSERT INTO sqlkit_batch_probe VALUES (42); SELECT n FROM sqlkit_batch_probe; COMMIT;").await.unwrap();
        assert_eq!(batch.rows[0]["n"], QueryValue::Int(42));
    }
    if engine == "mysql" {
        let typed = adapter.execute_query("SELECT 42 AS n, 1.25e0 AS f, CAST(18446744073709551615 AS UNSIGNED) AS big, CAST('abc' AS BINARY) AS raw").await.unwrap();
        assert_eq!(typed.rows[0]["n"], QueryValue::Int(42));
        assert_eq!(typed.rows[0]["f"], QueryValue::Float(1.25));
        assert_eq!(
            typed.rows[0]["big"],
            QueryValue::String("18446744073709551615".into())
        );
        assert_eq!(typed.rows[0]["raw"], QueryValue::Bytes(b"abc".to_vec()));
        let dml = adapter.execute_query("CREATE TEMPORARY TABLE sqlkit_dml_probe (n INT); INSERT INTO sqlkit_dml_probe VALUES (1), (2); UPDATE sqlkit_dml_probe SET n = n + 1; DROP TEMPORARY TABLE sqlkit_dml_probe;").await.unwrap();
        assert_eq!(dml.rows_affected, Some(4));
        let indexes = adapter
            .list_indexes(None, Some(schema), "users")
            .await
            .unwrap();
        assert!(!indexes
            .iter()
            .find(|i| i.name == "idx_users_city")
            .unwrap()
            .columns[0]
            .is_empty());
        let batch = adapter
            .execute_query("SET @sqlkit_probe = 42; SELECT CAST(@sqlkit_probe AS SIGNED) AS n;")
            .await
            .unwrap();
        assert_eq!(batch.rows[0]["n"], QueryValue::Int(42));
        let batch = adapter
            .execute_query("SELECT 1 AS probe_first; SELECT 2 AS probe_last;")
            .await
            .unwrap();
        assert_eq!(batch.rows[0]["probe_last"], QueryValue::Int(2));
        assert!(adapter
            .execute_query("SELECT 1; SELECT * FROM sqlkit_missing_table_probe")
            .await
            .is_err());
    }
}

async fn run<A: DatabaseAdapter>(mut adapter: A, engine: &str, schema: &str) {
    adapter.connect().await.expect("connect live database");
    if env::var_os("SQLKIT_STRESS_RESET").is_some() {
        for file in ["01_schema.sql", "02_seed.sql"] {
            let source = fixture(engine, file);
            let batches = match engine {
                "mysql" => mysql_statements(&source),
                "sqlserver" => source.split("\nGO").map(str::to_string).collect(),
                _ => vec![source],
            };
            for (i, batch) in batches.iter().enumerate() {
                adapter
                    .execute_query(batch)
                    .await
                    .unwrap_or_else(|e| panic!("{engine} {file} batch {i}: {e}"));
            }
        }
    }
    let mut failures = Vec::new();
    metadata(&adapter, schema, &mut failures).await;
    probes(&adapter, engine, schema).await;
    let sql = fixture(engine, "03_complex_queries.sql");
    let sections = sections(&sql);
    assert!(sections.len() >= 20, "all A..T sections must be exercised");
    for (title, body) in sections {
        let statements = match engine {
            "postgres" if title.starts_with("-- ── I.") => pg_statements(&body),
            "mysql" if body.contains("DELIMITER ") => mysql_statements(&body),
            _ => vec![body],
        };
        for (i, statement) in statements.iter().enumerate() {
            match adapter.execute_query(statement).await {
                Ok(r) => println!(
                    "{title} [{i}]: {} rows, {} columns, {:?} affected",
                    r.rows.len(),
                    r.columns.len(),
                    r.rows_affected
                ),
                Err(e)
                    if engine == "postgres"
                        && title.starts_with("-- ── I.")
                        && e.to_string().contains("23P01") =>
                {
                    println!("expected exclusion violation: {e}")
                }
                Err(e)
                    if engine == "sqlserver"
                        && title.starts_with("-- ── I.")
                        && e.to_string().contains("overlap") =>
                {
                    println!("expected overlap violation: {e}");
                    adapter
                        .execute_query("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION")
                        .await
                        .unwrap();
                }
                Err(e) => failures.push(format!("{title} [{i}]: {e}")),
            }
        }
    }
    adapter.disconnect().await.unwrap();
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[tokio::test]
#[ignore]
async fn postgres_stress_suite() {
    run(
        PostgresAdapter::new(config(
            DatabaseType::PostgreSQL,
            "POSTGRES",
            25432,
            "tester",
            "testpass",
            "testdb",
        )),
        "postgres",
        "store",
    )
    .await;
}
#[tokio::test]
#[ignore]
async fn mysql_stress_suite() {
    let c = config(
        DatabaseType::MySQL,
        "MYSQL",
        23306,
        "root",
        "testpass",
        "sqlkit-demo",
    );
    if env::var_os("SQLKIT_STRESS_RESET").is_some() {
        let mut bootstrap_config = c.clone();
        bootstrap_config.database = None;
        let mut bootstrap = MySQLAdapter::new(bootstrap_config);
        bootstrap.connect().await.unwrap();
        bootstrap
            .execute_query("CREATE DATABASE IF NOT EXISTS `sqlkit-demo`")
            .await
            .unwrap();
        bootstrap.disconnect().await.unwrap();
    }
    run(MySQLAdapter::new(c), "mysql", "sqlkit-demo").await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_stress_suite() {
    let mut c = config(
        DatabaseType::SqlServer,
        "SQLSERVER",
        21433,
        "sa",
        "TestPass!2024",
        "testdb",
    );
    c.trust_server_certificate = true;
    run(SqlServerAdapter::new(c), "sqlserver", "sqlkit").await;
}
#[tokio::test]
#[ignore]
async fn oracle_stress_suite() {
    let c = config(
        DatabaseType::Oracle,
        "ORACLE",
        21521,
        "sqlkit",
        "testpass",
        "FREEPDB1",
    )
    .with_oracle_options(OracleConnectionOptions {
        connection_method: "basic".into(),
        sid_or_service: Some("service_name".into()),
        role: None,
        tns_admin_dir: None,
        tns_alias: None,
        wallet_password: None,
        service_level: None,
    });
    run(JdbcBridgeAdapter::new(c), "oracle", "SQLKIT").await;
}
