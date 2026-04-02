//! SQLite adapter usage example.
//!
//! This example demonstrates how to use the SQLite adapter to interact with
//! both file-based and in-memory SQLite databases.
//!
//! Run this example with:
//! ```bash
//! cargo run --example sqlite_usage
//! ```

use sqlkit_lib::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, PoolConfig, SQLiteAdapter, SslMode,
};
use std::fs;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SQLite Adapter Usage Example ===\n");

    // Example 1: File-based database
    println!("--- Example 1: File-based Database ---");
    file_based_example().await?;

    println!("\n--- Example 2: In-memory Database ---");
    in_memory_example().await?;

    println!("\n--- Example 3: Metadata Queries ---");
    metadata_example().await?;

    Ok(())
}

async fn file_based_example() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join("example.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    // Clean up any existing database
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("db-wal"));
    let _ = fs::remove_file(db_path.with_extension("db-shm"));

    // Configure the database connection
    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 10,
        connection_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(1800),
        idle_timeout: Duration::from_secs(600),
    };

    let config = ConnectionConfig::new(DatabaseType::SQLite, "localhost", 0, "local")
        .with_database(&db_path_str)
        .with_pool_config(pool_config)
        .with_ssl_mode(SslMode::Disable);

    let mut adapter = SQLiteAdapter::new(config);

    // Connect to the database
    adapter.connect().await?;
    println!("✓ Connected to file-based database: {}", db_path_str);

    // Test the connection
    let status = adapter.test_connection().await?;
    println!("✓ SQLite version: {}", status.server_version.unwrap());

    // Create a table
    adapter
        .execute_query(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT UNIQUE NOT NULL,
                age INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .await?;
    println!("✓ Created 'users' table");

    // Insert data
    adapter
        .execute_query(
            "INSERT INTO users (name, email, age) VALUES 
                ('Alice Johnson', 'alice@example.com', 30),
                ('Bob Smith', 'bob@example.com', 25),
                ('Charlie Brown', 'charlie@example.com', 35)",
        )
        .await?;
    println!("✓ Inserted 3 users");

    // Query data
    let result = adapter
        .execute_query("SELECT id, name, email, age FROM users ORDER BY name")
        .await?;
    println!("✓ Queried {} users:", result.rows.len());

    for row in &result.rows {
        println!("  - {:?}", row);
    }

    // Update data
    let update_result = adapter
        .execute_query("UPDATE users SET age = age + 1 WHERE age < 30")
        .await?;
    println!(
        "✓ Updated {} user(s)",
        update_result.rows_affected.unwrap_or(0)
    );

    // Disconnect
    adapter.disconnect().await?;
    println!("✓ Disconnected");

    // Clean up
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("db-wal"));
    let _ = fs::remove_file(db_path.with_extension("db-shm"));

    Ok(())
}

async fn in_memory_example() -> Result<(), Box<dyn std::error::Error>> {
    // Configure in-memory database
    let config = ConnectionConfig::new(DatabaseType::SQLite, "localhost", 0, "local")
        .with_database(":memory:")
        .with_ssl_mode(SslMode::Disable);

    let mut adapter = SQLiteAdapter::new(config);

    // Connect
    adapter.connect().await?;
    println!("✓ Connected to in-memory database");

    // Create a table
    adapter
        .execute_query(
            "CREATE TABLE products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                price REAL NOT NULL,
                stock INTEGER DEFAULT 0
            )",
        )
        .await?;
    println!("✓ Created 'products' table");

    // Insert data
    adapter
        .execute_query(
            "INSERT INTO products (name, price, stock) VALUES 
                ('Laptop', 999.99, 10),
                ('Mouse', 29.99, 50),
                ('Keyboard', 79.99, 30)",
        )
        .await?;
    println!("✓ Inserted 3 products");

    // Query with aggregation
    let result = adapter
        .execute_query("SELECT COUNT(*) as total, SUM(stock) as total_stock FROM products")
        .await?;
    println!("✓ Aggregation result:");
    for row in &result.rows {
        println!("  Total products: {:?}", row.get("total"));
        println!("  Total stock: {:?}", row.get("total_stock"));
    }

    // Disconnect
    adapter.disconnect().await?;
    println!("✓ Disconnected");

    Ok(())
}

async fn metadata_example() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join("metadata_example.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    // Clean up
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("db-wal"));
    let _ = fs::remove_file(db_path.with_extension("db-shm"));

    let config = ConnectionConfig::new(DatabaseType::SQLite, "localhost", 0, "local")
        .with_database(&db_path_str)
        .with_ssl_mode(SslMode::Disable);

    let mut adapter = SQLiteAdapter::new(config);
    adapter.connect().await?;
    println!("✓ Connected to database");

    // Create multiple tables
    adapter
        .execute_query(
            "CREATE TABLE customers (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            )",
        )
        .await?;

    adapter
        .execute_query(
            "CREATE TABLE orders (
                id INTEGER PRIMARY KEY,
                customer_id INTEGER,
                amount REAL,
                FOREIGN KEY (customer_id) REFERENCES customers(id)
            )",
        )
        .await?;

    adapter
        .execute_query("CREATE VIEW customer_orders AS SELECT * FROM orders")
        .await?;

    println!("✓ Created tables and views");

    // List databases
    let databases = adapter.list_databases().await?;
    println!("\n✓ Databases:");
    for db in databases {
        println!("  - {}", db.name);
    }

    // List schemas
    let schemas = adapter.list_schemas(None).await?;
    println!("\n✓ Schemas:");
    for schema in schemas {
        println!("  - {}", schema);
    }

    // List tables
    let tables = adapter.list_tables(None, None).await?;
    println!("\n✓ Tables and Views:");
    for table in tables {
        println!("  - {} ({})", table.name, table.table_type);
    }

    // List columns for a table
    let columns = adapter.list_columns(None, None, "orders").await?;
    println!("\n✓ Columns in 'orders' table:");
    for col in columns {
        println!(
            "  - {} ({}) {}{}",
            col.name,
            col.data_type,
            if col.is_primary_key {
                "PRIMARY KEY "
            } else {
                ""
            },
            if col.nullable { "NULL" } else { "NOT NULL" }
        );
    }

    // Get table info
    adapter
        .execute_query("INSERT INTO customers (name) VALUES ('John Doe'), ('Jane Smith')")
        .await?;

    let table_info = adapter.get_table_info(None, None, "customers").await?;
    println!("\n✓ Table info for 'customers':");
    println!("  - Name: {}", table_info.name);
    println!("  - Type: {}", table_info.table_type);
    println!("  - Rows: {}", table_info.row_count.unwrap_or(0));

    // Disconnect
    adapter.disconnect().await?;
    println!("\n✓ Disconnected");

    // Clean up
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("db-wal"));
    let _ = fs::remove_file(db_path.with_extension("db-shm"));

    Ok(())
}
