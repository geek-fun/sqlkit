# SQLKit Copilot Instructions

## Project Overview

SQLKit is a **Tauri-based cross-platform SQL database GUI client** with an AI-powered interface. The architecture splits into two distinct parts:

- **Frontend**: Vue 3 + TypeScript + UnoCSS (shadcn-vue components)
- **Backend**: Rust with async database adapters for PostgreSQL, MySQL, SQL Server, and SQLite

## Architecture

### Rust Backend (`src-tauri/`)

The core abstraction is the **`DatabaseAdapter` trait** ([src-tauri/src/database/adapter.rs](src-tauri/src/database/adapter.rs)) which provides a unified interface for all database operations:

```rust
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    type Pool: ConnectionPool;
    async fn connect(&mut self) -> DbResult<()>;
    async fn execute_query(&self, query: &str) -> DbResult<QueryResult>;
    async fn list_databases(&self) -> DbResult<Vec<DatabaseSchema>>;
    // ... metadata retrieval methods
}
```

Each database has its own adapter ([postgres.rs](src-tauri/src/database/postgres.rs), [mysql.rs](src-tauri/src/database/mysql.rs), [sqlserver.rs](src-tauri/src/database/sqlserver.rs), [sqlite.rs](src-tauri/src/database/sqlite.rs)) implementing this trait with database-specific connection pooling.

**Connection Management Pattern**:

- `ConnectionManager<P>` ([manager.rs](src-tauri/src/database/manager.rs)) wraps pools to add health checks, metadata tracking, and lifecycle management
- Each adapter has its own `*Pool` struct implementing the `ConnectionPool` trait ([pool.rs](src-tauri/src/database/pool.rs))
- Pools use database-specific clients: `deadpool-postgres`, `mysql_async`, `tiberius`, `rusqlite`

**Configuration Builders**: Use fluent builder pattern:

```rust
ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "user")
    .with_database("mydb")
    .with_password("pass")
    .with_ssl_mode(SslMode::Require)
```

### Frontend (`src/`)

- **Layout Structure**: `AppLayout.vue` → `AppHeader.vue` + `AppSidebar.vue` + slot for main content
- **UI Components**: shadcn-vue based components in `src/components/ui/` (Button, Card, Dialog, Input, Label, Table)
- **Monaco Editor Integration**: `useMonacoEditor.ts` composable configures SQL syntax highlighting and autocomplete with custom worker setup for Vite
- **Theme System**: `useTheme.ts` composable + CSS variables in `assets/index.css` for light/dark modes

**Styling**: UnoCSS with Wind4 preset + shadcn preset ([uno.config.ts](uno.config.ts)). Uses utility-first approach with Tailwind-like syntax.

## Development Workflows

### Running the App

```bash
npm install                # Install frontend deps
npm run tauri dev          # Run in dev mode (starts Vite + Tauri)
```

Vite runs on **port 1420** (fixed, will fail if unavailable). HMR runs on 1421.

### Testing

**Frontend Tests**: Jest with ts-jest

```bash
npm test                   # Run with coverage
npm run test:ci            # CI mode
```

**Rust Tests**: Standard Cargo tests + integration tests in `src-tauri/tests/`

```bash
cd src-tauri
cargo test                 # Unit tests in *_tests.rs and integration tests
cargo test --test postgres_integration  # Specific integration test
```

Integration tests require actual database instances (see [BUILD.md](BUILD.md) for setup).

### Linting & Formatting

- **Frontend**: `@antfu/eslint-config` with auto-fix on pre-commit
- **Rust**: Standard `rustfmt` and `clippy`

```bash
npm run lint:fix           # Fix frontend issues
cargo fmt                  # Format Rust code
cargo clippy               # Rust linter
```

## Conventions & Patterns

### Rust Error Handling

All database operations return `DbResult<T>` (type alias for `Result<T, DbError>`). The `DbError` enum ([error.rs](src-tauri/src/database/error.rs)) has specific variants for Connection, Authentication, Query, Timeout, Pool errors.

### Async Patterns

