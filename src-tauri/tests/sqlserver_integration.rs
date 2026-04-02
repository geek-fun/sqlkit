//! Integration tests for SQL Server adapter.
//!
//! These tests require a running SQL Server instance.
//! Set the following environment variables:
//! - SQLSERVER_HOST (default: localhost)
//! - SQLSERVER_PORT (default: 1433)
//! - SQLSERVER_USER (default: sa)
//! - SQLSERVER_PASSWORD
//! - SQLSERVER_DB (default: master)
//! - SQLSERVER_USE_WINDOWS_AUTH (optional, default: false)
//! - SQLSERVER_TRUST_CERT (optional, default: true for testing)
//!
//! Example:
//! ```bash
//! export SQLSERVER_HOST=localhost
//! export SQLSERVER_PORT=1433
//! export SQLSERVER_USER=sa
//! export SQLSERVER_PASSWORD=YourPassword123!
//! export SQLSERVER_DB=testdb
//! export SQLSERVER_TRUST_CERT=true
//! cargo test --test sqlserver_integration -- --nocapture
//! ```

#![cfg(test)]

use sqlkit_lib::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, PoolConfig, QueryValue, SqlServerAdapter,
    SslMode,
};
use std::env;
use std::time::Duration;

fn get_test_config() -> ConnectionConfig {
    let host = env::var("SQLSERVER_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("SQLSERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(1433);
    let username = env::var("SQLSERVER_USER").unwrap_or_else(|_| "sa".to_string());
    let password = env::var("SQLSERVER_PASSWORD").ok();
    let database = env::var("SQLSERVER_DB").unwrap_or_else(|_| "master".to_string());
    let use_windows_auth = env::var("SQLSERVER_USE_WINDOWS_AUTH")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);
    let trust_cert = env::var("SQLSERVER_TRUST_CERT")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(true);

    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 5,
        connection_timeout: Duration::from_secs(10),
        max_lifetime: Duration::from_secs(300),
        idle_timeout: Duration::from_secs(60),
    };

    let mut config = ConnectionConfig::new(DatabaseType::SqlServer, host, port, username)
        .with_database(database)
        .with_pool_config(pool_config)
        .with_ssl_mode(SslMode::Require);

    if let Some(pwd) = password {
        config = config.with_password(pwd);
    }

    if use_windows_auth {
        config = config.with_option("use_windows_auth", "true");
    }

    if trust_cert {
        config = config.with_option("trust_server_certificate", "true");
    }

    config
}

