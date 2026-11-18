# SQLKit Agent Guidelines

## Project Overview

Tauri-based cross-platform SQL database GUI client with Vue 3 + TypeScript frontend and Rust backend.

## UI Design Guidelines

**Before adding or changing client UI, read [DESIGN.md](DESIGN.md).** It is the canonical client design specification; feature design documents describe behavior within that system.

- Preserve SQLKit's existing **blue primary**, light/dark/system themes, compact desktop workspace, and shared shadcn-vue/Radix Vue components. Keep semantic color ownership consistent and retain SQLKit's blue brand and primary actions.
- Use neutral surfaces, typography, spacing, and borders for hierarchy. Keep accent areas scarce (roughly 10% of application chrome); use at most one solid primary action per operation context. Keep Execute compact and unfilled. Preserve the original functional icon and data colors documented in DESIGN.md.
- Colors have fixed roles: `primary` = primary actions/interaction; `success` = actual success/connected/completed; `destructive` = errors/destructive actions. Preserve existing boolean, key/constraint, type, filter, permission, format, and icon recognition colors, with labels or distinct glyphs alongside them. Do not tint whole features or add a separate AI palette. Recognition colors describe content and actions independently of operational status; do not flatten them to gray during surface refactors.
- Reuse semantic CSS tokens from `src/assets/index.css` and their `uno.config.ts` mappings. Preserve existing palette utilities for functional recognition; a token migration must retain the original hues and theme variants. `accent` is a neutral interaction surface, not a second brand color.
- `warning` is registered with light/dark foreground, muted-background, and border pairs; `info` is **not registered**. Use `primary-text` and `destructive-text` for readable colored text, reserving the corresponding base tokens for solid actions. Any new semantic token must include light/dark definitions, necessary foreground pairs, UnoCSS mappings, shared variants where needed, contrast verification, and a DESIGN.md update in the same change.
- Non-interactive badges explicitly use `outline`/`secondary` by default; status colors require real state and accompanying text or icons. Syntax highlighting, data charts, diffs, and small vendor logos have the bounded exceptions documented in DESIGN.md.
- Provide visible keyboard focus, accessible icon-button names, loading/empty/error states, readable contrast, and localized copy. Reuse runtime platform detection for shortcuts and native window behavior.
- Existing functional recognition colors are intentional compatibility constraints. Avoid new unrelated hardcoded colors. Distinguish documented targets from implemented behavior, and verify the affected themes, window sizes, and interactions when changing UI.

## Build/Lint/Test Commands

### Frontend
```bash
npm run dev              # Vite dev server (port 1521, matches tauri.conf devUrl)
npm run build            # Build frontend (vue-tsc + vite)
npm run lint:check       # Check linting
npm run lint:fix         # Auto-fix lint issues
npm test                 # Run all tests with coverage
npm test -- src/__tests__/tabStore.test.ts  # Run single test file
npx jest -t "createTab"  # Run tests matching pattern
```

### Rust Backend
```bash
cd src-tauri
cargo build              # Build
cargo test               # Run all unit tests
cargo test test_name     # Run single test
cargo test --test postgres_integration  # Run integration test
cargo fmt                # Format code
cargo clippy             # Linter
```

### Full App
```bash
npm run tauri dev        # Run Tauri app in dev mode
```

### JDBC Bridge (Java, `jdbc-bridge/`)
```bash
cd jdbc-bridge
mvn clean package -DskipTests \
  -Dmaven.compiler.source=21 -Dmaven.compiler.target=21
# pom targets Java 25 — override to match a local JDK 21+ toolchain

# Deploy: the app loads ~/.sqlkit/jdbc-bridge/jdbc-bridge-<app-version>.jar
# (current app version, e.g. jdbc-bridge-1.0.2.jar; SQLKIT_JDBC_BRIDGE_JAR overrides)
cp target/jdbc-bridge-1.0.0.jar ~/.sqlkit/jdbc-bridge/jdbc-bridge-1.0.2.jar
pkill -f 'java -jar.*jdbc-bridge'   # restart the bridge after every jar swap
```

**Never swap the deployed jar while a bridge JVM is running.** The JVM holds an open handle to the file; an in-place rewrite desyncs its cached zip directory and lazily loaded classes fail with `NoClassDefFoundError` that looks like a broken build. Kill the process first — the app respawns it on the next connection.

Live smoke test (bridge speaks newline-delimited JSON on stdin/stdout):
```bash
echo '{"id":1,"method":"ping"}' | java -jar ~/.sqlkit/jdbc-bridge/jdbc-bridge-1.0.2.jar
```

## Code Style Guidelines

### TypeScript/Frontend

