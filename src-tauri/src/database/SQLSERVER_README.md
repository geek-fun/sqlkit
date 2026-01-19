# SQL Server Database Adapter

This module provides a comprehensive implementation of the `DatabaseAdapter` trait for Microsoft SQL Server databases using the `tiberius` client library.

## Features

- **Connection Management**: Robust connection pooling with configurable pool size and timeouts
- **Authentication**: Support for both SQL Server and Windows Authentication
- **TLS/SSL**: Full encryption support with certificate validation options
- **Complex Types**: Native handling of SQL Server-specific types:
  - XML
  - UNIQUEIDENTIFIER (GUID)
  - DATETIME2, DATETIMEOFFSET
  - NUMERIC, DECIMAL with precision
  - VARBINARY, BINARY
  - NVARCHAR, NTEXT (Unicode strings)
- **Schema Support**: Complete metadata access for databases, schemas, tables, and columns
- **Query Timeout**: Configurable statement timeout
- **SQL Server 2016+**: Compatible with modern SQL Server versions

## Installation

Add the required dependencies to your `Cargo.toml`:

```toml
[dependencies]
tiberius = { version = "0.12", default-features = false, features = [
  "sql-browser-tokio",
  "tds73",
  "rustls"
] }
tokio-util = { version = "0.7", features = [ "compat" ] }
tokio = { version = "1", features = [ "full" ] }
async-trait = "0.1"
```

## Usage

### Basic Connection

```rust
use sqlkit::database::{
    ConnectionConfig, DatabaseAdapter, DatabaseType, SqlServerAdapter, SslMode
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration with SQL Server Authentication
    let config = ConnectionConfig::new(
        DatabaseType::SqlServer,
        "localhost",
        1433,
        "sa"
    )
    .with_database("testdb")
    .with_password("YourPassword123!")
    .with_ssl_mode(SslMode::Require)
    .with_option("trust_server_certificate", "true"); // For self-signed certs

    // Create adapter and connect
    let mut adapter = SqlServerAdapter::new(config);
    adapter.connect().await?;

    // Test connection
    let status = adapter.test_connection().await?;
    println!("Connected to: {:?}", status.server_version);

    // Disconnect
    adapter.disconnect().await?;
    Ok(())
}
```

### Windows Authentication

For Windows Authentication (Integrated Security):

```rust
let config = ConnectionConfig::new(
    DatabaseType::SqlServer,
    "localhost",
    1433,
    "" // Username not required for Windows Auth
)
.with_database("testdb")
.with_option("use_windows_auth", "true")
.with_ssl_mode(SslMode::Require);
```

### SSL/TLS Configuration

#### Trust Server Certificate (for self-signed certificates)

```rust
let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa")
    .with_password("password")
    .with_ssl_mode(SslMode::Require)
    .with_option("trust_server_certificate", "true");
```

#### Verify Certificate (production)

```rust
let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa")
    .with_password("password")
    .with_ssl_mode(SslMode::VerifyFull); // Validates certificate chain
```

### Connection Pooling

```rust
use std::time::Duration;
use sqlkit::database::PoolConfig;

let pool_config = PoolConfig {
    min_connections: 2,
    max_connections: 20,
    connection_timeout: Duration::from_secs(30),
    max_lifetime: Duration::from_secs(1800),
    idle_timeout: Duration::from_secs(600),
};

let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa")
    .with_password("password")
    .with_database("mydb")
    .with_pool_config(pool_config);
```

### Query Execution

#### SELECT Query

```rust
let result = adapter.execute_query("SELECT * FROM users WHERE age > 18").await?;

println!("Found {} rows", result.rows.len());
for row in result.rows {
    println!("User: {:?}", row.get("username"));
}
```

#### INSERT/UPDATE/DELETE

```rust
let result = adapter
    .execute_query("INSERT INTO users (username, email) VALUES ('alice', 'alice@example.com')")
    .await?;

println!("Affected rows: {}", result.rows_affected.unwrap_or(0));
```

#### Parameterized queries are handled through the tiberius client

```rust
// Note: The current implementation uses simple_query
// For parameterized queries, you would need to access the underlying client
```

