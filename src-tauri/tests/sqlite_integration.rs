//! Integration tests for SQLite adapter.
//!
//! These tests cover file-based and in-memory SQLite databases,
//! including concurrency and WAL mode functionality.

#![cfg(test)]

use sqlkit_lib::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, PoolConfig, QueryValue, SQLiteAdapter, SslMode,
};
use std::fs;
use std::time::Duration;
use tokio::task;

fn get_test_config_file(db_name: &str) -> ConnectionConfig {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("{}.db", db_name));
    let db_path_str = db_path.to_string_lossy().to_string();

    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 5,
        connection_timeout: Duration::from_secs(10),
        max_lifetime: Duration::from_secs(300),
        idle_timeout: Duration::from_secs(60),
    };

    ConnectionConfig::new(DatabaseType::SQLite, "localhost", 0, "local")
        .with_database(db_path_str)
        .with_pool_config(pool_config)
        .with_ssl_mode(SslMode::Disable)
}

fn get_test_config_memory() -> ConnectionConfig {
    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 5,
        connection_timeout: Duration::from_secs(10),
        max_lifetime: Duration::from_secs(300),
        idle_timeout: Duration::from_secs(60),
    };

    ConnectionConfig::new(DatabaseType::SQLite, "localhost", 0, "local")
        .with_database(":memory:")
        .with_pool_config(pool_config)
        .with_ssl_mode(SslMode::Disable)
}

fn cleanup_db_file(db_name: &str) {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("{}.db", db_name));
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("db-wal"));
    let _ = fs::remove_file(db_path.with_extension("db-shm"));
}

#[tokio::test]
async fn test_connection_file_based() {
    let db_name = "test_connection";
    cleanup_db_file(db_name);

    let config = get_test_config_file(db_name);
    let mut adapter = SQLiteAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let status = adapter
        .test_connection()
        .await
        .expect("Failed to test connection");

    assert!(status.is_connected);
    assert!(status.server_version.is_some());
    assert!(status.current_database.is_some());
    assert_eq!(status.current_user, Some("local".to_string()));

    println!("Connected to SQLite {}", status.server_version.unwrap());
    println!("Database: {}", status.current_database.unwrap());

    adapter.disconnect().await.expect("Failed to disconnect");
    cleanup_db_file(db_name);
}

#[tokio::test]
async fn test_connection_in_memory() {
    let config = get_test_config_memory();
    let mut adapter = SQLiteAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    let status = adapter
        .test_connection()
        .await
        .expect("Failed to test connection");

    assert!(status.is_connected);
    assert!(status.server_version.is_some());
    assert_eq!(status.current_database, Some(":memory:".to_string()));

    adapter.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
async fn test_create_and_query_table() {
    let db_name = "test_create_query";
    cleanup_db_file(db_name);

    let config = get_test_config_file(db_name);
    let mut adapter = SQLiteAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create table
    let create_result = adapter
        .execute_query(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                age INTEGER,
                active INTEGER DEFAULT 1
            )",
        )
        .await
        .expect("Failed to create table");

    assert_eq!(create_result.rows_affected, Some(0));

    // Insert data
    let insert_result = adapter
        .execute_query("INSERT INTO users (name, age) VALUES ('Alice', 30), ('Bob', 25)")
        .await
        .expect("Failed to insert data");

    assert_eq!(insert_result.rows_affected, Some(2));

    // Query data
    let result = adapter
        .execute_query("SELECT id, name, age, active FROM users ORDER BY name")
        .await
        .expect("Failed to query data");

    assert_eq!(result.columns.len(), 4);
    assert_eq!(result.rows.len(), 2);

    // Check first row (Alice)
    let alice = &result.rows[0];
    assert!(matches!(alice.get("id"), Some(QueryValue::Int(_))));
    assert_eq!(
        alice.get("name"),
        Some(&QueryValue::String("Alice".to_string()))
    );
    assert_eq!(alice.get("age"), Some(&QueryValue::Int(30)));
    assert_eq!(alice.get("active"), Some(&QueryValue::Int(1)));

    // Check second row (Bob)
    let bob = &result.rows[1];
    assert_eq!(
        bob.get("name"),
        Some(&QueryValue::String("Bob".to_string()))
    );
    assert_eq!(bob.get("age"), Some(&QueryValue::Int(25)));

    adapter.disconnect().await.expect("Failed to disconnect");
    cleanup_db_file(db_name);
}