**Functions**: Use arrow functions (`const xxx = (...) => ...`). Prefer functional decomposition over OOP; avoid classes unless strictly necessary.
```typescript
// ✅ Correct
const getConnectionId = () => selectedConnectionId.value || connectionStore.activeConnectionId
const isConnectionActive = (connId: string | null): boolean =>
  connId !== null && connectionStore.getConnectionStatus(connId) === ConnectionStatus.CONNECTED

// ❌ Avoid
function getConnectionId() { ... }
class ConnectionManager { ... }
```

**Collections**: Replace `for`/`while` loops with `map`, `filter`, `find`, `some`, `every`, `reduce`, `flatMap`. Favor pipeline-style transformations.
```typescript
// ✅ Correct
const activeTabs = tabs.filter(t => t.isActive)
const tabById = (id: string) => tabs.find(t => t.id === id)
const names = users.map(u => u.name).sort()

// ❌ Avoid
for (const tab of tabs) { ... }
```

**Types**: Prefer `type`/`enum` over `interface`; use `type` when it can fully replace an `interface`.
```typescript
// ✅ Correct
type QueryTab = {
  id: string
  name: string
}

// ❌ Avoid
interface QueryTab { ... }
```

**Immutability**: Avoid in-place mutation (`push`, `splice`, mutating objects/arrays). Return new arrays/objects; model changes as explicit state-transform functions.
```typescript
// ✅ Correct
const newTabs = [...tabs, newTab]
return { ...state, tabs: newTabs }

// ❌ Avoid
tabs.push(newTab)
state.tabs = tabs
```

**Pure Functions**: Keep functions small, composable, and side-effect-free. If effects are required (I/O, logging), isolate them at boundaries.

**Module Boundaries**: Each module should export only via its `index.ts`; avoid deep imports.
```typescript
// ✅ Correct
import { Button } from '@/components/ui/button'
import { useTabStore } from '@/store/tabStore'

// ❌ Avoid
import { Button } from '@/components/ui/button/Button.vue'
import { useTabStore } from '../store/tabStore'
```

**Export Discipline**: Only export functions/types/constants that are used outside the module.

**Provider-agnostic Design**: Keep provider-agnostic abstractions; follow clean separation of concerns.

**Comments**: Use as few inline comments as possible; behavior should be clear from tests and naming.

**Formatting** (via @antfu/eslint-config):
- Single quotes
- Print width: 100
- Tab width: 2 spaces
- Semicolons: yes
- Arrow parens: avoid `(x) => x` → `x => x`

### Rust Backend

**Error Handling**: All database operations return `DbResult<T>`.
```rust
pub async fn connect(&mut self) -> DbResult<()> {
    let pool = create_pool().map_err(|e| DbError::Connection(e.to_string()))?;
    Ok(())
}
```

**Async**: Use `#[async_trait]` for trait methods, `tokio::sync::Mutex` for shared state.
```rust
#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    async fn connect(&mut self) -> DbResult<()> { ... }
}
```

**Tests**: Use `#[tokio::test]` for async tests, `#[ignore]` for integration tests requiring DB.
```rust
#[tokio::test]
#[ignore]  // Requires running PostgreSQL
async fn test_connection() { ... }
```

## Cross-Platform Support

SQLKit targets **macOS, Windows, and Linux**. All UI elements must be platform-aware.

**Keyboard Shortcuts**: Use `Ctrl` on Windows/Linux, `⌘` (Cmd) on macOS. Detect platform at runtime:
```typescript
const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0
const cmdKey = isMac ? '⌘' : 'Ctrl+'
```

**Never hardcode platform-specific modifiers** in UI labels (shortcuts, hints, tooltips). Always use runtime detection.

**File Paths**: Use `path.join()` or `path.sep` for cross-platform path handling. Never assume `/` or `\` as separator.

**Line Endings**: Handle both `\n` (Unix/macOS) and `\r\n` (Windows) when parsing files. Use `\n` when writing.

## Project Structure

```
src/
├── components/          # Vue components (shadcn-vue based)
│   ├── ui/             # Base UI components
│   └── index.ts        # Module exports only
├── store/              # Pinia stores
├── pages/              # Route pages
├── datasources/        # Tauri API wrappers
└── types/              # TypeScript types

