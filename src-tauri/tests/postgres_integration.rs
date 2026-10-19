//! Integration tests for PostgreSQL adapter.
//!
//! These tests require a running PostgreSQL instance.
//! Set the following environment variables:
//! - POSTGRES_HOST (default: localhost)
//! - POSTGRES_PORT (default: 5432)
//! - POSTGRES_USER (default: postgres)
//! - POSTGRES_PASSWORD
//! - POSTGRES_DB (default: postgres)
//!
//! Example:
//! ```bash
//! export POSTGRES_HOST=localhost
//! export POSTGRES_PORT=5432
//! export POSTGRES_USER=postgres
//! export POSTGRES_PASSWORD=password
//! export POSTGRES_DB=testdb
//! cargo test --test postgres_integration -- --nocapture
//! ```

#![cfg(test)]

use sqlkit_lib::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, PoolConfig, PostgresAdapter, QueryValue,
    SslMode,
};
use std::env;
use std::time::Duration;

const SESSION_SCRIPT: &str = include_str!("fixtures/postgres_session_script.sql");

fn script_test_config() -> ConnectionConfig {
    let mut config = get_test_config();
    config.pool_config.max_connections = 1;
    config
}

async fn prepare_session_tables(adapter: &PostgresAdapter) {
    adapter.execute_query(r#"
        CREATE TEMP TABLE sample_customers (id bigint, email text);
        CREATE TEMP TABLE sample_profiles (
            customer_id bigint, label text, state text, score bigint,
            region text, profile_type text, display_name text,
            reference_code text, contact_email text
        );
        CREATE TEMP TABLE sample_events (
            id bigint, customer_id bigint, type text, state text, unit text,
            amount bigint, source text, external_ref text,
            source_ref text, match_key text, created_at timestamptz DEFAULT now()
        );
        CREATE TEMP TABLE sample_requests (
            id bigint, customer_id bigint, category text, state text, amount bigint,
            unit text, payload jsonb, outcome text, owner_id bigint,
            created_at timestamptz DEFAULT now(), completed_at timestamptz
        );
        INSERT INTO sample_customers VALUES (1, 'o''connor测试@example.com'), (2, 'other@example.com');
        INSERT INTO sample_profiles (customer_id, label, state, score)
            VALUES (1, 'standard', 'active', 0);
        INSERT INTO sample_events (id, customer_id, type, state, unit, amount, external_ref, match_key)
            VALUES (1, 1, 'credit', 'success', 'USD', 1000, 'ref-a', NULL),
                   (2, 1, 'credit', 'success', 'USD', 2000, 'ref-b', NULL),
                   (3, 1, 'adjustment', 'success', 'USD', -300, NULL, 'request:42'),
                   (4, 1, 'excluded', 'success', 'USD', -5000, NULL, NULL),
                   (5, 1, 'credit', 'failed', 'USD', 9000, NULL, NULL),
                   (6, 1, 'adjustment', 'success', 'EUR', -100, NULL, 'request:42'),
                   (7, 2, 'credit', 'success', 'USD', 9000, NULL, NULL);
        INSERT INTO sample_requests (id, customer_id, category, state, amount, unit, payload)
            VALUES (42, 1, 'adjustment', 'completed', 300, 'USD', '{"source":"test"}'),
                   (43, 1, 'adjustment', 'pending', 100, 'USD', '{}'),
                   (44, 2, 'adjustment', 'pending', 100, 'USD', '{}');
    "#).await.expect("Failed to prepare synthetic session data");
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn test_session_script_preserves_all_results_and_transaction_parameters() {
    let mut adapter = PostgresAdapter::new(script_test_config());
    adapter.connect().await.unwrap();
    prepare_session_tables(&adapter).await;

    // Exercise both optional IDs, exact IDs, and Windows line endings.
    for (request, reference, expected_events, expected_requests, expected_matches) in
        [("", "", 3, 2, 0), ("42", "ref-a", 1, 1, 1)]
    {
        let script = SESSION_SCRIPT
            .replace("request_id', ''", &format!("request_id', '{}'", request))
            .replace(
                "external_ref', ''",
                &format!("external_ref', '{}'", reference),
            )
            .replace('\n', "\r\n");
        let result = adapter.execute_query(&script).await.unwrap();
        let sets = &result.result_sets;
        assert_eq!(sets.len(), 8);
        assert_eq!(
            sets[0].rows[0]["email"],
            QueryValue::String("o'connor测试@example.com".into())
        );
        assert_eq!(
            sets[1].rows[0]["transaction_read_only"],
            QueryValue::String("on".into())
        );
        assert!(matches!(
            sets[1].rows[0]["checked_at"],
            QueryValue::DateTime(_)
        ));
        assert_eq!(sets[2].rows.len(), 1);
        assert_eq!(sets[2].rows[0]["reference_code"], QueryValue::Null);
        assert_eq!(sets[3].rows.len(), expected_events);
        assert_eq!(sets[4].rows.len(), expected_requests);
        assert_eq!(
            sets[5].rows[0]["total_amount"],
            QueryValue::String("2700".into())
        );
        assert_eq!(sets[6].rows.len(), if request.is_empty() { 0 } else { 2 });
        assert!(!sets[6].columns.is_empty());
        assert_eq!(
            sets[7].rows[0]["matched_rows"],
            QueryValue::Int(expected_matches)
        );
        assert_eq!(
            sets[7].rows[0]["matched_amount"],
            QueryValue::String(if request.is_empty() { "0" } else { "-300" }.into())
        );
        assert_eq!(result.rows, sets[7].rows);
        assert_eq!(result.columns, sets[7].columns);

        let after = adapter.execute_query("SELECT current_setting('transaction_read_only') AS read_only, current_setting('script_test.email', true) AS email").await.unwrap();
        assert_eq!(after.rows[0]["read_only"], QueryValue::String("off".into()));
        assert_eq!(after.rows[0]["email"], QueryValue::String(String::new()));
        assert!(after.result_sets.is_empty());
    }
    adapter.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn test_script_uses_one_session_and_repeatable_read_under_concurrent_queries() {
    let mut adapter = PostgresAdapter::new(get_test_config());
    adapter.connect().await.unwrap();
    let (script, other) = tokio::join!(
        adapter.execute_query(
            "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;
             SELECT pg_backend_pid() AS pid, set_config('sqlkit_test.marker', 'same-session', true);
             SELECT pg_sleep(0.05);
             SELECT pg_backend_pid() AS pid, current_setting('sqlkit_test.marker') AS marker,
                    current_setting('transaction_isolation') AS isolation;
             COMMIT;"
        ),
        adapter.execute_query("SELECT pg_sleep(0.05)")
    );
    other.unwrap();
    let result = script.unwrap();
    assert_eq!(result.result_sets.len(), 3);
    assert_eq!(result.result_sets[0].rows[0]["pid"], result.rows[0]["pid"]);
    assert_eq!(
        result.rows[0]["marker"],
        QueryValue::String("same-session".into())
    );
    assert_eq!(
        result.rows[0]["isolation"],
        QueryValue::String("repeatable read".into())
    );
    adapter.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn test_failed_script_discards_transaction_and_stops_before_commit() {
    let mut adapter = PostgresAdapter::new(script_test_config());
    adapter.connect().await.unwrap();
    let result = adapter.execute_query("BEGIN; SELECT set_config('sqlkit_test.marker', 'pending', true); SELECT 1 / 0; COMMIT; SELECT 42").await;
    assert!(result.unwrap_err().to_string().contains("22012"));
    let after = adapter
        .execute_query("SELECT current_setting('sqlkit_test.marker', true) AS marker, 42 AS value")
        .await
        .unwrap();
    assert_eq!(after.rows[0]["marker"], QueryValue::Null);
    assert_eq!(after.rows[0]["value"], QueryValue::Int(42));
    adapter.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn test_script_timeout_discards_connection_even_when_future_is_dropped() {
    let mut adapter = PostgresAdapter::new(script_test_config());
    adapter.connect().await.unwrap();
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        adapter.execute_query("BEGIN; SELECT set_config('sqlkit_test.marker', 'pending', true); SELECT pg_sleep(10); COMMIT;"),
    ).await;
    assert!(result.is_err());
    let after = tokio::time::timeout(
        Duration::from_secs(2),
        adapter.execute_query("SELECT current_setting('sqlkit_test.marker', true) AS marker"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(after.rows[0]["marker"], QueryValue::Null);
    adapter.disconnect().await.unwrap();
}

fn get_test_config() -> ConnectionConfig {
    let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("POSTGRES_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432);
    let username = env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
    let password = env::var("POSTGRES_PASSWORD").ok();
    let database = env::var("POSTGRES_DB").unwrap_or_else(|_| "postgres".to_string());

    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 5,
        connection_timeout: Duration::from_secs(10),
        max_lifetime: Duration::from_secs(300),
        idle_timeout: Duration::from_secs(60),
    };

    let mut config = ConnectionConfig::new(DatabaseType::PostgreSQL, host, port, username)
        .with_database(database)
        .with_pool_config(pool_config)
        .with_ssl_mode(SslMode::Prefer)
        .with_option("application_name", "sqlkit_test");

    if let Some(pwd) = password {
        config = config.with_password(pwd);
    }

    config
}

#[tokio::test]
#[ignore] // Ignored by default, requires PostgreSQL instance
async fn test_connection() {
    let config = get_test_config();
    let mut adapter = PostgresAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let status = adapter
        .test_connection()
        .await
        .expect("Failed to test connection");

    assert!(status.is_connected);
    assert!(status.server_version.is_some());
    assert!(status.current_database.is_some());
    assert!(status.current_user.is_some());

    println!("Connected to PostgreSQL {}", status.server_version.unwrap());
    println!("Current database: {}", status.current_database.unwrap());
    println!("Current user: {}", status.current_user.unwrap());

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_simple_query() {
    let config = get_test_config();
    let mut adapter = PostgresAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let result = adapter
        .execute_query("SELECT 1 as num, 'hello' as text, true as flag")
        .await
        .expect("Failed to execute query");

    assert_eq!(result.columns.len(), 3);
    assert_eq!(result.columns[0], "num");
    assert_eq!(result.columns[1], "text");
    assert_eq!(result.columns[2], "flag");

    assert_eq!(result.rows.len(), 1);
    let row = &result.rows[0];

    match row.get("num").unwrap() {
        QueryValue::Int(n) => assert_eq!(*n, 1),
        _ => panic!("Expected Int"),
    }

    match row.get("text").unwrap() {
        QueryValue::String(s) => assert_eq!(s, "hello"),
        _ => panic!("Expected String"),
    }

    match row.get("flag").unwrap() {
        QueryValue::Bool(b) => assert!(*b),
        _ => panic!("Expected Bool"),
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_create_and_query_table() {
    let config = get_test_config();
    let mut adapter = PostgresAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create a test table
    adapter
        .execute_query(
            "CREATE TEMPORARY TABLE test_users (
                id SERIAL PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                email VARCHAR(255),
                age INTEGER,
                balance NUMERIC(10, 2),
                created_at TIMESTAMP DEFAULT NOW()
            )",
        )
        .await
        .expect("Failed to create table");

    // Insert test data
    let result = adapter
        .execute_query(
            "INSERT INTO test_users (name, email, age, balance) VALUES 
                ('Alice', 'alice@example.com', 30, 1000.50),
                ('Bob', 'bob@example.com', 25, 750.25),
                ('Charlie', NULL, 35, 2000.00)",
        )
        .await
        .expect("Failed to insert data");

    assert_eq!(result.rows_affected, Some(3));

    // Query the data
    let result = adapter
        .execute_query("SELECT * FROM test_users ORDER BY name")
        .await
        .expect("Failed to query data");

    assert_eq!(result.rows.len(), 3);
    assert!(result.columns.contains(&"name".to_string()));
    assert!(result.columns.contains(&"email".to_string()));

    // Check first row
    let alice = &result.rows[0];
    match alice.get("name").unwrap() {
        QueryValue::String(s) => assert_eq!(s, "Alice"),
        _ => panic!("Expected String"),
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_list_databases() {
    let config = get_test_config();
    let mut adapter = PostgresAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let databases = adapter
        .list_databases()
        .await
        .expect("Failed to list databases");

    assert!(!databases.is_empty());
    assert!(databases.iter().any(|db| db.name == "postgres"));

    for db in databases {
        println!("Database: {} - {:?}", db.name, db.description);
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_list_schemas() {
    let config = get_test_config();
    let mut adapter = PostgresAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let schemas = adapter
        .list_schemas(None)
        .await
        .expect("Failed to list schemas");

    assert!(!schemas.is_empty());
    assert!(schemas.contains(&"public".to_string()));

    // System schemas should be filtered out
    assert!(!schemas.contains(&"pg_catalog".to_string()));
    assert!(!schemas.contains(&"information_schema".to_string()));

    for schema in schemas {
        println!("Schema: {}", schema);
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_json_types() {
    let config = get_test_config();
    let mut adapter = PostgresAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create a table with JSON columns
    adapter
        .execute_query(
            "CREATE TEMPORARY TABLE test_json (
                id SERIAL PRIMARY KEY,
                data JSON,
                metadata JSONB
            )",
        )
        .await
        .expect("Failed to create table");

    // Insert JSON data
    adapter
        .execute_query(
            r#"INSERT INTO test_json (data, metadata) VALUES 
                ('{"name": "Alice", "age": 30}', '{"role": "admin", "active": true}'),
                ('{"name": "Bob", "age": 25}', '{"role": "user", "active": false}')"#,
        )
        .await
        .expect("Failed to insert data");

    // Query JSON data
    let result = adapter
        .execute_query("SELECT * FROM test_json ORDER BY id")
        .await
        .expect("Failed to query data");

    assert_eq!(result.rows.len(), 2);

    for row in &result.rows {
        match row.get("data").unwrap() {
            QueryValue::String(s) => {
                println!("JSON data: {}", s);
                assert!(s.contains("name"));
            }
            _ => panic!("Expected String for JSON"),
        }

        match row.get("metadata").unwrap() {
            QueryValue::String(s) => {
                println!("JSONB metadata: {}", s);
                assert!(s.contains("role"));
            }
            _ => panic!("Expected String for JSONB"),
        }
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_query_timeout() {
    // Test that queries timeout when they exceed the configured statement_timeout
    let config = script_test_config().with_option("statement_timeout", "1000");

    let mut adapter = PostgresAdapter::new(config);
    adapter.connect().await.expect("Failed to connect");

    // This query should timeout after 1 second (tries to sleep for 5 seconds)
    let result = adapter
        .execute_query("BEGIN; SELECT set_config('sqlkit_test.marker', 'pending', true); SELECT pg_sleep(5); COMMIT;")
        .await;

    assert!(matches!(
        result,
        Err(sqlkit_lib::database::DbError::Timeout(_))
    ));
    let after = adapter
        .execute_query("SELECT current_setting('sqlkit_test.marker', true) AS marker")
        .await
        .unwrap();
    assert_eq!(after.rows[0]["marker"], QueryValue::Null);

    adapter.disconnect().await.expect("Failed to disconnect");
}