#[tokio::test]
async fn test_list_tables() {
    let db_name = "test_list_tables";
    cleanup_db_file(db_name);

    let config = get_test_config_file(db_name);
    let mut adapter = SQLiteAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create multiple tables
    adapter
        .execute_query("CREATE TABLE table1 (id INTEGER PRIMARY KEY)")
        .await
        .expect("Failed to create table1");

    adapter
        .execute_query("CREATE TABLE table2 (id INTEGER PRIMARY KEY)")
        .await
        .expect("Failed to create table2");

    adapter
        .execute_query("CREATE VIEW view1 AS SELECT * FROM table1")
        .await
        .expect("Failed to create view");

    // List tables
    let tables = adapter
        .list_tables(None, None)
        .await
        .expect("Failed to list tables");

    assert_eq!(tables.len(), 3);

    let table_names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    assert!(table_names.contains(&"table1"));
    assert!(table_names.contains(&"table2"));
    assert!(table_names.contains(&"view1"));

    adapter.disconnect().await.expect("Failed to disconnect");
    cleanup_db_file(db_name);
}

#[tokio::test]
async fn test_list_columns() {
    let db_name = "test_list_columns";
    cleanup_db_file(db_name);

    let config = get_test_config_file(db_name);
    let mut adapter = SQLiteAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create table with various column types
    adapter
        .execute_query(
            "CREATE TABLE test_table (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT UNIQUE,
                age INTEGER,
                balance REAL DEFAULT 0.0,
                active INTEGER DEFAULT 1
            )",
        )
        .await
        .expect("Failed to create table");

    // List columns
    let columns = adapter
        .list_columns(None, None, "test_table")
        .await
        .expect("Failed to list columns");

    assert_eq!(columns.len(), 6);

    // Check id column
    let id_col = columns.iter().find(|c| c.name == "id").unwrap();
    assert_eq!(id_col.data_type, "INTEGER");
    assert!(!id_col.nullable);
    assert!(id_col.is_primary_key);

    // Check name column
    let name_col = columns.iter().find(|c| c.name == "name").unwrap();
    assert_eq!(name_col.data_type, "TEXT");
    assert!(!name_col.nullable);
    assert!(!name_col.is_primary_key);

    // Check balance column
    let balance_col = columns.iter().find(|c| c.name == "balance").unwrap();
    assert_eq!(balance_col.data_type, "REAL");
    assert!(balance_col.nullable);
    assert!(balance_col.default_value.is_some());

    adapter.disconnect().await.expect("Failed to disconnect");
    cleanup_db_file(db_name);
}

#[tokio::test]
async fn test_get_table_info() {
    let db_name = "test_table_info";
    cleanup_db_file(db_name);

    let config = get_test_config_file(db_name);
    let mut adapter = SQLiteAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create and populate table
    adapter
        .execute_query("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)")
        .await
        .expect("Failed to create table");

    adapter
        .execute_query(
            "INSERT INTO products (name) VALUES ('Product 1'), ('Product 2'), ('Product 3')",
        )
        .await
        .expect("Failed to insert data");

    // Get table info
    let table_info = adapter
        .get_table_info(None, None, "products")
        .await
        .expect("Failed to get table info");

    assert_eq!(table_info.name, "products");
    assert_eq!(table_info.table_type, "TABLE");
    assert_eq!(table_info.row_count, Some(3));
    assert_eq!(table_info.schema, Some("main".to_string()));

    adapter.disconnect().await.expect("Failed to disconnect");
    cleanup_db_file(db_name);
}

