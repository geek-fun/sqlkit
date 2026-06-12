# Transfer Module Scope-Based Architecture Design

> **Status**: Draft for review
> **Date**: 2026-05-28
> **Author**: Architecture proposal based on user requirements

## Executive Summary

This design introduces a **Scope Selector** pattern across all transfer wizards (Export, Import, Migration, Structure) to provide a unified, simplified experience. The scope determines what level of database objects the operation targets:

- **Server**: Operate on all databases within a connection
- **Database**: Operate on all tables/objects within a specific database
- **Tables**: Operate on specific selected tables (simplified - no column-level selection)

---

## 1. Type Model Changes

### 1.1 New Shared Enum

```typescript
// Frontend: src/types/transfer.ts
export type TransferScope = 'server' | 'database' | 'tables'
```

```rust
// Backend: src-tauri/src/transfer/types.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum TransferScope {
    #[default]
    Tables,
    Database,
    Server,
}
```

### 1.2 Modified Request Types

#### ExportRequest

**Before**:
```typescript
export type ExportRequest = {
  connectionId: string
  database?: string
  schema?: string
  source: ExportSource  // single table
  format: ExportFormat
  outputPath: string
  // ...options
}
```

**After**:
```typescript
export type ExportRequest = {
  scope: TransferScope              // NEW (default: 'tables')
  connectionId: string
  database?: string                 // required for 'database'/'tables' scope
  schema?: string
  sources: ExportSource[]           // CHANGED: array for multi-table
  format: ExportFormat
  outputPath: string                // For 'tables': single file; for 'database/server': directory
  // ...options unchanged
}

export type ExportSource = {
  table: string
  columns: string[]                 // When scope='tables': user-selected; else: all columns
  // whereClause, orderBy, limit removed for simplicity
}
```

#### ImportRequest

**Before**:
```typescript
export type ImportRequest = {
  connectionId: string
  database?: string
  table: string                     // single target table
  filePath: string
  // ...
}
```

**After**:
```typescript
export type ImportRequest = {
  scope: TransferScope              // NEW
  connectionId: string
  database?: string                 // required for 'database'/'tables' scope
  createDatabaseIfNotExists?: boolean // NEW: for 'database' scope
  tables: ImportTarget[]            // CHANGED: array for multi-table
  filePath: string
  // ...
}

export type ImportTarget = {
  sourceTable?: string              // From file (for multi-sheet Excel, multi-table SQL)
  targetTable: string
  columnMappings?: ColumnMapping[]
}
```

#### MigrationRequest

**Before**:
```typescript
export type MigrationRequest = {
  sourceConnectionId: string
  sourceDatabase?: string
  targetConnectionId: string
  targetDatabase?: string
  tablePlans: MigrationTablePlan[]
  // ...
}
```

**After**:
```typescript
export type MigrationRequest = {
  scope: TransferScope              // NEW
  sourceConnectionId: string
  sourceDatabase?: string           // required for 'database'/'tables' scope
  targetConnectionId: string
  targetDatabase?: string
  createTargetDatabaseIfNotExists?: boolean // NEW: for 'database' scope
  tablePlans: MigrationTablePlan[]  // auto-populated for 'server'/'database' scope
  // ...
}
```

#### DdlRequest

**After**:
```typescript
export type DdlRequest = {
  scope: TransferScope              // NEW
  connectionId: string
  database?: string
  objects: DdlObject[]              // auto-populated for 'server'/'database' scope
  options: DdlOptions
}
```

---

## 2. UI Component Changes

### 2.1 New Component: ScopeSelector

**Location**: `src/components/transfer/shared/ScopeSelector.vue`

**Purpose**: Horizontal chip toggle for scope selection

**Props**:
```typescript
defineProps<{
  scope: TransferScope
  disabled?: boolean
}>()

defineEmits<{
  'update:scope': [value: TransferScope]
}>()
```

**Visual Design**:
```
┌─────────────────────────────────────────────┐
│  [Server]  [Database]  [Tables]              │  ← chip buttons
│   gray      gray       primary               │  ← selected styling
└─────────────────────────────────────────────┘
```

**Styling**:
- Horizontal flex container
- Each chip: `px-3 py-1.5 rounded-md text-xs font-medium`
- Selected: `bg-primary/10 text-primary border border-primary/30`
- Unselected: `bg-muted/30 text-muted-foreground hover:bg-muted/50`
- Gap between chips: `gap-1.5`