### Working with Complex Types

#### XML Data

```rust
adapter.execute_query(
    "CREATE TABLE documents (
        id INT PRIMARY KEY,
        content XML
    )"
).await?;

adapter.execute_query(
    "INSERT INTO documents VALUES (1, '<root><item>data</item></root>')"
).await?;

let result = adapter.execute_query("SELECT * FROM documents").await?;
// XML is returned as QueryValue::String
```

#### UNIQUEIDENTIFIER (GUID)

```rust
adapter.execute_query(
    "CREATE TABLE entities (
        id UNIQUEIDENTIFIER PRIMARY KEY DEFAULT NEWID(),
        name NVARCHAR(100)
    )"
).await?;

let result = adapter.execute_query("SELECT * FROM entities").await?;
// GUID is returned as QueryValue::String
```

#### Temporal Types

```rust
let result = adapter.execute_query(
    "SELECT
        CAST('2024-01-15' AS DATE) as date_col,
        CAST('2024-01-15 10:30:00' AS DATETIME2) as datetime_col,
        CAST('2024-01-15 10:30:00 +00:00' AS DATETIMEOFFSET) as offset_col"
).await?;
// All temporal types are returned as QueryValue::DateTime(String)
```

### Metadata Operations

#### List Databases

```rust
let databases = adapter.list_databases().await?;
for db in databases {
    println!("Database: {}", db.name);
}
```

#### List Schemas

```rust
let schemas = adapter.list_schemas(None).await?;
for schema in schemas {
    println!("Schema: {}", schema);
}
```

#### List Tables

```rust
// List tables in 'dbo' schema
let tables = adapter.list_tables(None, Some("dbo")).await?;
for table in tables {
    println!("Table: {}.{} ({})",
        table.schema.unwrap_or_default(),
        table.name,
        table.table_type
    );
}
```

#### Get Table Information

```rust
let table_info = adapter.get_table_info(None, Some("dbo"), "users").await?;
println!("Table: {}", table_info.name);
println!("Rows: {:?}", table_info.row_count);
println!("Size: {:?} bytes", table_info.size_bytes);
```

#### List Columns

```rust
let columns = adapter.list_columns(None, Some("dbo"), "users").await?;
for col in columns {
    println!("Column: {} ({}) - Nullable: {}, PK: {}",
        col.name,
        col.data_type,
        col.nullable,
        col.is_primary_key
    );
}
```

### Query Timeout

Configure a timeout for long-running queries:

```rust
let config = ConnectionConfig::new(DatabaseType::SqlServer, "localhost", 1433, "sa")
    .with_password("password")
    .with_option("statement_timeout", "30000"); // 30 seconds in milliseconds

let mut adapter = SqlServerAdapter::new(config);
adapter.connect().await?;

// This will timeout if it runs longer than 30 seconds
let result = adapter.execute_query("SELECT * FROM large_table").await;
```

## Type Mapping

SQL Server types are mapped to `QueryValue` as follows:

| SQL Server Type                             | QueryValue Type  | Notes                              |
| ------------------------------------------- | ---------------- | ---------------------------------- |
| BIT                                         | Bool             | Boolean values                     |
| TINYINT, SMALLINT, INT, BIGINT              | Int(i64)         | All integer types                  |
| FLOAT, REAL                                 | Float(f64)       | Floating point numbers             |
| NUMERIC, DECIMAL                            | String           | High precision decimals as strings |
| CHAR, VARCHAR, NCHAR, NVARCHAR, TEXT, NTEXT | String           | Character data                     |
| BINARY, VARBINARY, IMAGE                    | Bytes(Vec<u8>)   | Binary data                        |
| DATE, DATETIME, DATETIME2, SMALLDATETIME    | DateTime(String) | Date and time values               |
| DATETIMEOFFSET                              | DateTime(String) | Timezone-aware timestamps          |
| TIME                                        | String           | Time values                        |
| UNIQUEIDENTIFIER                            | String           | GUIDs as strings                   |
| XML                                         | String           | XML documents as strings           |
| NULL                                        | Null             | Null values                        |

