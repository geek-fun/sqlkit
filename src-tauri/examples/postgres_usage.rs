//! Example usage of the PostgreSQL adapter.
//!
//! This example demonstrates how to use the PostgreSQL adapter to connect
//! to a database and perform various operations.
//!
//! To run this example:
//! ```bash
//! cargo run --example postgres_usage
//! ```
//!
//! Make sure to set the following environment variables:
//! - POSTGRES_HOST (default: localhost)
//! - POSTGRES_PORT (default: 5432)
//! - POSTGRES_USER (default: postgres)
//! - POSTGRES_PASSWORD
//! - POSTGRES_DB (default: postgres)

use sqlkit_lib::database::{
    ConnectionConfig, ConnectionPool, DatabaseAdapter, DatabaseType, PoolConfig, PostgresAdapter,
    SslMode,
};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PostgreSQL Adapter Example");
    println!("==========================\n");

    // Get configuration from environment variables
    let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("POSTGRES_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432);
    let username = env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
    let password = env::var("POSTGRES_PASSWORD").ok();
    let database = env::var("POSTGRES_DB").unwrap_or_else(|_| "postgres".to_string());

    // Configure connection pooling
    let pool_config = PoolConfig {
        min_connections: 2,
        max_connections: 10,
        connection_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(1800), // 30 minutes
        idle_timeout: Duration::from_secs(600),  // 10 minutes
    };

    // Build connection configuration
    let mut config = ConnectionConfig::new(DatabaseType::PostgreSQL, host, port, username)
        .with_database(&database)
        .with_pool_config(pool_config)
        .with_ssl_mode(SslMode::Prefer)
        .with_option("application_name", "sqlkit_example")
        .with_option("statement_timeout", "30000"); // 30 seconds

    if let Some(pwd) = password {
        config = config.with_password(pwd);
    }

    // Create and connect the adapter
    println!("Connecting to PostgreSQL...");
    let mut adapter = PostgresAdapter::new(config);
    adapter.connect().await?;
    println!("✓ Connected successfully\n");

    // Test connection
    println!("Testing connection...");
    let status = adapter.test_connection().await?;
    println!("✓ Connection Status:");
    println!(
        "  - Server Version: {}",
        status.server_version.unwrap_or_default()
    );
    println!(
        "  - Current Database: {}",
        status.current_database.unwrap_or_default()
    );
    println!(
        "  - Current User: {}\n",
        status.current_user.unwrap_or_default()
    );

    // List databases
    println!("Listing databases...");
    let databases = adapter.list_databases().await?;
    println!("✓ Found {} databases:", databases.len());
    for db in databases.iter().take(5) {
        println!("  - {}", db.name);
    }
    if databases.len() > 5 {
        println!("  ... and {} more", databases.len() - 5);
    }
    println!();

    // List schemas
    println!("Listing schemas in current database...");
    let schemas = adapter.list_schemas(None).await?;
    println!("✓ Found {} schemas:", schemas.len());
    for schema in &schemas {
        println!("  - {}", schema);
    }
    println!();

    // List tables in public schema
    println!("Listing tables in 'public' schema...");
    let tables = adapter.list_tables(None, Some("public")).await?;
    println!("✓ Found {} tables/views:", tables.len());
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
    println!();

    // Execute a simple query
    println!("Executing query: SELECT version()");
    let result = adapter.execute_query("SELECT version() as version").await?;
    println!(
        "✓ Query executed in {}ms",
        result.execution_time_ms.unwrap_or(0)
    );
    if let Some(row) = result.rows.first() {
        if let Some(version) = row.get("version") {
            println!("  PostgreSQL Version: {:?}\n", version);
        }
    }

    // Show a sample of current connections
    println!("Executing query: SELECT current_timestamp, current_user");
    let result = adapter
        .execute_query("SELECT current_timestamp, current_user")
        .await?;
    println!(
        "✓ Query executed in {}ms",
        result.execution_time_ms.unwrap_or(0)
    );
    println!("  Columns: {:?}", result.columns);
    println!("  Rows: {}\n", result.rows.len());

    // Demonstrate array and JSON handling
    println!("Testing complex types (arrays and JSON)...");
    let query = r#"
        SELECT 
            ARRAY[1, 2, 3, 4, 5] as numbers,
            ARRAY['rust', 'postgres', 'sql'] as tags,
            '{"name": "Alice", "age": 30}'::json as user_json,
            '{"role": "admin", "active": true}'::jsonb as metadata
    "#;
    let result = adapter.execute_query(query).await?;
    println!(
        "✓ Complex types query executed in {}ms",
        result.execution_time_ms.unwrap_or(0)
    );
    if let Some(row) = result.rows.first() {
        println!("  Result contains: {:?}", row.keys().collect::<Vec<_>>());
    }
    println!();

    // Check pool statistics
    println!("Connection Pool Statistics:");
    if let Some(pool) = adapter.get_pool() {
        println!("  - Active connections: {}", pool.active_connections());
        println!("  - Idle connections: {}", pool.idle_connections());
        println!("  - Max connections: {}", pool.max_connections());
    }
    println!();

    // Disconnect
    println!("Disconnecting...");
    adapter.disconnect().await?;
    println!("✓ Disconnected successfully\n");

    println!("Example completed!");

    Ok(())
}