### 2.2 Modified Component: TransferStepCard

**Current header layout** (line 39-48):
```
[icon] [01] TITLE              summary
```

**Proposed header layout**:
```
[icon] [01] TITLE  [ScopeSelector]  summary
```

**New Props**:
```typescript
defineProps<{
  // ... existing props
  scope?: TransferScope           // NEW
  scopeDisabled?: boolean         // NEW
}>()

defineEmits<{
  'update:scope': [value: TransferScope]  // NEW
}>()
```

**Header slot integration**:
```vue
<div class="px-3 py-2 border-b border-border/40 flex gap-2.5 items-center">
  <!-- existing: icon, stepLabel, title -->
  <ScopeSelector 
    v-if="scope" 
    :scope="scope" 
    :disabled="scopeDisabled"
    @update:scope="$emit('update:scope', $event)"
    class="ml-2" 
  />
  <!-- existing: summary -->
</div>
```

---

## 3. Wizard Flow Changes

### 3.1 ExportWizard (Priority 1)

**Current Steps**:
1. Source (Connection → Database → Schema → Tables → Columns)
2. Format & Output

**New Steps**:
1. Scope + Source (with scope chips in header)
2. Format & Output

**Step 1: Scope + Source**

```
┌─ Step 1: Source ────────────────────────────────────────┐
│ [icon] [01] SOURCE  [Server][Database][Tables]  3 tables │
│                                                         │
│  IF scope === 'server':                                 │
│    • ConnectionSelector only                            │
│    • Badge: "All databases on this server"              │
│    • Summary: Auto-count all tables across all DBs      │
│                                                         │
│  IF scope === 'database':                               │
│    • ConnectionSelector                                 │
│    • DatabaseSelector (required)                        │
│    • Badge: "All tables in {database}"                  │
│    • Summary: Auto-count tables in selected DB          │
│                                                         │
│  IF scope === 'tables':                                 │
│    • ConnectionSelector                                 │
│    • DatabaseSelector                                   │
│    • SchemaSelector (optional)                          │
│    • MultiTableSelector (checkbox grid)                 │
│    • Summary: "N tables selected"                       │
│                                                         │
│  ❌ REMOVE: TabbedColumnSelector (no column selection)  │
└─────────────────────────────────────────────────────────┘
```

**Step 2: Format & Output** (unchanged structure, but conditional output)

```
┌─ Step 2: Format & Output ───────────────────────────────┐
│  Format selector (CSV/JSONL/Excel/SQL)                   │
│  Format-specific options                                 │
│                                                         │
│  IF scope === 'tables':                                 │
│    • Output: Single file path                           │
│                                                         │
│  IF scope === 'database':                               │
│    • Output: Directory path                             │
│    • Filename pattern: {database}_{table}.{ext}         │
│                                                         │
│  IF scope === 'server':                                 │
│    • Output: Directory path                             │
│    • Filename pattern: {database}/{table}.{ext}         │
│    • Creates subdirectory per database                  │
└─────────────────────────────────────────────────────────┘
```

### 3.2 MigrationWizard (Priority 2)

**Current Steps**:
1. Source Connection + Table Selection
2. Target Connection + Options
3. Preview & Execute

**New Steps** (same count, scope in Step 1 header):

```
┌─ Step 1: Scope + Source ────────────────────────────────┐
│ [01] SOURCE  [Server][Database][Tables]                  │
│                                                         │
│  IF scope === 'server':                                 │
│    • Source ConnectionSelector                          │
│    • Badge: "All databases will be migrated"            │
│                                                         │
│  IF scope === 'database':                               │
│    • Source ConnectionSelector + Database               │
│    • Badge: "All tables in {database}"                  │
│                                                         │
│  IF scope === 'tables':                                 │
│    • Current behavior: multi-table checkbox grid        │
└─────────────────────────────────────────────────────────┘

┌─ Step 2: Target ────────────────────────────────────────┐
│  Target ConnectionSelector                              │
│                                                         │
│  IF scope === 'server':                                 │
│    • No target database selector                        │
│    • Option: "Create databases if not exist"            │
│                                                         │
│  IF scope === 'database':                               │
│    • Target database selector                           │
│    • Option: "Create database if not exist" ✓           │
│                                                         │
│  IF scope === 'tables':                                 │
│    • Current behavior                                   │
└─────────────────────────────────────────────────────────┘

┌─ Step 3: Preview & Execute ────────────────────────────┐
│  (unchanged)                                             │
└─────────────────────────────────────────────────────────┘
```

