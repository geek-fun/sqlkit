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
    ConnectionConfig, DatabaseAdapter, DatabaseType, PostgresAdapter, PoolConfig, QueryValue,
    SslMode,
};
use std::env;
use std::time::Duration;

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
    let config = get_test_config().with_option("statement_timeout", "1000"); // 1000 milliseconds (1 second)

    let mut adapter = PostgresAdapter::new(config);
    adapter.connect().await.expect("Failed to connect");

    // This query should timeout
    let result = adapter.execute_query("SELECT pg_sleep(5)").await;

    assert!(result.is_err());
    if let Err(e) = result {
        println!("Expected timeout error: {}", e);
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}
