# SQLKit Agent Guidelines

## Project Overview

Tauri-based cross-platform SQL database GUI client with Vue 3 + TypeScript frontend and Rust backend.

## Build/Lint/Test Commands

### Frontend
```bash
npm run dev              # Vite dev server (port 1420)
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

**Adding a Tauri Command**:
1. Add function in `src-tauri/src/commands/`
2. Add `#[tauri::command]` attribute
3. Export from `commands/mod.rs`
4. Register in `lib.rs` `generate_handler![]`

**Adding a Database Adapter**:
1. Create `src-tauri/src/database/newdb.rs`
2. Implement `DatabaseAdapter` trait
3. Add to `mod.rs` exports
4. Add variant to `ActiveConnection` enum in `state.rs`
5. Update command match statements