### 3.3 StructureWizard (Priority 3)

**GenerateDdl sub-tab**:

```
┌─ Step 1: Scope + Source ────────────────────────────────┐
│ [01] SOURCE  [Server][Database][Tables]                  │
│                                                         │
│  IF scope === 'server':                                 │
│    • ConnectionSelector only                            │
│    • Auto-select: all databases, all objects            │
│                                                         │
│  IF scope === 'database':                               │
│    • ConnectionSelector + Database                      │
│    • Auto-select: all objects in DB                     │
│                                                         │
│  IF scope === 'tables':                                 │
│    • Current object checkbox grid                       │
└─────────────────────────────────────────────────────────┘
```

**RunSqlFile sub-tab**:

```
┌─ Step 1: Scope + Target ────────────────────────────────┐
│ [01] TARGET  [Server][Database][Tables]                  │
│                                                         │
│  IF scope === 'server':                                 │
│    • ConnectionSelector only                            │
│    • SQL can CREATE DATABASE                            │
│                                                         │
│  IF scope === 'database':                               │
│    • ConnectionSelector + Database                      │
│    • Option: "Create database if not exist"             │
│                                                         │
│  IF scope === 'tables':                                 │
│    • ConnectionSelector + Database                      │
│    • SQL targets specific tables                        │
└─────────────────────────────────────────────────────────┘
```

### 3.4 ImportWizard (Priority 4)

**Current Steps**:
1. Source File
2. Target & Mapping
3. Options & Execute

**New Steps** (scope in Step 2 header):

```
┌─ Step 1: Source File ───────────────────────────────────┐
│  (unchanged - file drop, detection, preview)            │
└─────────────────────────────────────────────────────────┘

┌─ Step 2: Scope + Target ────────────────────────────────┐
│ [02] TARGET  [Server][Database][Tables]                  │
│                                                         │
│  IF scope === 'server':                                 │
│    • ConnectionSelector only                            │
│    • SQL file can CREATE DATABASE                       │
│    • No table mapping UI                                │
│                                                         │
│  IF scope === 'database':                               │
│    • ConnectionSelector + Database                      │
│    • Checkbox: "Create database if not exists" ✓        │
│    • Auto-create tables from file structure             │
│    • No manual column mapping                           │
│                                                         │
│  IF scope === 'tables':                                 │
│    • Current behavior: single table + column mapping    │
└─────────────────────────────────────────────────────────┘

┌─ Step 3: Options & Execute ────────────────────────────┐
│  (unchanged)                                             │
└─────────────────────────────────────────────────────────┘
```

---

## 4. Backend Command Changes

### 4.1 execute_export_data

**Current signature**:
```rust
pub async fn execute_export_data(
    request: ExportRequest,
    app_state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<TransferResult, String>
```

**Changes needed**:

1. **Handle `scope` field**:
```rust
match request.scope {
    TransferScope::Server => {
        // 1. List all databases
        let databases = adapter.list_databases()?;
        // 2. For each database, list tables
        // 3. Export each table to: {output_path}/{database}/{table}.{ext}
    }
    TransferScope::Database => {
        // 1. List all tables in request.database
        // 2. Export each table to: {output_path}/{database}_{table}.{ext}
    }
    TransferScope::Tables => {
        // Current behavior: iterate request.sources
        // Export each table to: {output_path}
    }
}
```

2. **Multi-table support**:
```rust
// Change from single source to sources array
for source in request.sources.iter() {
    export_table(adapter, source, &request.format, output_path)?;
}
```

3. **Progress events**: Include current database/table in progress for server/database scope

### 4.2 execute_import_data

**Changes needed**:

1. **Handle `scope` field**:
```rust
match request.scope {
    TransferScope::Server => {
        // Execute SQL file directly at connection level
        // File may contain CREATE DATABASE statements
    }
    TransferScope::Database => {
        // 1. Check if database exists, create if request.create_database_if_not_exists
        // 2. Auto-create tables from file structure (for CSV/Excel)
        // 3. Import data
    }
    TransferScope::Tables => {
        // Current behavior
    }
}
```