## Error Handling

All operations return `DbResult<T>` which is an alias for `Result<T, DbError>`:

```rust
use sqlkit::database::DbError;

match adapter.execute_query("SELECT * FROM users").await {
    Ok(result) => println!("Success: {} rows", result.rows.len()),
    Err(DbError::Connection(msg)) => eprintln!("Connection error: {}", msg),
    Err(DbError::QueryExecution(msg)) => eprintln!("Query error: {}", msg),
    Err(DbError::Timeout(msg)) => eprintln!("Timeout: {}", msg),
    Err(DbError::Authentication(msg)) => eprintln!("Auth error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Connection String Options

Additional options can be configured via `with_option()`:

| Option                     | Type   | Description                              |
| -------------------------- | ------ | ---------------------------------------- |
| `use_windows_auth`         | bool   | Enable Windows Authentication            |
| `trust_server_certificate` | bool   | Trust self-signed certificates           |
| `connect_timeout`          | u64    | Connection timeout in seconds            |
| `statement_timeout`        | u64    | Query timeout in milliseconds            |
| `application_name`         | string | Application name for connection tracking |

## Running Tests

Integration tests require a running SQL Server instance:

```bash
# Set environment variables
export SQLSERVER_HOST=localhost
export SQLSERVER_PORT=1433
export SQLSERVER_USER=sa
export SQLSERVER_PASSWORD=YourPassword123!
export SQLSERVER_DB=testdb
export SQLSERVER_TRUST_CERT=true

# Run tests
cargo test --test sqlserver_integration -- --nocapture --ignored
```

## Running Examples

```bash
# Set environment variables
export SQLSERVER_HOST=localhost
export SQLSERVER_PORT=1433
export SQLSERVER_USER=sa
export SQLSERVER_PASSWORD=YourPassword123!
export SQLSERVER_DB=testdb

# Run example
cargo run --example sqlserver_usage
```

## Limitations

1. **Multiple Result Sets**: When a query returns multiple result sets, only the first set is returned
2. **Prepared Statements**: Current implementation uses simple queries; prepared statements are not exposed through the adapter interface
3. **Transactions**: Transaction management is not currently exposed through the adapter interface
4. **Connection Pooling**: Uses a simple custom pool implementation; for high-performance scenarios, consider using a dedicated connection pool library

## Performance Considerations

- Use connection pooling for multi-threaded applications
- Set appropriate timeout values for your workload
- Enable TLS only when required (adds overhead)
- Use appropriate data types to minimize conversions
- Consider using temporary tables (prefixed with #) for session-specific data

## Security Best Practices

1. **Always use SSL/TLS in production**: Set `SslMode::Require` or higher
2. **Validate certificates**: Use `SslMode::VerifyFull` instead of trusting all certificates
3. **Use strong passwords**: Minimum 8 characters with mixed case, numbers, and symbols
4. **Principle of least privilege**: Use database users with minimal required permissions
5. **Rotate credentials regularly**: Change passwords and connection strings periodically
6. **Store credentials securely**: Use environment variables or secure vaults, never hardcode

## Troubleshooting

### Connection Failures

- Verify SQL Server is running and accessible
- Check firewall rules allow port 1433
- Ensure TCP/IP protocol is enabled in SQL Server Configuration Manager
- Verify SQL Server Authentication is enabled (for SQL auth)

### Authentication Errors

- Check username and password are correct
- For Windows Auth, ensure the machine is domain-joined
- Verify the user has permission to access the database

### SSL/TLS Errors

- Use `trust_server_certificate: true` for self-signed certificates
- Ensure SQL Server is configured to use encryption
- Check certificate validity and trust chain

### Query Timeouts

- Increase `statement_timeout` for long-running queries
- Optimize slow queries with proper indexes
- Consider breaking large operations into smaller batches

## Contributing

When contributing to the SQL Server adapter:

1. Follow the existing code style and patterns
2. Add tests for new features
3. Update documentation
4. Ensure all tests pass
5. Consider backward compatibility

## License

This adapter is part of the sqlkit project and follows the same license.
