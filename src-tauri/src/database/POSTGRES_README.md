# PostgreSQL Adapter

This module provides a complete implementation of the `DatabaseAdapter` trait for PostgreSQL databases.

## Features

### Connection Management

- **Connection Pooling**: Uses `deadpool-postgres` for efficient connection pooling
- **Configurable Pool Size**: Set minimum and maximum connections via `PoolConfig`
- **Connection Timeout**: Configurable timeout for acquiring connections from the pool
- **Automatic Connection Reuse**: Connections are automatically returned to the pool when dropped

### SSL/TLS Support

Supports all PostgreSQL SSL modes:

- `Disable`: No SSL encryption
- `Prefer`: Use SSL if available, fallback to unencrypted
- `Require`: Require SSL, fail if not available
- `VerifyCA`: Require SSL and verify the CA certificate
- `VerifyFull`: Require SSL and verify the full certificate chain

### Complex Type Handling

The adapter handles PostgreSQL's rich type system:

- **Primitive Types**: BOOL, INT2, INT4, INT8, FLOAT4, FLOAT8
- **String Types**: VARCHAR, TEXT, BPCHAR, NAME
- **Binary Data**: BYTEA
- **JSON Types**: JSON and JSONB (converted to string representation)
- **Temporal Types**: TIMESTAMP, TIMESTAMPTZ, DATE, TIME, TIMETZ
- **Array Types**: All array types (converted to string representation)

### Schema Support

PostgreSQL's multi-schema architecture is fully supported:

- List all schemas in a database
- Filter tables and columns by schema
- Default to "public" schema when not specified
- Automatically exclude system schemas (pg_catalog, information_schema, pg_toast, pg_temp)

### Query Execution

- **Timeout Support**: Configure query timeouts via `statement_timeout` option
- **Prepared Statements**: Automatically used by tokio-postgres for better performance
- **Query Type Detection**: Automatically distinguishes between SELECT and DML queries
- **Execution Time Tracking**: Returns execution time for all queries

### Metadata Retrieval

Comprehensive metadata access:

- **Databases**: List all databases with descriptions
- **Schemas**: List all user schemas (excluding system schemas)
- **Tables**: List tables and views with schema information
- **Columns**: Detailed column information including:
  - Data type
  - Nullable flag
  - Default values
  - Primary key status
  - Auto-increment detection (via sequences)
  - Column descriptions/comments
  - Precision and scale for numeric types
  - Maximum length for string types
- **Table Statistics**: Row count and size in bytes for tables

## Usage Example

```rust
use sqlkit::database::{
    PostgresAdapter, ConnectionConfig, DatabaseType, SslMode, PoolConfig
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a connection configuration
    let pool_config = PoolConfig {
        min_connections: 2,
        max_connections: 20,
        connection_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(1800),
        idle_timeout: Duration::from_secs(600),
    };

    let config = ConnectionConfig::new(
        DatabaseType::PostgreSQL,
        "localhost",
        5432,
        "postgres",
    )
    .with_database("mydb")
    .with_password("password")
    .with_ssl_mode(SslMode::Require)
    .with_pool_config(pool_config)
    .with_option("application_name", "sqlkit")
    .with_option("statement_timeout", "30000"); // 30 seconds

    // Create and connect the adapter
    let mut adapter = PostgresAdapter::new(config);
    adapter.connect().await?;

    // Test the connection
    let status = adapter.test_connection().await?;
    println!("Connected to PostgreSQL {}", status.server_version.unwrap());
    println!("Current database: {}", status.current_database.unwrap());

    // Execute a query
    let result = adapter.execute_query("SELECT * FROM users LIMIT 10").await?;
    println!("Retrieved {} rows in {}ms",
        result.rows.len(),
        result.execution_time_ms.unwrap_or(0));

    // List all databases
    let databases = adapter.list_databases().await?;
    for db in databases {
        println!("Database: {} - {}", db.name, db.description.unwrap_or_default());
    }

    // List schemas in the current database
    let schemas = adapter.list_schemas(None).await?;
    for schema in schemas {
        println!("Schema: {}", schema);
    }

    // List tables in a specific schema
    let tables = adapter.list_tables(None, Some("public")).await?;
    for table in tables {
        println!("Table: {}.{} ({})",
            table.schema.unwrap_or_default(),
            table.name,
            table.table_type);
    }

    // Get detailed column information
    let columns = adapter.list_columns(None, Some("public"), "users").await?;
    for col in columns {
        println!("Column: {} {} {}",
            col.name,
            col.data_type,
            if col.is_primary_key { "[PK]" } else { "" });
    }

    // Get table statistics
    let table_info = adapter.get_table_info(None, Some("public"), "users").await?;
    println!("Table 'users' has {} rows and is {} bytes",
        table_info.row_count.unwrap_or(0),
        table_info.size_bytes.unwrap_or(0));

    // Disconnect
    adapter.disconnect().await?;

    Ok(())
}
```