### 4.3 execute_migration_data

**Changes needed**:

1. **Handle `scope` field**:
```rust
match request.scope {
    TransferScope::Server => {
        // Migrate all databases from source to target
        // Auto-create target databases
    }
    TransferScope::Database => {
        // 1. Check/create target database if request.create_target_database_if_not_exists
        // 2. Migrate all tables
    }
    TransferScope::Tables => {
        // Current behavior
    }
}
```

### 4.4 generate_ddl_for_objects

**Changes needed**:

1. **Handle `scope` field**:
```rust
match request.scope {
    TransferScope::Server => {
        // Generate DDL for all databases, all objects
        // Output: multiple DDL files per database
    }
    TransferScope::Database => {
        // Generate DDL for all objects in database
    }
    TransferScope::Tables => {
        // Current behavior
    }
}
```

---

## 5. Visual Wireframe

### ExportWizard with Scope Selector

```
┌─────────────────────────────────────────────────────────────────────┐
│ TRANSFER                                              [Task Manager] │
│ Data import, export, and migration                                   │
├─────────────────────────────────────────────────────────────────────┤
│ [Export] [Import] [Migration] [Structure]                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│ ┌─ Step 1: Source ───────────────────────────────────────────────┐ │
│ │ [📊] [01] SOURCE  [Server][Database][Tables]  3 tables         │ │
│ │                                                                │ │
│ │  ┌────────────────────┐  ┌──────────────────────────────────┐ │ │
│ │  │ Connection         │  │ Tables                           │ │ │
│ │  │ ┌────────────────┐ │  │ ┌──────────────────────────────┐ │ │ │
│ │  │ │ localhost:5432 │ │  │ │ ☑ users          1,234 rows │ │ │ │
│ │  │ └────────────────┘ │  │ │ ☐ products       5,678 rows │ │ │ │
│ │  │                    │ │  │ │ ☐ orders         2,345 rows │ │ │ │
│ │  │ Database           │ │  │ │ ...                          │ │ │ │
│ │  │ ┌────────────────┐ │  │ │ └─────────────────────────── │ │ │ │
│ │  │ │ mydb           │ │  │ │ [Select All] [Deselect All]   │ │ │ │
│ │  │ └────────────────┘ │  │ └──────────────────────────────┘ │ │ │
│ │  │                    │ │                                  │ │ │
│ │  │ Schema (optional)  │ │                                  │ │ │
│ │  │ ┌────────────────┐ │ │                                  │ │ │
│ │  │ │ public         │ │ │                                  │ │ │
│ │  │ └────────────────┘ │ │                                  │ │ │
│ │  └────────────────────┘ │                                  │ │ │
│ │ └────────────────────────────────────────────────────────────┘ │
│ │                                                                │
│ └────────────────────────────────────────────────────────────────│ │
│                                                                      │
│ ┌─ Step 2: Format & Output ──────────────────────────────────────┐ │
│ │ [📄] [02] FORMAT & OUTPUT                                      │ │
│ │                                                                │ │
│ │  Format:                                                       │ │
│ │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │ │
│ │  │[CSV]     │ │ JSONL    │ │ Excel    │ │ SQL      │          │ │
│ │  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │ │
│ │                                                                │ │
│ │  CSV Options:                                                  │ │
│ │  Delimiter: [Comma (,) ▼]   ☑ Include header row              │ │
│ │                                                                │ │
│ │  Output Path:                                                  │ │
│ │  [/path/to/output.csv                  ] [Browse]              │ │
│ │                                                                │ │
│ │  ──────────────────────────────────────────────────────────── │ │
│ │                                            [▶ Start Export]    │ │
│ │                                                                │ │
│ │  Summary: 3 tables | 4 cols | CSV                              │ │
│ └────────────────────────────────────────────────────────────────│ │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 6. Implementation Checklist

### Phase 1: Types & Components (Frontend + Backend)

**Frontend**:
1. Add `TransferScope` to `src/types/transfer.ts`
2. Update `ExportRequest` (add `scope`, change `source` → `sources: ExportSource[]`)
3. Update `ImportRequest` (add `scope`, `createDatabaseIfNotExists`)
4. Update `MigrationRequest` (add `scope`, `createTargetDatabaseIfNotExists`)
5. Update `DdlRequest` (add `scope`)

**Backend**:
1. Add `TransferScope` to `src-tauri/src/transfer/types.rs`
2. Update corresponding Rust request structs
3. Add timestamp generation helper function

**Shared Components**:
1. Create `ScopeSelector.vue` component (horizontal chip toggle)
2. Modify `TransferStepCard.vue` to accept and render scope selector in header

### Phase 2: ExportWizard (Frontend)

1. Add scope state (default: `'tables'`)
2. Pass scope to TransferStepCard Step 1 header
3. Conditionally show/hide selectors based on scope:
   - `server`: ConnectionSelector only, show summary badge
   - `database`: ConnectionSelector + DatabaseSelector, show summary badge
   - `tables`: ConnectionSelector + DatabaseSelector + MultiTableSelector
4. Remove `TabbedColumnSelector` usage
5. Change `source` to `sources` array in store sync
6. Wire `startExport()`:
   - For `tables` scope: direct invoke with single/multiple sources
   - For `database`/`server` scope: async task creation, poll for status
7. Handle output path:
   - `tables` (single): file picker for `.csv/.sql/.xlsx`
   - `tables` (multi): file picker for `.zip`
   - `database`/`server`: directory picker (create ZIP inside)

### Phase 3: Export Backend

1. Update `execute_export_data` command signature
2. Add timestamp generator: `format_datetime(chrono::Local::now())`
3. Implement scope-based iteration:
   - `server`: List databases → for each DB → list tables → export to ZIP nested
   - `database`: List tables → export each to ZIP
   - `tables`: Direct multi-table export to ZIP or single file
4. ZIP creation logic (use `zip` crate):
   - Server scope: nested entries `{database}/{table}.{ext}`
   - Database scope: flat entries `{table}.{ext}`
5. Progress events: Include `current_database`, `current_table`, `total_tables`
6. Async task pattern:
   - For `database`/`server` scope: Return task ID immediately
   - Background thread processes export
   - Client polls `/task_status/{taskId}` (reuse existing BackgroundTask system)

### Phase 4: MigrationWizard + Backend

1. Add scope state to MigrationWizard (default: `'tables'`)
2. Conditionally show selectors:
   - `server`: Source + Target ConnectionSelector, no DB selector
   - `database`: ConnectionSelector + DatabaseSelector for both source/target
   - `tables`: Current behavior
3. Add `createTargetDatabaseIfNotExists` checkbox for `database` scope
4. Update `execute_migration_data`:
   - For `server` scope: Iterate all databases
   - For `database` scope: Single DB migration (all tables or selected)
5. Handle target database creation

### Phase 5: StructureWizard + Backend

1. Add scope to GenerateDdl:
   - `server`: Export DDL for all databases (ZIP output)
   - `database`: Export DDL for all objects (single SQL or ZIP)
   - `tables`: Current behavior (selected objects)
2. Add scope to RunSqlFile:
   - `server`: SQL can CREATE DATABASE
   - `database`: Add `createDatabaseIfNotExists` checkbox
   - `tables`: Current behavior
3. Update backend commands accordingly

### Phase 6: ImportWizard + Backend

1. Add scope state to ImportWizard (default: `'tables'`)
2. Scope affects Step 2 (Target & Mapping):
   - `server`: ConnectionSelector only, SQL file can CREATE DATABASE
   - `database`: ConnectionSelector + DatabaseSelector, add `createDatabaseIfNotExists` checkbox, auto-create table from file
   - `tables`: Current behavior (select target table + column mapping)
3. For `database` scope: Single file imports to ONE table (match Chat2DB)
4. Update `execute_import_data` for scope handling

---

## 7. Design Decisions (Based on Chat2DB Research)

> Reference: Chat2DB GitHub - https://github.com/codePhiliaX/Chat2DB

### 7.1 Output Naming Convention (Adopt Chat2DB's Pattern)

| Scope | Single Table | Multiple Tables |
|-------|--------------|-----------------|
| **Tables** | `{tableName}_{timestamp}.{ext}` | `export_{tables}_data_{timestamp}.zip` → `{tableName}.{ext}` inside |
| **Database** | — | `export_{databaseName}_data_{timestamp}.zip` → `{tableName}.{ext}` inside |
| **Server** | — | `export_{connectionName}_data_{timestamp}.zip` → `{database}/{tableName}.{ext}` nested |

**Timestamp format**: `YYYYMMDDHHmmss` (pure datetime, matches Chat2DB)

**Examples**:
```
# Tables scope (single)
users_20240324153045.csv