- All adapter methods use `#[async_trait]` for trait async support
- Use Tokio runtime (`tokio = { version = "1", features = ["full"] }`)
- Connection pools return `Arc<Connection>` for shared ownership
- **State synchronization**: Use `tokio::sync::Mutex` for state accessed across Tauri commands (not `std::sync::Mutex` - it's not Send across await points)
- In tests: Use `blocking_lock()` for tokio::Mutex when in synchronous test setup functions

### Type Conversions

Each adapter implements database-specific type mapping to `QueryValue` enum:

- PostgreSQL: Handles arrays, JSON/JSONB, custom types
- MySQL: JSON columns, binary data, various numeric types
- SQL Server: XML, UNIQUEIDENTIFIER, DATETIME2, hierarchyid
- SQLite: Limited type system (NULL, INTEGER, REAL, TEXT, BLOB)

### Vue Component Structure

Use `<script setup lang="ts">` syntax with Composition API. Import UI components from relative paths (aliased with `@/` for src).

### Tauri Commands

Commands are defined with `#[tauri::command]` and registered in [lib.rs](src-tauri/src/lib.rs) via `generate_handler![]`.

**Implemented Commands** (15 total across 4 modules):

- **Server Management** ([server.rs](src-tauri/src/commands/server.rs)): `save_server`, `list_servers`, `delete_server`, `test_connection`
- **Connection Lifecycle** ([connection.rs](src-tauri/src/commands/connection.rs)): `connect_server`, `disconnect_server`, `get_connection_status`
- **Query Execution** ([query.rs](src-tauri/src/commands/query.rs)): `execute_query`, `cancel_query` (placeholder), `explain_query`
- **Database Browsing** ([browse.rs](src-tauri/src/commands/browse.rs)): `list_databases`, `list_schemas`, `list_tables`, `get_table_info`, `get_table_data`

**State Management**: `AppState` ([state.rs](src-tauri/src/state.rs)) holds:

- Active database connections (wrapped adapters in tokio::sync::Mutex)
- Server configurations (persisted via `AppConfig`)
- Uses `Arc<Mutex<T>>` for thread-safe shared state across commands

**Key Pattern**: Commands store full adapter instances (not just pools) in `ActiveConnection` enum, making query execution straightforward without recreating adapters.

## Coding/Architecture Guidelines

### TypeScript/Frontend Patterns

- **Use functional TypeScript**: define functions as `const xxx = (...) => ...`. Prefer **functional decomposition** over OOP; **avoid classes** unless strictly necessary.
- **Prefer declarative/functional collection handling**: replace `for`/`while` loops with `map`, `filter`, `find`, `some`, `every`, `reduce`, `flatMap` (and `sort` when appropriate). Favor pipeline-style transformations over step-by-step imperative logic.
- **Favor immutability**: avoid in-place mutation (`push`, `splice`, mutating objects/arrays, shared mutable state). Instead, return new arrays/objects and model changes as explicit state-transform functions (e.g., reducers).
- **Prefer pure functions**: keep functions small, composable, and side-effect-free where possible. If effects are required (I/O, logging), isolate them at the boundaries and keep core logic pure.
- **Types**: prefer `type`/`enum` over `interface` where possible; use `type` when it can fully replace an `interface`.
- **Module boundaries**: each module should export **only** via its `index.ts`; avoid deep imports.
- **Export discipline**: only export functions/types/constants that are used outside the module.
- **Provider-agnostic design**: keep provider-agnostic abstractions and follow clean separation of concerns.
- **Comments and documentation**: use as few inline comments as possible; behavior should be clear from tests and naming. When unifying/refactoring code, document the newly unified sections and the migration process with targeted comments and **README updates**.

## Key Files to Reference

- [src-tauri/src/database/README.md](src-tauri/src/database/README.md): Comprehensive database module docs
- [BUILD.md](BUILD.md): Platform-specific build requirements
- [src/components/README.md](src/components/README.md): UI component usage examples
- [src-tauri/examples/](src-tauri/examples/): Usage examples for each database adapter

## Integration Points

**Frontend ↔ Rust**: Tauri IPC layer (not yet fully implemented for database operations)

- Frontend will invoke Rust commands via `@tauri-apps/api`
- Return data as JSON-serializable `QueryResult` structs

**External Dependencies**:

- PostgreSQL: Requires OpenSSL/native-tls for SSL connections
- SQL Server: Uses rustls (pure Rust TLS), supports TDS 7.3+
- Monaco Editor: Custom worker configuration in Vite for web worker support

## Common Tasks

**Adding a new database adapter**:

1. Create `src-tauri/src/database/yourdb.rs`
2. Implement `DatabaseAdapter` trait and `ConnectionPool` trait for `YourDbPool`
3. Add to [mod.rs](src-tauri/src/database/mod.rs) exports
4. Create usage example in `src-tauri/examples/`
5. Add variant to `ActiveConnection` enum in [state.rs](src-tauri/src/state.rs)
6. Update command match statements in [connection.rs](src-tauri/src/commands/connection.rs), [query.rs](src-tauri/src/commands/query.rs), and [browse.rs](src-tauri/src/commands/browse.rs)

**Adding Tauri commands**:

1. Define in appropriate module under `src-tauri/src/commands/`
2. Add `#[tauri::command]` attribute
3. Use `State<'_, AppState>` parameter for accessing shared state
4. Return `Result<T, String>` where T is JSON-serializable
5. Export from [commands/mod.rs](src-tauri/src/commands/mod.rs)
6. Register in [lib.rs](src-tauri/src/lib.rs) `generate_handler![]` macro
7. Add unit tests in the same file using `#[tokio::test]`

**Calling commands from frontend**: Use `@tauri-apps/api` invoke:

```typescript
import { invoke } from '@tauri-apps/api/core'

const result = await invoke<QueryResult>('execute_query', {
  connectionId: 'server-uuid',
  sql: 'SELECT * FROM users'
})
```
