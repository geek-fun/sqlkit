//! Integration tests for MySQL adapter.
//!
//! These tests require a running MySQL instance.
//! Set the following environment variables:
//! - MYSQL_HOST (default: localhost)
//! - MYSQL_PORT (default: 3306)
//! - MYSQL_USER (default: root)
//! - MYSQL_PASSWORD
//! - MYSQL_DB (default: mysql)
//!
//! Example:
//! ```bash
//! export MYSQL_HOST=localhost
//! export MYSQL_PORT=3306
//! export MYSQL_USER=root
//! export MYSQL_PASSWORD=password
//! export MYSQL_DB=testdb
//! cargo test --test mysql_integration -- --nocapture
//! ```

#![cfg(test)]

use sqlkit_lib::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, MySQLAdapter, PoolConfig, QueryValue, SslMode,
};
use std::env;
use std::time::Duration;

fn get_test_config() -> ConnectionConfig {
    let host = env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("MYSQL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3306);
    let username = env::var("MYSQL_USER").unwrap_or_else(|_| "root".to_string());
    let password = env::var("MYSQL_PASSWORD").ok();
    let database = env::var("MYSQL_DB").unwrap_or_else(|_| "mysql".to_string());

    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 5,
        connection_timeout: Duration::from_secs(10),
        max_lifetime: Duration::from_secs(300),
        idle_timeout: Duration::from_secs(60),
    };

    let mut config = ConnectionConfig::new(DatabaseType::MySQL, host, port, username)
        .with_database(database)
        .with_pool_config(pool_config)
        .with_ssl_mode(SslMode::Prefer);

    if let Some(pwd) = password {
        config = config.with_password(pwd);
    }

    config
}

#[tokio::test]
#[ignore] // Ignored by default, requires MySQL instance
async fn test_connection() {
    let config = get_test_config();
    let mut adapter = MySQLAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let status = adapter
        .test_connection()
        .await
        .expect("Failed to test connection");

    assert!(status.is_connected);
    assert!(status.server_version.is_some());
    assert!(status.current_user.is_some());

    println!("Connected to MySQL {}", status.server_version.unwrap());
    println!("Current database: {:?}", status.current_database);
    println!("Current user: {}", status.current_user.unwrap());

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_simple_query() {
    let config = get_test_config();
    let mut adapter = MySQLAdapter::new(config);

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

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_create_and_query_table() {
    let config = get_test_config();
    let mut adapter = MySQLAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create a test table
    adapter
        .execute_query(
            "CREATE TEMPORARY TABLE test_users (
                id INT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                email VARCHAR(255),
                age INT,
                balance DECIMAL(10, 2),
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
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
    let mut adapter = MySQLAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let databases = adapter
        .list_databases()
        .await
        .expect("Failed to list databases");

    assert!(!databases.is_empty());
    // MySQL should have at least the 'mysql' system database
    assert!(databases.iter().any(|db| db.name == "mysql"
        || db.name == "information_schema"
        || db.name == "performance_schema"));

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_transactions() {
    let config = get_test_config();
    let mut adapter = MySQLAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create a test table
    adapter
        .execute_query(
            "CREATE TEMPORARY TABLE test_transactions (
                id INT AUTO_INCREMENT PRIMARY KEY,
                value INT
            )",
        )
        .await
        .expect("Failed to create table");

    // Test transaction commit
    adapter
        .execute_query("START TRANSACTION")
        .await
        .expect("Failed to start transaction");

    adapter
        .execute_query("INSERT INTO test_transactions (value) VALUES (100)")
        .await
        .expect("Failed to insert");

    adapter
        .execute_query("COMMIT")
        .await
        .expect("Failed to commit");

    let result = adapter
        .execute_query("SELECT COUNT(*) as count FROM test_transactions")
        .await
        .expect("Failed to query");

    assert_eq!(result.rows.len(), 1);

    // Test transaction rollback
    adapter
        .execute_query("START TRANSACTION")
        .await
        .expect("Failed to start transaction");

    adapter
        .execute_query("INSERT INTO test_transactions (value) VALUES (200)")
        .await
        .expect("Failed to insert");

    adapter
        .execute_query("ROLLBACK")
        .await
        .expect("Failed to rollback");

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_blob_and_text_types() {
    let config = get_test_config();
    let mut adapter = MySQLAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create a table with BLOB and TEXT columns
    adapter
        .execute_query(
            "CREATE TEMPORARY TABLE test_blob_text (
                id INT AUTO_INCREMENT PRIMARY KEY,
                text_data TEXT,
                blob_data BLOB
            )",
        )
        .await
        .expect("Failed to create table");

    // Insert test data
    adapter
        .execute_query(
            r#"INSERT INTO test_blob_text (text_data, blob_data) 
               VALUES ('Sample text', 'Binary data')"#,
        )
        .await
        .expect("Failed to insert data");

    // Query the data
    let result = adapter
        .execute_query("SELECT * FROM test_blob_text")
        .await
        .expect("Failed to query data");

    assert_eq!(result.rows.len(), 1);

    let row = &result.rows[0];
    match row.get("text_data").unwrap() {
        QueryValue::String(s) => assert_eq!(s, "Sample text"),
        _ => panic!("Expected String for TEXT"),
    }

    // BLOB data might be returned as String or Bytes
    match row.get("blob_data").unwrap() {
        QueryValue::String(s) => assert_eq!(s, "Binary data"),
        QueryValue::Bytes(b) => assert_eq!(b, b"Binary data"),
        _ => panic!("Expected String or Bytes for BLOB"),
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_query_timeout() {
    // Test that queries timeout when they exceed the configured statement_timeout
    let config = get_test_config().with_option("statement_timeout", "1000"); // 1000 milliseconds

    let mut adapter = MySQLAdapter::new(config);
    adapter.connect().await.expect("Failed to connect");

    // This query should timeout after 1 second (tries to sleep for 5 seconds)
    let result = adapter.execute_query("SELECT SLEEP(5)").await;

    assert!(result.is_err());
    if let Err(e) = result {
        println!("Expected timeout error: {}", e);
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}