# Tables scope (multiple: users, orders)
export_users_orders_data_20240324153045.zip
  → users.csv
  → orders.csv

# Database scope (mydb)
export_mydb_data_20240324153045.zip
  → users.csv
  → orders.csv
  → products.csv

# Server scope (localhost_5432)
export_localhost_5432_data_20240324153045.zip
  → mydb/
    → users.csv
    → orders.csv
  → testdb/
    → test_table.csv
```

### 7.2 Import Database Scope (Match Chat2DB's Approach)

**Decision**: Single file → creates ONE target table (same as Chat2DB)

- For CSV/JSONL/Excel: User selects target database, file imports to one table
- For SQL files: Backend can auto-detect multiple CREATE TABLE statements and create accordingly
- The "create database if not exists" checkbox applies to the target database selection, not file parsing

**Why**: Chat2DB has no multi-table import. This simplifies UX and aligns with common patterns.

### 7.3 Scope Default

**Decision**: `tables` scope for all wizards (default)

- Safest (current behavior)
- Most common use case
- Matches Chat2DB's single-table focus

### 7.4 Scope Persistence

**Decision**: No persistence needed

- Scope is per-operation (not remembered)
- Each wizard starts with `tables` scope
- Simpler implementation
- Matches Chat2DB's implicit scope approach (no state)

### 7.5 Async Task Pattern for Bulk Operations

**Decision**: Adopt async task pattern for database/server scope (like Chat2DB)

- Database/server scope exports: Return task ID, poll for status
- Tables scope: Direct execution (smaller scope, faster)
- Use existing `BackgroundTask` system in `transferStore`

### 7.6 Scope UI Approach (Keep Explicit Picker)

**Decision**: Keep explicit scope selector (unlike Chat2DB's implicit approach)

**Why SQLKit differs from Chat2DB**:
- SQLKit has a dedicated Transfer page (not tree-context driven)
- Better discoverability for users unfamiliar with database hierarchy
- Consistent experience across Export/Import/Migration/Structure tabs
- Chat2DB's approach works for tree-based UI; SQLKit's wizard-based UI needs explicit selection

---

## 8. Acceptance Criteria

- [ ] Scope selector appears in header of Step 1 for all wizards
- [ ] Scope selector uses chip toggle styling (horizontal, 3 options: Server/Database/Tables)
- [ ] `tables` scope is default for all wizards
- [ ] Scope is not persisted (resets to `tables` on wizard open)
- [ ] Selectors conditionally render based on selected scope
- [ ] `tables` scope behaves like current behavior (minus column selection for Export)
- [ ] `database` scope auto-selects all tables, shows count summary badge
- [ ] `server` scope auto-selects all databases, shows count summary badge
- [ ] **Output naming follows Chat2DB pattern**:
  - [ ] Single table: `{tableName}_{timestamp}.{ext}`
  - [ ] Multiple tables: ZIP with `{tableName}.{ext}` inside
  - [ ] Database scope: ZIP with `{tableName}.{ext}` inside
  - [ ] Server scope: ZIP nested with `{database}/{tableName}.{ext}`
- [ ] Backend handles all 3 scope levels correctly
- [ ] **Async task pattern** for database/server scope exports:
  - [ ] Returns task ID immediately
  - [ ] Background thread processes export
  - [ ] Client can poll for status via BackgroundTask system
- [ ] Progress events include `current_database`, `current_table`, `total_tables` for bulk scopes
- [ ] Import `database` scope: Single file imports to one table, with `createDatabaseIfNotExists` option
- [ ] Migration `database` scope: Has `createTargetDatabaseIfNotExists` option
- [ ] Structure `database` scope: Auto-selects all objects for DDL generation

---

## Next Steps

Please review and provide feedback on:
1. Type model changes (section 1)
2. UI wireframe (section 5)
3. Open questions (section 7)
4. Any additional requirements or concerns

Once approved, implementation will proceed in priority order:
Export → Migration → Structure → Import