src-tauri/src/
├── commands/           # Tauri command handlers
├── database/           # Database adapters (trait impl)
├── state.rs           # AppState with active connections
└── lib.rs             # Command registration
```

## JDBC Bridge Architecture

The bridge is a Java subprocess (`jdbc-bridge/`) speaking newline-delimited JSON-RPC (`{id, method, params}` → `{id, result, error, error_type}`) over stdin/stdout. It owns a HikariCP pool per connection and is shared across all JDBC connections of the app. Drivers resolve on demand: Maven Central for thin JARs, `download_url` for pre-shaded bundles (Cassandra wrapper, GBase), GitHub Releases for the bridge itself.

## CQL Engines (Cassandra/ScyllaDB)

Connection driver: ING's `cassandra-jdbc-wrapper` pre-shaded bundle from its GitHub releases (Maven Central hosts only the thin JAR); configured in `src-tauri/src/database/jdbc_bridge/drivers.toml`. Both engines share one bundle and cache dir.

CQL diverges from SQL in ways the code must honor:

- **Keyspaces map to JDBC schemas, not catalogs.** The wrapper's catalog lookup returns nothing, and its `DatabaseMetaData` only sees the connect-time keyspace. `JdbcBridgeAdapter::fold_catalog_into_schema` routes metadata RPCs to the schema slot, and `list_databases`/`list_tables`/`list_columns` query `system_schema` directly. Foreign keys and views return empty lists (CQL has no FKs; the wrapper throws on those calls).
- **No TLS negotiation in the CQL native protocol.** The server is fixed to TLS-or-plaintext at startup, so `sslmode=prefer` connects plaintext for CQL (libpq's try-then-fallback does not exist here — a TLS attempt against a plaintext server just times out). `require`/`verify-ca`/`verify-full` enforce TLS. The decision lives in three aligned places: frontend `normalizeSslMode`, bridge `SslPropertyMapper.requiresCqlTls`, and the adapter's ssl_mode translation.
- **No sqlparser dialect.** `classify_cql` (keyword classifier) replaces AST parsing on the MCP/AI risk path, and `wrap_query` passes CQL through unparsed — grid sort/filter/paging is intentionally unavailable for CQL. ScyllaDB 6.x additionally rejects `SimpleStrategy` keyspaces (tablet replication).

## Dependency Version Alignment

`libsqlite3-sys` declares `links = "sqlite3"` — cargo allows exactly one version per build. sqlkit, dockit, and data-studio-agent all consume rusqlite, so **rusqlite upgrades must move all three repos in the same train**: framework PR + tag first, then sqlkit/dockit bumps (data-studio-agent is pinned by tag in `src-tauri/Cargo.toml`). rusqlite 0.43+ also made `chrono` an implicit optional-dependency feature — absent from crates.io metadata but still enabled by `features = ["chrono"]`; verify feature existence against the tag's Cargo.toml, not the registry API.

## Key Patterns

**Tauri Commands**: Return `Result<T, String>`, use `State<'_, AppState>`.
```rust
#[tauri::command]
pub async fn execute_query(
    connection_id: String,
    sql: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    // ...
}
```

**Frontend API Calls**: Use `invoke` from `@tauri-apps/api/core`.
```typescript
const result = await invoke<QueryResult>('execute_query', {
  connectionId: 'server-uuid',
  sql: 'SELECT * FROM users',
})
```

**Vue Components**: Use `<script setup lang="ts">`.
```vue
<script setup lang="ts">
import { ref } from 'vue'

const count = ref(0)
</script>
```

## Testing

**Frontend**: Jest with mocks. Mock Tauri APIs.
```typescript
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))
```

**Rust**: Unit tests in same file with `#[cfg(test)]`, integration tests in `src-tauri/tests/`.

## Git & Commits

**CRITICAL**: NEVER commit changes unless the user explicitly asks you to.

- ❌ Do NOT commit after every change
- ❌ Do NOT commit to "save progress"
- ❌ Do NOT commit without explicit request like "commit this" or "create a commit"
- ✅ Wait for user to explicitly request a commit
- ✅ When asked to commit, use descriptive messages

**When user requests a commit**:
```bash
git add .
git commit -m "feat: description of changes"
git push
```

## Common Tasks

**Adding a Database Engine**:
1. Add the variant to `DatabaseType` in `src-tauri/src/database/config.rs` and wire it in `strategy.rs` (connection routing, default port), `state.rs`, `commands/helpers.rs`, `commands/jdbc.rs`, and `jdbc_bridge/fallback.rs` + `registry.rs`
2. Frontend: enum + `dbTypeToBackend`/`dbTypeFromBackend` in `src/store/connectionStore.ts`, icon in `src/composables/useDatabaseIcon.ts`, SSL behavior in `src/types/connection.ts`, form handling in `ServerFormDialog.vue`, labels in all seven `src/lang/*` files
3. Check `src/components/sidebar/dbCapabilities.ts` — feature-gate what the engine cannot do instead of letting UI affordances fail at runtime
4. JDBC engines additionally need a driver entry in `src-tauri/src/database/jdbc_bridge/drivers.toml` and `DatabaseType` → registry key in `jdbc_bridge/registry.rs`

**Adding a Tauri Command**:
1. Add function in `src-tauri/src/commands/`
2. Add `#[tauri::command]` attribute
3. Export from `commands/mod.rs`
4. Register in `lib.rs` `generate_handler![]`