#[tokio::test]
async fn test_concurrent_reads() {
    let db_name = "test_concurrent_reads";
    cleanup_db_file(db_name);

    let config = get_test_config_file(db_name);
    let mut adapter = SQLiteAdapter::new(config.clone());

    adapter.connect().await.expect("Failed to connect");

    // Create and populate table
    adapter
        .execute_query("CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT)")
        .await
        .expect("Failed to create table");

    for i in 0..100 {
        adapter
            .execute_query(&format!("INSERT INTO items (value) VALUES ('item_{}')", i))
            .await
            .expect("Failed to insert data");
    }

    adapter.disconnect().await.expect("Failed to disconnect");

    // Now test concurrent reads with multiple adapters
    let mut handles = vec![];

    for i in 0..5 {
        let config = config.clone();
        let handle = task::spawn(async move {
            let mut adapter = SQLiteAdapter::new(config);
            adapter.connect().await.expect("Failed to connect");

            // Perform multiple reads
            for _ in 0..10 {
                let result = adapter
                    .execute_query("SELECT COUNT(*) as count FROM items")
                    .await
                    .expect("Failed to query");

                assert_eq!(result.rows.len(), 1);
                let count = match result.rows[0].get("count") {
                    Some(QueryValue::Int(c)) => *c,
                    _ => panic!("Expected integer count"),
                };
                assert_eq!(count, 100);
            }

            adapter.disconnect().await.expect("Failed to disconnect");
            i
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task failed");
    }

    cleanup_db_file(db_name);
}

#[tokio::test]
async fn test_concurrent_writes() {
    let db_name = "test_concurrent_writes";
    cleanup_db_file(db_name);

    let config = get_test_config_file(db_name);
    let mut adapter = SQLiteAdapter::new(config.clone());

    adapter.connect().await.expect("Failed to connect");

    // Create table
    adapter
        .execute_query("CREATE TABLE counter (id INTEGER PRIMARY KEY, count INTEGER)")
        .await
        .expect("Failed to create table");

    adapter
        .execute_query("INSERT INTO counter (id, count) VALUES (1, 0)")
        .await
        .expect("Failed to insert initial value");

    adapter.disconnect().await.expect("Failed to disconnect");

    // Test concurrent writes
    let mut handles = vec![];

    for i in 0..5 {
        let config = config.clone();
        let handle = task::spawn(async move {
            let mut adapter = SQLiteAdapter::new(config);
            adapter.connect().await.expect("Failed to connect");

            // Perform writes
            for _ in 0..10 {
                let _ = adapter
                    .execute_query("UPDATE counter SET count = count + 1 WHERE id = 1")
                    .await;
            }

            adapter.disconnect().await.expect("Failed to disconnect");
            i
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task failed");
    }

    // Verify final count
    let mut adapter = SQLiteAdapter::new(config);
    adapter.connect().await.expect("Failed to connect");

    let result = adapter
        .execute_query("SELECT count FROM counter WHERE id = 1")
        .await
        .expect("Failed to query");

    assert_eq!(result.rows.len(), 1);
    let count = match result.rows[0].get("count") {
        Some(QueryValue::Int(c)) => *c,
        _ => panic!("Expected integer count"),
    };

    // With WAL mode, all writes should succeed
    assert_eq!(count, 50);

    adapter.disconnect().await.expect("Failed to disconnect");
    cleanup_db_file(db_name);
}

#[tokio::test]
async fn test_wal_mode_enabled() {
    let db_name = "test_wal_mode";
    cleanup_db_file(db_name);

    let config = get_test_config_file(db_name);
    let mut adapter = SQLiteAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Check that WAL mode is enabled
    let result = adapter
        .execute_query("PRAGMA journal_mode")
        .await
        .expect("Failed to check journal mode");

    assert_eq!(result.rows.len(), 1);
    let mode = match result.rows[0].get("journal_mode") {
        Some(QueryValue::String(s)) => s.to_uppercase(),
        _ => panic!("Expected string journal mode"),
    };

    assert_eq!(mode, "WAL");

    // Check that foreign keys are enabled
    let result = adapter
        .execute_query("PRAGMA foreign_keys")
        .await
        .expect("Failed to check foreign keys");

    assert_eq!(result.rows.len(), 1);
    let fk_enabled = match result.rows[0].get("foreign_keys") {
        Some(QueryValue::Int(i)) => *i,
        _ => panic!("Expected integer for foreign_keys"),
    };

    assert_eq!(fk_enabled, 1);

    adapter.disconnect().await.expect("Failed to disconnect");
    cleanup_db_file(db_name);
}

#[tokio::test]
async fn test_in_memory_operations() {
    let config = get_test_config_memory();
    let mut adapter = SQLiteAdapter::new(config);

    adapter.connect().await.expect("Failed to connect");

    // Create table
    adapter
        .execute_query("CREATE TABLE temp_data (id INTEGER PRIMARY KEY, value TEXT)")
        .await
        .expect("Failed to create table");

    // Insert data
    adapter
        .execute_query("INSERT INTO temp_data (value) VALUES ('test1'), ('test2')")
        .await
        .expect("Failed to insert data");

    // Query data
    let result = adapter
        .execute_query("SELECT * FROM temp_data ORDER BY id")
        .await
        .expect("Failed to query data");

    assert_eq!(result.rows.len(), 2);

    // List tables
    let tables = adapter
        .list_tables(None, None)
        .await
        .expect("Failed to list tables");

    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].name, "temp_data");

    adapter.disconnect().await.expect("Failed to disconnect");
}
