//! SQL Server database adapter usage example.
//!
//! This example demonstrates how to use the SQL Server adapter to connect to
//! a SQL Server database and perform various operations.
//!
//! To run this example:
//! ```bash
//! export SQLSERVER_HOST=localhost
//! export SQLSERVER_PORT=1433
//! export SQLSERVER_USER=sa
//! export SQLSERVER_PASSWORD=YourPassword123!
//! export SQLSERVER_DB=testdb
//! cargo run --example sqlserver_usage
//! ```

use sqlkit_lib::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, SqlServerAdapter, PoolConfig, SslMode,
};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get connection details from environment variables
    let host = env::var("SQLSERVER_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("SQLSERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(1433);
    let username = env::var("SQLSERVER_USER").unwrap_or_else(|_| "sa".to_string());
    let password = env::var("SQLSERVER_PASSWORD")?;
    let database = env::var("SQLSERVER_DB").unwrap_or_else(|_| "master".to_string());

    println!("Connecting to SQL Server at {}:{}", host, port);

    // Configure connection pool
    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 10,
        connection_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(1800),
        idle_timeout: Duration::from_secs(600),
    };

    // Create connection configuration
    let config = ConnectionConfig::new(DatabaseType::SqlServer, host, port, username)
        .with_database(database)
        .with_password(password)
        .with_ssl_mode(SslMode::Require)
        .with_pool_config(pool_config)
        .with_option("trust_server_certificate", "true") // For self-signed certificates
        .with_option("application_name", "sqlkit_example");

    // Create adapter and connect
    let mut adapter = SqlServerAdapter::new(config);
    adapter.connect().await?;

    println!("Connected successfully!");

    // Test connection
    let status = adapter.test_connection().await?;
    println!(
        "Server version: {}",
        status.server_version.unwrap_or_else(|| "Unknown".to_string())
    );
    println!(
        "Current database: {}",
        status.current_database.unwrap_or_else(|| "Unknown".to_string())
    );
    println!(
        "Current user: {}",
        status.current_user.unwrap_or_else(|| "Unknown".to_string())
    );

    // List databases
    println!("\n=== Databases ===");
    let databases = adapter.list_databases().await?;
    for db in databases.iter().take(5) {
        println!("  - {}", db.name);
    }
    if databases.len() > 5 {
        println!("  ... and {} more", databases.len() - 5);
    }

    // List schemas
    println!("\n=== Schemas ===");
    let schemas = adapter.list_schemas(None).await?;
    for schema in schemas.iter().take(10) {
        println!("  - {}", schema);
    }
    if schemas.len() > 10 {
        println!("  ... and {} more", schemas.len() - 10);
    }

    // List tables in dbo schema
    println!("\n=== Tables in dbo schema ===");
    let tables = adapter.list_tables(None, Some("dbo")).await?;
    for table in tables.iter().take(10) {
        println!(
            "  - {}.{} ({})",
            table.schema.as_ref().unwrap_or(&"".to_string()),
            table.name,
            table.table_type
        );
    }
    if tables.len() > 10 {
        println!("  ... and {} more", tables.len() - 10);
    }

    // Example: Create a temporary table and query it
    println!("\n=== Example: Working with temporary tables ===");
    
    // Create table
    adapter
        .execute_query(
            "CREATE TABLE #demo_users (
                id INT IDENTITY(1,1) PRIMARY KEY,
                username NVARCHAR(50) NOT NULL,
                email NVARCHAR(100),
                created_at DATETIME2 DEFAULT GETDATE()
            )",
        )
        .await?;
    println!("Created temporary table #demo_users");

    // Insert data
    let insert_result = adapter
        .execute_query(
            "INSERT INTO #demo_users (username, email) VALUES 
                ('alice', 'alice@example.com'),
                ('bob', 'bob@example.com'),
                ('charlie', 'charlie@example.com')",
        )
        .await?;
    println!(
        "Inserted {} rows",
        insert_result.rows_affected.unwrap_or(0)
    );

    // Query data
    let result = adapter
        .execute_query("SELECT * FROM #demo_users ORDER BY id")
        .await?;
    println!("Query returned {} rows:", result.rows.len());
    println!("Execution time: {}ms", result.execution_time.unwrap_or(0));

    // Display results
    for row in result.rows {
        println!("  - User: {:?}", row.get("username"));
    }

    // Example: Complex types
    println!("\n=== Example: Working with complex types ===");
    
    adapter
        .execute_query(
            "CREATE TABLE #demo_complex (
                id INT IDENTITY(1,1) PRIMARY KEY,
                xml_data XML,
                guid_value UNIQUEIDENTIFIER DEFAULT NEWID(),
                json_data NVARCHAR(MAX)
            )",
        )
        .await?;
    
    adapter
        .execute_query(
            r#"INSERT INTO #demo_complex (xml_data, json_data) VALUES 
                ('<root><name>Test</name></root>', '{"type": "example", "status": "active"}')"#,
        )
        .await?;
    
    let complex_result = adapter
        .execute_query("SELECT * FROM #demo_complex")
        .await?;
    
    println!("Complex types data:");
    for row in complex_result.rows {
        println!("  - XML: {:?}", row.get("xml_data"));
        println!("  - GUID: {:?}", row.get("guid_value"));
        println!("  - JSON: {:?}", row.get("json_data"));
    }

    // Disconnect
    adapter.disconnect().await?;
    println!("\nDisconnected from SQL Server");

    Ok(())
}