## Configuration Options

### Connection Options

You can pass additional PostgreSQL connection options using `with_option()`:

```rust
let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "user")
    .with_option("application_name", "my_app")      // Set application name
    .with_option("connect_timeout", "10")           // Connection timeout in seconds
    .with_option("statement_timeout", "30000")      // Query timeout in milliseconds
    .with_option("search_path", "public,custom")    // Schema search path
    .with_option("timezone", "UTC");                // Set timezone
```

### Pool Configuration

Configure connection pooling behavior:

```rust
use std::time::Duration;

let pool_config = PoolConfig {
    min_connections: 2,                           // Minimum connections to maintain
    max_connections: 20,                          // Maximum connections allowed
    connection_timeout: Duration::from_secs(30),  // Max wait time for a connection
    max_lifetime: Duration::from_secs(1800),      // Max connection lifetime (30 min)
    idle_timeout: Duration::from_secs(600),       // Max idle time (10 min)
};

let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "user")
    .with_pool_config(pool_config);
```

## PostgreSQL Version Support

The adapter is designed to work with PostgreSQL 12 and later versions. It uses standard PostgreSQL system catalogs and information schema tables that are stable across versions.

## Thread Safety

The `PostgresAdapter` is fully thread-safe and can be safely shared across multiple threads. The connection pool handles concurrent access automatically.

## Error Handling

All methods return `DbResult<T>` which is an alias for `Result<T, DbError>`. The adapter provides detailed error information:

```rust
match adapter.execute_query("SELECT * FROM users").await {
    Ok(result) => println!("Query succeeded: {} rows", result.rows.len()),
    Err(DbError::Connection(msg)) => eprintln!("Connection error: {}", msg),
    Err(DbError::QueryExecution(msg)) => eprintln!("Query error: {}", msg),
    Err(DbError::Timeout(msg)) => eprintln!("Query timeout: {}", msg),
    Err(DbError::Authentication(msg)) => eprintln!("Auth error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Testing

The module includes comprehensive unit tests for:

- Adapter creation
- Connection string building
- SSL mode mapping
- Configuration validation
- Option handling

Run tests with:

```bash
cargo test --lib database::postgres
```

For integration tests with a real PostgreSQL database, ensure you have a PostgreSQL instance running and set the appropriate environment variables:

```bash
export POSTGRES_HOST=localhost
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=password
export POSTGRES_DB=testdb

cargo test --test postgres_integration
```

## Limitations

- **Cross-Database Queries**: The adapter cannot query across different databases without reconnecting. If you need to work with multiple databases, create separate adapter instances.
- **Large Result Sets**: Very large result sets are loaded entirely into memory. For large datasets, consider using pagination in your queries.
- **Streaming**: The current implementation does not support streaming query results. All rows are fetched before returning.

## Future Enhancements

Potential improvements for future versions:

- Support for prepared statement caching
- Streaming query results for large datasets
- Support for PostgreSQL COPY protocol
- Binary protocol support for better performance
- Support for PostgreSQL notifications (LISTEN/NOTIFY)
- Support for cursors for large result sets
- Better handling of PostgreSQL-specific types (geometric types, network types, etc.)
