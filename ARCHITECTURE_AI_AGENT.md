# SQLKit Data Studio AI Agent — Architecture & Implementation Plan

## 1. Executive Summary

Port the Data Studio AI agent from **DocKit** to **SQLKit**. The agent enables natural-language database querying: users attach a data source (PostgreSQL, MySQL, SQL Server, SQLite → future Oracle, DB2, H2, ClickHouse), chat with an LLM that understands the database schema, and the LLM generates & executes SQL queries via tool calls.

### Reference: DocKit Implementation
DocKit's agent supports **Elasticsearch, OpenSearch, DynamoDB, MongoDB** via a capability-registry + agent-loop architecture. SQLKit needs the same for **SQL databases**—the core (LLM orchestration, session management, event streaming, permissions) is identical; only the **tool implementations** (capabilities) differ.

---

## 2. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│  FRONTEND (Vue 3 + TypeScript)                                      │
│                                                                     │
│  DataStudioPage.vue                                                 │
│    ├── ChatPanel.vue (reused from DocKit)                            │
│    ├── useDataStudioChatAgent.ts (→ useChatAgent.ts)                 │
│    ├── agentRuntime.ts (event handlers)                              │
│    └── dataStudioStore.ts (Pinia: sessions, messages, sources)       │
│                                                                     │
│  agentApi.ts ─────────────────── Tauri IPC ──────────────────────── │
└─────────────────────────────────────────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────────┐
│  BACKEND (Rust)                                                      │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  agent/ module                                                │   │
│  │  ├── harness.rs      — run_agent_step, validate_llm_config     │   │
│  │  ├── loop_runner.rs  — run_agent_loop orchestrator             │   │
│  │  ├── tool_executor.rs— dispatch tool calls                     │   │
│  │  ├── conversation.rs — message persistence                     │   │
│  │  ├── compact.rs      — context window compaction               │   │
│  │  ├── chat_formatter/ — OpenAI/Anthropic message formatting      │   │
│  │  ├── config.rs       — provider config resolution              │   │
│  │  └── ...             — model_registry, token_counter, etc.     │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  capabilities/ module (SQL-specific tools)                    │   │
│  │  ├── types.rs        — Capability, CapabilityHandler, RiskLevel│  │
│  │  ├── registry.rs     — global CapabilityRegistry              │   │
│  │  ├── commands.rs     — invoke_capability, get_available_tools  │   │
│  │  ├── sqlkit.rs       — SQLKit app-level tools (list_connections)│  │
│  │  ├── postgres.rs     — PostgreSQL agent tools (future)         │   │
│  │  ├── mysql.rs        — MySQL agent tools (future)              │   │
│  │  ├── sqlserver.rs    — SQL Server agent tools (future)         │   │
│  │  └── sqlite.rs       — SQLite agent tools (future)             │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  database/ module (EXISTING)                                  │   │
│  │  ├── adapter.rs      — DatabaseAdapter trait                   │   │
│  │  ├── postgres.rs     — PostgresAdapter                         │   │
│  │  ├── mysql.rs        — MySQLAdapter                            │   │
│  │  ├── sqlserver.rs    — SqlServerAdapter                        │   │
│  │  ├── sqlite.rs       — SQLiteAdapter                           │   │
│  │  └── types.rs        — QueryResult, ColumnInfo, etc.           │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  state.rs — AppState (EXISTING, extended with AgentDb)              │
│  lib.rs   — Tauri command registration                              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Reference Architecture: DocKit Agent (What We're Porting)

### 3.1 Frontend Files (To Port)

| File | Purpose | Port Status |
|---|---|---|
| `views/data-studio/index.vue` | Main Data Studio page with ChatPanel, source attachment | **Port (adapt)** |
| `views/data-studio/components/session-history-panel.vue` | Session list sidebar | **Port** |
| `views/data-studio/components/modify-source-modal.vue` | Edit source permissions | **Port** |
| `views/data-studio/components/tool-confirmation-card.vue` | Tool confirmation dialog | **Port** |
| `store/dataStudioStore.ts` | Pinia store: sessions, messages, sources, rules | **Port** |
| `composables/useDataStudioChatAgent.ts` | Agent composable (adapts store → chat agent) | **Port** |
| `composables/useChatAgent.ts` | Core chat agent logic | **Port** |
| `composables/agentRuntime.ts` | Agent loop event handlers | **Port** |
| `composables/useAgentContext.ts` | Context string builder (simplify) | **Adapt** |
| `components/chat-panel.vue` | Chat UI component | **Port** |
| `datasources/agentApi.ts` | Tauri IPC for agent commands/events | **Port (adapt)** |
| `types/chat.ts` | Chat type definitions | **Port** |
| `common/jsonify.ts` | JSON parse helper | **Port** |

### 3.2 Backend Files (To Port)

| File | Purpose | Port Status |
|---|---|---|
| `agent/mod.rs` | Module declarations | **Port** |
| `agent/harness.rs` | `run_agent_step`, `validate_llm_config`, `list_llm_models` | **Port** |
| `agent/loop_runner.rs` | `run_agent_loop`, `cancel_agent_loop`, streaming events | **Port** |
| `agent/loop_runner_support.rs` | Support utilities for loop runner | **Port** |
| `agent/tool_executor.rs` | Tool execution dispatch | **Port** |
| `agent/executor.rs` | Tool result envelope types | **Port** |
| `agent/conversation.rs` | Message persistence in SQLite | **Port** |
| `agent/compact.rs` | Context window compaction | **Port** |
| `agent/config.rs` | Provider/base URL resolution | **Port** |
| `agent/chat_formatter/mod.rs` | Chat formatter trait | **Port** |
| `agent/chat_formatter/openai.rs` | OpenAI message formatting | **Port** |
| `agent/chat_formatter/anthropic.rs` | Anthropic message formatting | **Port** |
| `agent/provider_adapter.rs` | API compatibility mapping | **Port** |
| `agent/model_registry.rs` | Model/provider registry | **Port** |
| `agent/session_store.rs` | Session CRUD in SQLite | **Port** |
| `agent/token_counter.rs` | Token counting (tiktoken-rs) | **Port** |
| `agent/query_history.rs` | Query history | **Port** |
| `agent/tools.rs` | `get_all_tools` command | **Port** |
| `capabilities/mod.rs` | Module declarations | **Port** |
| `capabilities/types.rs` | Capability trait, RiskLevel, SourceKind | **Port** |
| `capabilities/registry.rs` | Global capability registry | **Port** |
| `capabilities/commands.rs` | `invoke_capability`, `get_available_tools` | **Port** |
| `capabilities/dockit.rs` | App-level tools (→ rename to `sqlkit.rs`) | **Adapt** |
| `common/http_client.rs` | HTTP client for LLM calls | **Port** |
| `common/format.rs` | Tool output truncation | **Port** |
| `common/connection_resolver.rs` | Resolve connection config (→ adapt for SQL) | **Adapt** |

### 3.3 New SQL Capabilities (Build from Scratch)

| File | Purpose |
|---|---|
| `capabilities/postgres.rs` | PostgreSQL tools: `sql_query`, `list_tables`, `get_schema`, etc. |
| `capabilities/mysql.rs` | MySQL tools (same interface) |
| `capabilities/sqlserver.rs` | SQL Server tools (same interface) |
| `capabilities/sqlite.rs` | SQLite tools (same interface) |

---

## 4. Implementation Phases

### Phase 1: Foundation — Backend Agent Infrastructure (Rust)
**Goal**: Get the agent loop working with LLM providers, no database tools yet.

#### 1.1 Add Cargo Dependencies
```toml
# In src-tauri/Cargo.toml
async-openai = "0.25" # Or latest
tiktoken-rs = "0.5" # Token counting
reqwest = { version = "0.12", features = [
  "json",
  "native-tls"
] }
futures = "0.3"
rand = "0.8"
```

#### 1.2 Port `agent/` Module
Copy Dockit's `agent/` module (15 files). Adapt:
- `conversation.rs` / `session_store.rs`: SQLKit already uses `rusqlite` bundled — adapt the `AgentDb` to use the same SQLite connection.
- `harness.rs` / `loop_runner.rs`: Use `reqwest` instead of Dockit's `common::http_client` for LLM API calls (or port `http_client.rs` as-is).
- `chat_formatter/`: Port both OpenAI and Anthropic formatters.

#### 1.3 Port `capabilities/` Module (minus DB-specific tools)
- `types.rs`, `registry.rs`, `commands.rs`: Direct port.
- `capabilities/sqlkit.rs`: Port the `dockit.rs` tools (`sqlkit__list_connections`, etc.) adapted for SQLKit's backend types.

#### 1.4 Register Commands in `lib.rs`
Add all agent Tauri commands: `run_agent_loop`, `cancel_agent_loop`, `run_agent_step`, `validate_llm_config`, `confirm_tool_call`, `get_available_tools`, `compact_agent_session`, etc.

#### 1.5 Add `AgentDb` to AppState
The agent needs a SQLite database for session/message persistence. SQLKit already has `tauri-plugin-store` — use a separate SQLite database (`~/.sqlkit/agent.db`) managed by the agent module.

### Phase 2: Tool System — SQL Capabilities (Rust)
**Goal**: Define and register SQL query tools for all supported databases.

#### 2.1 Define SQL Capability Traits
Create a unified SQL tool handler that takes a connection config + SQL string and delegates to the appropriate `DatabaseAdapter`:

```rust
// capabilities/sql.rs (shared base for all SQL databases)
pub async fn execute_sql_tool(
    sql: &str,
    config: &Value,           // connection config JSON
    state: &AppState,
) -> Result<String, String> {
    let conn_id = config["connectionId"].as_str().ok_or("missing connectionId")?;
    let adapter = resolve_adapter(state, conn_id).await?;
    let result = adapter.execute_query(sql).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&result).map_err(|e| e.to_string())?)
}
```

#### 2.2 Register Agent Tools for Each Database Type
Each SQL database will expose these agent tools:
- `sqlkit__execute_query` — execute any SQL, return results
- `sqlkit__list_databases` — list databases on server
- `sqlkit__list_schemas` — list schemas in a database
- `sqlkit__list_tables` — list tables in a schema
- `sqlkit__get_schema` — full schema dump for context
- `sqlkit__describe_table` — column info for a table
- `sqlkit__explain_query` — query execution plan

These tools are registered with `SourceKind::Database("POSTGRESQL")` etc., so they're only shown when the user attaches a matching database type.

#### 2.3 Connection Resolution
Port `common/connection_resolver.rs` but adapt it to look up connections from `AppState` (the active connections HashMap) instead of Dockit's config file pattern.

### Phase 3: Frontend — UI (Vue 3 + TypeScript)
**Goal**: Replace the placeholder DataStudioPage with the full agent chat UI.

#### 3.1 Port Core Files
Copy and adapt:
- `store/dataStudioStore.ts` → `sqlkit/src/store/dataStudioStore.ts`
- `composables/useChatAgent.ts` → `sqlkit/src/composables/useChatAgent.ts`
- `composables/agentRuntime.ts` → `sqlkit/src/composables/agentRuntime.ts`
- `composables/useDataStudioChatAgent.ts` → `sqlkit/src/composables/useDataStudioChatAgent.ts`
- `datasources/agentApi.ts` → `sqlkit/src/datasources/agentApi.ts`
- `types/chat.ts` → `sqlkit/src/types/chat.ts`

#### 3.2 Port UI Components
- `components/chat-panel.vue` → port as-is (it's generic)
- `views/data-studio/index.vue` → port, adapt for SQLKit's database types:
  - Change `AGENT_SUPPORTED_TYPES` to use SQLKit's `DatabaseType` (POSTGRESQL, MYSQL, SQLITE, SQLSERVER)
  - Update `getConnectionIcon` to use SQLKit's database icons
  - Update `getConnectionMeta` for SQL connection display

#### 3.3 Replace Placeholder DataStudioPage
Replace `pages/DataStudioPage.vue` with the ported data-studio view.

#### 3.4 LLM Settings Integration
SQLKit already has an `appStore` with `llmSettings` — ensure compatibility with Dockit's provider/model system so the agent settings UI works.

### Phase 4: Context & Schema Gathering
**Goal**: Inject database schema into LLM context for informed query generation.

#### 4.1 Schema Aggregation
When a user attaches a data source, the agent should:
1. Call `list_databases()`, `list_schemas()`, `list_tables()`, `list_columns()` to build a full schema map
2. Format it as a DDL-like string for the system prompt
3. Cache the schema and invalidate on explicit refresh

#### 4.2 Context-Aware System Prompt
Port `useChatAgent.ts`'s `buildSystemPrompt()` but adapt the database knowledge sections for SQL:

```typescript
function _buildSQLKnowledge(dbType: string): string {
  const knowledge = {
    POSTGRESQL: [
      'PostgreSQL knowledge:',
      '- Uses standard SQL with extensions (JSON/JSONB, arrays, full-text search)',
      '- Supports schemas (public by default), transactions, CTEs, window functions',
      '- String concat: || operator (not +)',
      '- ILIKE for case-insensitive matching',
      '- LIMIT/OFFSET for pagination',
      '- RETURNING clause for INSERT/UPDATE/DELETE',
    ],
    MYSQL: [
      'MySQL knowledge:',
      '- Uses standard SQL with some variations',
      '- Backtick quoting for identifiers',
      '- LIMIT/OFFSET for pagination',
      '- CONCAT() function for string concatenation (not ||)',
      '- AUTO_INCREMENT for auto-incrementing columns',
      '- SHOW statements for metadata',
    ],
    SQLSERVER: [
      'SQL Server knowledge:',
      '- Uses T-SQL dialect',
      '- Square bracket quoting for identifiers: [column_name]',
      '- TOP n instead of LIMIT',
      '- OFFSET/FETCH NEXT for pagination (SQL Server 2012+)',
      '- GETDATE()/GETUTCDATE() for current datetime',
      '- String concat: + operator',
      '- IDENTITY for auto-incrementing columns',
      '- Supports schemas (dbo by default)',
    ],
    SQLITE: [
      'SQLite knowledge:',
      '- Lightweight embedded SQL database',
      '- Limited ALTER TABLE support (can only ADD COLUMN or RENAME)',
      '- No RIGHT JOIN or FULL OUTER JOIN',
      '- Uses dynamic typing (affinity-based)',
      '- LIMIT/OFFSET for pagination',
      '- No stored procedures',
    ],
  }
  return knowledge[dbType]?.join('\n') ?? ''
}
```

### Phase 5: Permissions & Safety
**Goal**: Prevent destructive queries without confirmation.

#### 5.1 Risk Classification
Classify SQL operations by risk:
- **Safe**: SELECT, EXPLAIN, SHOW, DESCRIBE, PRAGMA
- **Elevated**: INSERT, UPDATE, DELETE (with WHERE), CREATE TEMP TABLE
- **Destructive**: DROP TABLE, TRUNCATE, ALTER, DELETE without WHERE, DROP DATABASE

#### 5.2 Permission Guard
Port Dockit's `shouldRequireConfirmation()` logic — in Ask mode, elevated+destructive operations require user confirmation via ToolConfirmationCard. In Auto mode, only destructive requires confirmation.

#### 5.3 Read-Only Mode
Support session-level read-only mode where only SELECT queries are allowed. This maps to Dockit's permissions model (per-source read/create/update/delete flags).

### Phase 6: Polish & Edge Cases
- **Large result sets**: Auto-truncate at configurable limits (Dockit uses `truncate_tool_output`)
- **Query cancellation**: Port `cancel_query` command
- **Context window management**: Port auto-compaction when approaching token limits
- **Multi-source sessions**: Support attaching multiple databases to one session
- **Session history**: Port session-history-panel for browsing past conversations

---

## 5. Detailed File Manifest

### 5.1 Backend Files to Create

#### Ported from Dockit (adapt as needed):
```
src-tauri/src/agent/
├── mod.rs
├── harness.rs              # run_agent_step, validate_llm_config, list_llm_models
├── loop_runner.rs          # run_agent_loop orchestrator (~1300 lines)
├── loop_runner_support.rs  # helpers
├── tool_executor.rs        # tool dispatch
├── executor.rs             # ToolEnvelope types
├── conversation.rs         # message persistence (→ adapt for rusqlite)
├── compact.rs              # context compaction
├── config.rs               # provider config
├── chat_formatter/
│   ├── mod.rs
│   ├── openai.rs
│   └── anthropic.rs
├── provider_adapter.rs     # API compatibility
├── model_registry.rs       # model/provider registry
├── session_store.rs        # session CRUD
├── token_counter.rs        # token counting
├── query_history.rs        # query history
└── tools.rs                # get_all_tools command

src-tauri/src/capabilities/
├── mod.rs
├── types.rs                # Capability, CapabilityHandler trait
├── registry.rs             # global registry
├── commands.rs             # invoke_capability, get_available_tools
├── sqlkit.rs               # app-level tools (list_connections, etc.)
├── postgres.rs             # PostgreSQL tools (NEW)
├── mysql.rs                # MySQL tools (NEW)
├── sqlserver.rs            # SQL Server tools (NEW)
└── sqlite.rs               # SQLite tools (NEW)

src-tauri/src/common/
├── mod.rs
├── http_client.rs          # HTTP client for LLM calls
├── format.rs               # tool output truncation
└── connection_resolver.rs  # resolve connection config from state
```

### 5.2 Frontend Files to Create

```
src/
├── store/
│   └── dataStudioStore.ts          # Ported from Dockit
├── composables/
│   ├── useChatAgent.ts             # Ported from Dockit (core agent logic)
│   ├── agentRuntime.ts             # Ported from Dockit (event handlers)
│   ├── useDataStudioChatAgent.ts   # Ported from Dockit (data-studio specific)
│   └── useAgentContext.ts          # Ported from Dockit (simplify)
├── datasources/
│   └── agentApi.ts                 # Ported from Dockit (Tauri IPC)
├── types/
│   └── chat.ts                     # Ported from Dockit
├── components/
│   ├── chat-panel.vue              # Ported from Dockit
│   ├── agent-message-bubble.vue    # Ported from Dockit (or extract from chat-panel)
│   └── data-studio/                # New directory
│       ├── session-history-panel.vue
│       ├── modify-source-modal.vue
│       └── tool-confirmation-card.vue
└── pages/
    └── DataStudioPage.vue          # REPLACE placeholder
```

---

## 6. Key Integration Points

### 6.1 Connection Resolution
**Dockit approach**: Connections are resolved from a JSON config file via `ConnectionResolver`.

**SQLKit approach**: Connections are already in memory via `AppState.connections: Arc<Mutex<HashMap<String, ActiveConnection>>>`. The agent needs to look up a connection by ID and get its `ConnectionConfig` to reconstruct an adapter for tool execution.

**Solution**: Add a method to `AppState`:
```rust
impl AppState {
    pub async fn get_connection_config(&self, conn_id: &str) -> Result<ActiveConnection, String> {
        let conns = self.connections.lock().await;
        conns.get(conn_id).cloned().ok_or_else(|| format!("Connection not found: {}", conn_id))
    }
}
```

### 6.2 SQLite Database for Agent Persistence
**Dockit approach**: Dedicated `AgentDb` struct managing a SQLite file via `rusqlite`.

**SQLKit approach**: SQLKit also uses `rusqlite` (bundled). Create `~/.sqlkit/agent.db` with the same schema:
```sql
CREATE TABLE agent_sessions (id TEXT PRIMARY KEY, title TEXT, status TEXT, ...);
CREATE TABLE agent_messages (id TEXT PRIMARY KEY, session_id TEXT, role TEXT, content TEXT, ...);
CREATE TABLE attached_sources (id TEXT PRIMARY KEY, kind TEXT, ...);
CREATE TABLE confirmation_rules (id TEXT PRIMARY KEY, session_id TEXT, tool_name TEXT, ...);
```

### 6.3 LLM Provider Configuration
**Dockit approach**: `appStore.llmSettings.providers[]` — user configures API keys, base URLs, models via UI.

**SQLKit approach**: SQLKit already has `appStore` — ensure it has the same `llmSettings` structure or adapt the agent to use SQLKit's existing config mechanism. If not present, add a similar provider config store.

### 6.4 Tauri Event Streaming
The agent loop uses `app.emit("agent-loop-delta", payload)` to stream LLM responses to the frontend. SQLKit's Tauri setup already supports `Emitter` — ensure `AppHandle` is passed to the agent loop.

---

## 7. Naming & Conventions

### Agent Tool Naming
```text
# SQLKit agent tools follow the pattern:
sqlkit__<action>[_<subresource>]

# Examples:
sqlkit__execute_query
sqlkit__list_databases
sqlkit__list_schemas
sqlkit__list_tables
sqlkit__get_schema
sqlkit__describe_table
sqlkit__explain_query
sqlkit__list_connections       # App-level (no source needed)
```

### SourceKind Values
```rust
// Each supported SQL database type
SourceKind::Database("POSTGRESQL")
SourceKind::Database("MYSQL")
SourceKind::Database("SQLSERVER")
SourceKind::Database("SQLITE")
// Future
SourceKind::Database("ORACLE")
SourceKind::Database("DB2")
SourceKind::Database("H2")
SourceKind::Database("CLICKHOUSE")
```

### Database Type in config
Dockit uses `ELASTICSEARCH`, `DYNAMODB`, `MONGODB`.
SQLKit uses `POSTGRESQL`, `MYSQL`, `SQLSERVER`, `SQLITE` (already defined in `connectionStore.ts`).

---

## 8. Implementation Order

### Sprint 1: Backend Skeleton (3-5 days)
1. Add Cargo dependencies (`async-openai`, `tiktoken-rs`, `reqwest`)
2. Port `agent/` module (all files) — adapt `conversation.rs`/`session_store.rs` for SQLKit's DB
3. Port `capabilities/types.rs`, `registry.rs`, `commands.rs`
4. Port `common/http_client.rs`, `format.rs`, `connection_resolver.rs`
5. Create `capabilities/sqlkit.rs` with `sqlkit__list_connections` tool
6. Register all agent commands in `lib.rs`
7. **Verify**: `cargo build` passes, `get_available_tools` returns `sqlkit__list_connections`

### Sprint 2: SQL Tools (2-3 days)
1. Create `capabilities/sql.rs` — shared SQL tool executor using `DatabaseAdapter`
2. Create `capabilities/postgres.rs` — register tools for `POSTGRESQL` source kind
3. Create `capabilities/mysql.rs` — register tools for `MYSQL` source kind
4. Create `capabilities/sqlserver.rs` — register tools for `SQLSERVER` source kind
5. Create `capabilities/sqlite.rs` — register tools for `SQLITE` source kind
6. Initialize registry in `main.rs` setup
7. **Verify**: Tools appear filtered by source kind

### Sprint 3: Frontend Core (3-4 days)
1. Port `store/dataStudioStore.ts` — sessions, messages, sources
2. Port `datasources/agentApi.ts` — all `invoke` + `listen` calls
3. Port `composables/useChatAgent.ts` — core agent logic
4. Port `composables/agentRuntime.ts` — event handlers
5. Port `composables/useDataStudioChatAgent.ts` — data-studio wrapper
6. Port `types/chat.ts`
7. **Verify**: Agent loop starts, events stream

### Sprint 4: Frontend UI (2-3 days)
1. Port `components/chat-panel.vue` — chat UI with messages, input, tool confirmation
2. Create `components/agent-message-bubble.vue` (if separate from chat-panel)
3. Port data-studio components (session-history-panel, modify-source-modal, tool-confirmation-card)
4. Rewrite `pages/DataStudioPage.vue` — replace placeholder with full agent view
5. Update router to point to new DataStudioPage
6. Add i18n keys for all agent UI strings
7. **Verify**: Full agent UI renders, can type messages, see streaming responses

### Sprint 5: Schema Context & Permissions (2-3 days)
1. Implement schema gathering (list tables → list columns → format as DDL)
2. Implement schema caching per source
3. Port risk classification for SQL operations
4. Port permission guard (Ask/Auto modes)
5. Port `tool-confirmation-card.vue` for destructive operations
6. **Verify**: Agent knows schema, respects permissions

### Sprint 6: Polish & Hardening (2-3 days)
1. Large result truncation
2. Query cancellation
3. Auto-compaction
4. Session history browsing
5. Error handling edge cases
6. Multi-source session support
7. **Verify**: End-to-end agent flow works for all 4 database types

---

## 9. System Prompt Architecture

The agent's system prompt is dynamically built per-session:

```
[Core identity]
You are a Data Studio agent in SQLKit, a cross-platform SQL GUI client.

[Source summary]
Attached data sources:
- alias1: POSTGRESQL (permissions: read, create)
- alias2: MYSQL (permissions: read)

[Mode]
Current mode: ASK / AUTO

[Database-specific knowledge]
PostgreSQL knowledge:
- Uses schemas (public by default)
- || for string concat
- LIMIT/OFFSET for pagination
- RETURNING clause...

[Session-wide rules]
- Never fabricate data
- Explain reasoning before executing
- Be concise, no filler or emojis

[Optional: Database schema]
Database Schema:
CREATE TABLE public.users (
  id SERIAL PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  email VARCHAR(255) UNIQUE,
  created_at TIMESTAMP DEFAULT NOW()
);
...
```

---

## 10. Risks & Mitigations

| Risk | Mitigation |
|---|---|
| LLM hallucinates SQL syntax | Inject dynamic schema context; DB-specific knowledge in system prompt |
| Destructive queries executed by mistake | Permission system (Ask/Auto) + risk classification; tool confirmation card |
| Large result sets overflow context | Auto-truncation at tool output; configurable row/byte limits |
| Token budget exhausted mid-conversation | Auto-compaction; context usage monitoring |
| Provider API instability | Retry with jitter; rate limit handling; graceful error messages |
| SQL dialect differences between dbs | Separate capability per db type; DB-specific knowledge blocks in prompt |
| Connection lost mid-session | Reconnection on tool execution; clear error messages |

---

## 11. Success Criteria

- [ ] User can attach a PostgreSQL/MySQL/SQL Server/SQLite connection to a Data Studio session
- [ ] Agent lists tables, views schema, and answers questions about the database
- [ ] Agent generates and executes correct SQL queries
- [ ] Destructive operations require user confirmation in Ask mode
- [ ] Large results are truncated safely
- [ ] Session history is persisted and browsable
- [ ] Context compaction works for long conversations
- [ ] Multiple database sources in one session work correctly
- [ ] All 4 current database types are supported
- [ ] `cargo build` passes with no warnings
- [ ] `npm run build` passes with no errors
