# Database Adapter Module

This module provides a unified interface for interacting with various database systems through the `DatabaseAdapter` trait.

## Overview

The database adapter module is designed to support multiple database types:

- **PostgreSQL** ✅ (Implemented - see [POSTGRES_README.md](./POSTGRES_README.md))
- **MySQL** ✅ (Implemented)
- **SQL Server** ✅ (Implemented)
- Oracle (Planned)
- IBM DB2 (Planned)
- SQLite (Planned)
- H2 (Planned)
- ClickHouse (Planned)

## Implemented Adapters

### PostgreSQL

Full-featured adapter with:

- Connection pooling with deadpool-postgres
- SSL/TLS support (all modes)
- Complex type handling (arrays, JSON, JSONB, timestamps)
- Schema support
- Query timeout
- Prepared statements

See [POSTGRES_README.md](./POSTGRES_README.md) for detailed documentation and usage examples.

### MySQL

Full-featured adapter with:

- Connection pooling with mysql_async
- SSL/TLS support (all modes)
- Complex type handling (JSON, binary data, timestamps)
- Database-level operations
- Query timeout
- Prepared statements

### SQL Server

Full-featured adapter with:

- Connection pooling with custom implementation
- TLS/SSL support with certificate validation
- SQL Server and Windows Authentication
- Complex type handling (XML, UNIQUEIDENTIFIER, DATETIME2, etc.)
- Schema support (databases, schemas, tables, columns)
- Query timeout
- Support for SQL Server 2016+

## Architecture

### Core Components

#### DatabaseAdapter Trait

The main trait that defines the interface for all database operations:

- Connection management (`connect`, `disconnect`, `test_connection`)
- Query execution (`execute_query`)
- Metadata retrieval (`list_databases`, `list_schemas`, `list_tables`, `list_columns`, `get_table_info`)
- Connection pooling support

#### Error Types

Comprehensive error handling through the `DbError` enum:

- Connection errors
- Authentication failures
- Query execution errors
- Timeout errors
- Pool errors
- And more...

#### Configuration

- `ConnectionConfig`: Database connection parameters
- `DatabaseType`: Enum for supported database types
- `SslMode`: SSL/TLS configuration options
- `PoolConfig`: Connection pooling parameters

#### Connection Pooling

- `ConnectionPool` trait: Interface for connection pool implementations
- `PoolStats`: Statistics and metrics for pool monitoring

#### Data Types

- `QueryResult`: Results from query execution
- `QueryValue`: Individual cell values
- `DatabaseSchema`, `TableInfo`, `ColumnInfo`: Metadata structures

## Usage Example

```rust
use sqlkit::database::{
    DatabaseAdapter, ConnectionConfig, DatabaseType, SslMode
};

async fn example<A: DatabaseAdapter>(mut adapter: A) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to database
    adapter.connect().await?;

    // Test connection
    let status = adapter.test_connection().await?;
    println!("Connected to: {:?}", status.server_version);

    // Execute a query
    let result = adapter.execute_query("SELECT * FROM users LIMIT 10").await?;
    println!("Retrieved {} rows", result.rows.len());

    // List databases
    let databases = adapter.list_databases().await?;
    for db in databases {
        println!("Database: {}", db.name);
    }

    // List tables
    let tables = adapter.list_tables(None, None).await?;
    for table in tables {
        println!("Table: {}", table.name);
    }

    // Get table info
    let table_info = adapter.get_table_info(None, None, "users").await?;
    println!("Table has {} rows", table_info.row_count.unwrap_or(0));

    // List columns
    let columns = adapter.list_columns(None, None, "users").await?;
    for col in columns {
        println!("Column: {} ({})", col.name, col.data_type);
    }

    // Disconnect
    adapter.disconnect().await?;

    Ok(())
}
```

## Configuration Example

```rust
use sqlkit::database::{ConnectionConfig, DatabaseType, SslMode, PoolConfig};
use std::time::Duration;

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
    "myuser",
)
.with_database("mydb")
.with_password("mypassword")
.with_ssl_mode(SslMode::Require)
.with_pool_config(pool_config)
.with_option("application_name", "sqlkit");
```

## Implementing a Database Adapter

To implement support for a new database:

1. Create a struct for your adapter:

```rust
pub struct MyDatabaseAdapter {
    config: ConnectionConfig,
    pool: Option<Arc<MyConnectionPool>>,
    // ... other fields
}
```

2. Implement the `DatabaseAdapter` trait:

```rust
#[async_trait]
impl DatabaseAdapter for MyDatabaseAdapter {
    type Pool = MyConnectionPool;

    async fn connect(&mut self) -> DbResult<()> {
        // Implementation
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        // Implementation
    }

    // ... implement other required methods
}
```

3. Optionally implement the `ConnectionPool` trait for pooling support:

```rust
#[async_trait]
impl ConnectionPool for MyConnectionPool {
    type Connection = MyConnection;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        // Implementation
    }

    // ... implement other required methods
}
```

## Error Handling

All operations return `DbResult<T>`, which is an alias for `Result<T, DbError>`. The `DbError` enum provides detailed error information:

```rust
match adapter.execute_query("SELECT * FROM users").await {
    Ok(result) => println!("Success: {} rows", result.rows.len()),
    Err(DbError::Connection(msg)) => eprintln!("Connection error: {}", msg),
    Err(DbError::QueryExecution(msg)) => eprintln!("Query error: {}", msg),
    Err(DbError::Timeout(msg)) => eprintln!("Timeout: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Testing

The module includes comprehensive unit tests for all data structures and configuration builders. Run tests with:

```bash
cargo test --lib
```

## Thread Safety

All traits and types are designed to be `Send` and `Sync`, enabling safe concurrent access across multiple threads.
