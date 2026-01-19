//! MySQL adapter usage example.
//!
//! This example demonstrates how to use the MySQL database adapter.
//!
//! To run this example:
//! ```bash
//! export MYSQL_HOST=localhost
//! export MYSQL_PORT=3306
//! export MYSQL_USER=root
//! export MYSQL_PASSWORD=your_password
//! export MYSQL_DB=your_database
//! cargo run --example mysql_usage
//! ```

use sqlkit_lib::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, MySQLAdapter, PoolConfig, SslMode,
};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("MySQL Adapter Usage Example\n");

    // Get connection parameters from environment
    let host = env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("MYSQL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3306);
    let username = env::var("MYSQL_USER").unwrap_or_else(|_| "root".to_string());
    let password = env::var("MYSQL_PASSWORD").ok();
    let database = env::var("MYSQL_DB").unwrap_or_else(|_| "mysql".to_string());

    println!("Connecting to MySQL at {}:{}", host, port);

    // Configure connection pool
    let pool_config = PoolConfig {
        min_connections: 2,
        max_connections: 10,
        connection_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(1800), // 30 minutes
        idle_timeout: Duration::from_secs(600),   // 10 minutes
    };

    // Create connection configuration
    let mut config = ConnectionConfig::new(DatabaseType::MySQL, host, port, username)
        .with_database(database)
        .with_pool_config(pool_config)
        .with_ssl_mode(SslMode::Prefer)
        .with_option("connect_timeout", "10");

    if let Some(pwd) = password {
        config = config.with_password(pwd);
    }

    // Create adapter and connect
    let mut adapter = MySQLAdapter::new(config);
    adapter.connect().await?;

    println!("✓ Connected successfully!\n");

    // Test connection and get server info
    println!("=== Connection Status ===");
    let status = adapter.test_connection().await?;
    println!("Server Version: {}", status.server_version.unwrap());
    println!(
        "Current Database: {}",
        status.current_database.unwrap_or_else(|| "None".to_string())
    );
    println!("Current User: {}\n", status.current_user.unwrap());

    // List all databases
    println!("=== Available Databases ===");
    let databases = adapter.list_databases().await?;
    for db in databases.iter().take(10) {
        println!("  • {}", db.name);
    }
    if databases.len() > 10 {
        println!("  ... and {} more", databases.len() - 10);
    }
    println!();

    // Create a temporary test table
    println!("=== Creating Test Table ===");
    adapter
        .execute_query(
            "CREATE TEMPORARY TABLE demo_users (
                id INT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                email VARCHAR(255),
                age INT,
                balance DECIMAL(10, 2),
                is_active BOOLEAN DEFAULT TRUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .await?;
    println!("✓ Table 'demo_users' created\n");

    // Insert sample data
    println!("=== Inserting Sample Data ===");
    let result = adapter
        .execute_query(
            "INSERT INTO demo_users (name, email, age, balance) VALUES 
                ('Alice Johnson', 'alice@example.com', 30, 1250.50),
                ('Bob Smith', 'bob@example.com', 25, 890.25),
                ('Charlie Brown', 'charlie@example.com', 35, 2100.00),
                ('Diana Prince', 'diana@example.com', 28, 1500.75)",
        )
        .await?;
    println!("✓ Inserted {} rows\n", result.rows_affected.unwrap());

    // Query the data
    println!("=== Querying Data ===");
    let result = adapter
        .execute_query("SELECT id, name, email, balance FROM demo_users ORDER BY balance DESC")
        .await?;

    println!("Query returned {} rows:", result.rows.len());
    println!(
        "Execution time: {} ms\n",
        result.execution_time_ms.unwrap_or(0)
    );

    for (i, row) in result.rows.iter().enumerate() {
        println!("Row {}:", i + 1);
        for (col_name, value) in row {
            println!("  {}: {:?}", col_name, value);
        }
        println!();
    }

    // Test transactions
    println!("=== Testing Transaction Support ===");
    adapter.execute_query("START TRANSACTION").await?;
    adapter
        .execute_query("INSERT INTO demo_users (name, email, age, balance) VALUES ('Test User', 'test@example.com', 99, 0.00)")
        .await?;
    adapter.execute_query("ROLLBACK").await?;
    println!("✓ Transaction rolled back successfully\n");

    // Test connection pool health
    println!("=== Connection Pool Health Check ===");
    if let Some(pool) = adapter.get_pool() {
        pool.health_check().await?;
        println!("✓ Connection pool is healthy\n");
    }

    // Disconnect
    println!("=== Disconnecting ===");
    adapter.disconnect().await?;
    println!("✓ Disconnected successfully\n");

    println!("Example completed successfully!");

    Ok(())
}