#[tokio::test]
#[ignore] // Ignored by default, requires SQL Server instance
async fn test_connection() {
    let config = get_test_config();
    let mut adapter = SqlServerAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let status = adapter
        .test_connection()
        .await
        .expect("Failed to test connection");

    assert!(status.is_connected);
    assert!(status.server_version.is_some());
    assert!(status.current_database.is_some());
    assert!(status.current_user.is_some());

    println!("Connected to SQL Server {}", status.server_version.unwrap());
    println!("Current database: {}", status.current_database.unwrap());
    println!("Current user: {}", status.current_user.unwrap());

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_simple_query() {
    let config = get_test_config();
    let mut adapter = SqlServerAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let result = adapter
        .execute_query("SELECT 1 as num, 'hello' as text, CAST(1 AS BIT) as flag")
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
    let mut adapter = SqlServerAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create a temporary table (using # prefix for SQL Server temp tables)
    adapter
        .execute_query(
            "CREATE TABLE #test_users (
                id INT IDENTITY(1,1) PRIMARY KEY,
                name NVARCHAR(100) NOT NULL,
                email NVARCHAR(255),
                age INT,
                balance DECIMAL(10, 2),
                created_at DATETIME2 DEFAULT GETDATE()
            )",
        )
        .await
        .expect("Failed to create table");

    // Insert test data
    let result = adapter
        .execute_query(
            "INSERT INTO #test_users (name, email, age, balance) VALUES 
                ('Alice', 'alice@example.com', 30, 1000.50),
                ('Bob', 'bob@example.com', 25, 750.25),
                ('Charlie', NULL, 35, 2000.00)",
        )
        .await
        .expect("Failed to insert data");

    assert!(result.rows_affected.is_some());

    // Query the data
    let result = adapter
        .execute_query("SELECT * FROM #test_users ORDER BY name")
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
    let mut adapter = SqlServerAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let databases = adapter
        .list_databases()
        .await
        .expect("Failed to list databases");

    assert!(!databases.is_empty());

    for db in databases {
        println!("Database: {} - {:?}", db.name, db.description);
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_list_schemas() {
    let config = get_test_config();
    let mut adapter = SqlServerAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let schemas = adapter
        .list_schemas(None)
        .await
        .expect("Failed to list schemas");

    assert!(!schemas.is_empty());
    assert!(schemas.contains(&"dbo".to_string()));

    // System schemas should be filtered out
    assert!(!schemas.contains(&"sys".to_string()));
    assert!(!schemas.contains(&"INFORMATION_SCHEMA".to_string()));

    for schema in schemas {
        println!("Schema: {}", schema);
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_xml_and_complex_types() {
    let config = get_test_config();
    let mut adapter = SqlServerAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create a table with XML and other complex types
    adapter
        .execute_query(
            "CREATE TABLE #test_complex (
                id INT IDENTITY(1,1) PRIMARY KEY,
                xml_data XML,
                unique_id UNIQUEIDENTIFIER,
                json_data NVARCHAR(MAX)
            )",
        )
        .await
        .expect("Failed to create table");

    // Insert data with complex types
    adapter
        .execute_query(
            r#"INSERT INTO #test_complex (xml_data, unique_id, json_data) VALUES 
                ('<root><item>test</item></root>', NEWID(), '{"name": "Alice", "age": 30}'),
                ('<root><item>another</item></root>', NEWID(), '{"name": "Bob", "age": 25}')"#,
        )
        .await
        .expect("Failed to insert data");

    // Query the data
    let result = adapter
        .execute_query("SELECT * FROM #test_complex ORDER BY id")
        .await
        .expect("Failed to query data");

    assert_eq!(result.rows.len(), 2);

    for row in &result.rows {
        // XML data should be converted to string
        match row.get("xml_data").unwrap() {
            QueryValue::String(s) => {
                println!("XML data: {}", s);
                assert!(s.contains("root"));
            }
            _ => panic!("Expected String for XML"),
        }

        // UNIQUEIDENTIFIER should be converted to string
        match row.get("unique_id").unwrap() {
            QueryValue::String(s) => {
                println!("GUID: {}", s);
                assert!(s.len() > 0);
            }
            _ => panic!("Expected String for GUID"),
        }

        // JSON (stored as NVARCHAR) should be a string
        match row.get("json_data").unwrap() {
            QueryValue::String(s) => {
                println!("JSON data: {}", s);
                assert!(s.contains("name"));
            }
            _ => panic!("Expected String for JSON"),
        }
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_query_timeout() {
    // Test that queries timeout when they exceed the configured statement_timeout
    let config = get_test_config().with_option("statement_timeout", "1000"); // 1000 milliseconds (1 second)

    let mut adapter = SqlServerAdapter::new(config);
    adapter.connect().await.expect("Failed to connect");

    // This query should timeout after 1 second (tries to wait for 5 seconds)
    let result = adapter.execute_query("WAITFOR DELAY '00:00:05'").await;

    assert!(result.is_err());
    if let Err(e) = result {
        println!("Expected timeout error: {}", e);
    }

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_multiple_result_sets() {
    let config = get_test_config();
    let mut adapter = SqlServerAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // SQL Server can return multiple result sets, but our adapter returns the first one
    let result = adapter
        .execute_query("SELECT 1 as num; SELECT 2 as num2")
        .await
        .expect("Failed to execute query");

    // Should get the first result set
    assert_eq!(result.rows.len(), 1);
    assert!(result.columns.contains(&"num".to_string()));

    adapter.disconnect().await.expect("Failed to disconnect");
}
