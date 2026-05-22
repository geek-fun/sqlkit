# Transfer — Design Blueprint

> Status: design-only. Implementation tracked in branch `feat/transfer-redesign-scope-first`.
> Supersedes `docs/TRANSFER_DESIGN.md` (deleted — the 4-tab Export/Import/Structure/Migration page model is retired).

## 1. Why this redesign

The previous Transfer page forced every workflow through a top-level page with four sibling tabs. That model has three real-world problems:

1. **Wrong entry point.** Users live in the database browser. When they decide to export a table or back up a database, the natural gesture is right-click on the object — not "navigate to a separate page, pick the tool, then re-pick the object." The existing UI inverts the user's mental model.
2. **No whole-server scope.** Real DBAs migrate *servers*, not single tables. They want "back up everything on this PostgreSQL instance" or "copy this MySQL server to staging," covering many databases at once. The current Migration tab only handles single-database → single-database.
3. **Blocking long operations.** Exporting 50M rows or migrating a 200GB database takes minutes to hours. The current task panel is a modal-ish slide-out; users feel trapped. Long-running work should run in the background with a persistent, non-blocking surface they can collapse, ignore, and return to.

The redesign fixes all three by moving from a **page-first** model to a **scope-first** model.

## 2. Core mental model

> **Scope × Action × Surface**

| Axis | Values |
|---|---|
| **Scope** | `table` · `database` · `server` |
| **Action** | `export` · `import` · `backup` · `restore` · `migrate` · `run-sql` · `generate-ddl` |
| **Surface** | `modal` · `sheet` · `full-page` · `inline-drawer` |

The surface is chosen by the *weight* of the action, not by the developer's convenience:

| Weight | Examples | Surface |
|---|---|---|
| Light, one-off | Quick CSV export of one table | Modal |
| Medium, multi-step | Back up one database; migrate one DB | Right-side Sheet |
| Heavy, multi-object | Back up whole server; migrate server | Full Page (with tree) |
| Async progress | Any running job | Persistent bottom **ActivityDrawer** |

## 3. Entry points

### 3.1 Primary: right-click in `DatabaseBrowser`

The database browser already has tree nodes for server → database → schema → table. The right-click menu gains scope-appropriate transfer entries:

```
[Server node]               [Database node]              [Table node]
─────────────               ───────────────              ────────────
Connect / Disconnect        Open in new tab              Open table view
─────────────               ───────────────              ────────────
Back up server…             Back up database…            Quick export…
Migrate server…             Migrate database…            Quick import…
Run SQL file…               Run SQL file…                Generate DDL
                            Generate DDL                 ─────────────
─────────────               ─────────────                Copy table to…
Refresh                     Refresh                      ─────────────
                                                         Refresh
```

All entries open the appropriate surface (modal/sheet/page) preloaded with that scope.

### 3.2 Secondary: Activity Center (`/transfer` route)

The old `/transfer` page becomes the **Activity Center**: a dashboard of running jobs, recent history, and saved profiles. No wizards live here — wizards launch from the right-click entry points. The Activity Center is where you go to see *what is happening*, not to *start something*.

### 3.3 Tertiary: ActivityDrawer

Persistent collapsible drawer pinned to the bottom of the app shell (VSCode/Chromium download-tray pattern). Visible whenever ≥1 job is running. Collapsing it leaves a 32px status bar showing aggregate progress ("2 jobs · 47%"). Clicking expands to per-job rows with progress, ETA, cancel, and "Show in Activity Center."

## 4. Surfaces — detailed

### 4.1 `QuickExportModal` (table scope, light)

- Trigger: right-click table → "Quick export…"
- 1 dialog, 1 click to confirm.
- Defaults: file format inferred from extension, UTF-8, comma delimiter, include header, all columns, no row limit.
- Advanced disclosure: format options, column subset, WHERE clause, chunk size.
- Submits → job enters ActivityDrawer; modal closes immediately.

### 4.2 `BackupDatabaseSheet` (database scope, medium)

- Trigger: right-click database → "Back up database…"
- Right-side sheet (40% viewport). Reuses existing `ExportWizard` step components but scoped to "all tables in this DB."
- Steps: object selection (default = all tables) → format & options → destination → review.
- Submits → ActivityDrawer; sheet closes.

### 4.3 `BackupServerPage` (server scope, heavy)

- Trigger: right-click server → "Back up server…"
- Full-page route `/transfer/backup/:serverId`.
- Layout: left = `ServerObjectTree` (tristate selection of DBs/schemas/tables), right = wizard pane (format, destination, parallelism, review).
- Submits → ActivityDrawer; route navigates back to Activity Center.

### 4.4 `MigrateDatabaseSheet` / `MigrateServerPage`

Same pattern as backup, but with an additional "target connection" step. Reuses `MigrationWizard` step components from PR #52.

### 4.5 `ActivityDrawer` (global)

- Mounted in `AppLayout`, below main content, above status bar.
- Three states: hidden (0 jobs), collapsed (32px bar), expanded (240px).
- Per-job row: name, scope badge, progress bar, ETA ("about 3 minutes"), cancel, "Open."
- On job completion: row goes green for 5s, then auto-dismisses unless user pinned it. OS notification if app is unfocused.

### 4.6 `TransferPage` → Activity Center

- Three sections:
  - **Active jobs** (mirrors ActivityDrawer, larger)
  - **History** (last 50 completed jobs, filter by scope/action/connection)
  - **Saved profiles** (reusable Backup/Migrate configurations — Phase 1 foundation, Phase 2 polish)

## 5. ServerObjectTree component

A reusable tree with tristate checkboxes:

```
☐ pg-production                    (server)
  ☐ analytics                      (database)
    ☑ public                       (schema, all-checked)
      ☑ events
      ☑ users
    ◐ reporting                    (schema, some-checked)
      ☑ daily_rollup
      ☐ weekly_rollup
  ☐ ops
```

- Lazy-loads children (databases on server expand, schemas on DB expand, tables on schema expand) via existing `list_databases` / `list_schemas` / `list_tables` Tauri cmds.
- Emits `selection: { serverId, databases: string[], schemas: Record<db, string[]>, tables: Record<db.schema, string[]> }`.
- Respects existing `useConnectionStore` for active connection state.

## 6. Backend additions

New Tauri commands in `src-tauri/src/commands/transfer.rs`:

| Command | Args | Returns | Notes |
|---|---|---|---|
| `backup_server` | `connection_id`, `selection`, `format`, `destination`, `options` | `job_id` | Iterates DBs serially, streams per-table via existing export path. |
| `migrate_server` | `source_connection_id`, `target_connection_id`, `selection`, `mapping`, `options` | `job_id` | Creates target DBs if absent; runs migration per DB. |
| `save_transfer_profile` | `profile: TransferProfile` | `profile_id` | Persists to app config (encrypted if it contains credentials). |
| `list_transfer_profiles` | — | `TransferProfile[]` | |
| `run_transfer_profile` | `profile_id` | `job_id` | Loads profile, dispatches to the right command. |

All emit progress events on the existing `transfer://progress/{job_id}` channel; the frontend `transferStore` already has subscription infrastructure.

`TransferProfile` schema (Rust + TS mirror):

```ts
type TransferProfile = {
  id: string
  name: string
  kind: 'backup' | 'migrate' | 'export' | 'import'
  scope: 'table' | 'database' | 'server'
  connectionId: string
  targetConnectionId?: string         // migrate only
  selection: ObjectSelection
  format?: ExportFormat
  destination?: string                // file path or "ask-each-time"
  options: Record<string, unknown>
  createdAt: number
  lastRunAt?: number
}
```

## 7. Frontend store extensions

`useTransferStore`:

```ts
type TransferJob = {
  id: string
  name: string
  kind: TransferProfile['kind']
  scope: TransferProfile['scope']
  connectionId: string
  status: 'queued' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled'
  progress: { stage: string; current: number; total: number; etaMs?: number }
  startedAt: number
  finishedAt?: number
  error?: string
}

// New state
jobs: TransferJob[]
savedProfiles: TransferProfile[]

// New actions
startBackupServer(args)
startMigrateServer(args)
saveProfile(profile)
runProfile(profileId)
cancelJob(jobId)
dismissJob(jobId)
```

The existing per-task subscription pattern (4 listeners) is generalized into one `subscribeToJob(jobId)` that the store calls on `startX*`.

## 8. i18n

New key namespace `transfer.*`:

- `transfer.scope.{table,database,server}`
- `transfer.action.{export,import,backup,restore,migrate,runSql,generateDdl}`
- `transfer.surface.quickExport.*`, `transfer.surface.backupDatabase.*`, etc.
- `transfer.drawer.{collapsed,expanded,jobsRunning,about_n_minutes}`
- `transfer.tree.{server,database,schema,table,selectAll,clearAll}`

Both `src/lang/enUS.ts` and `src/lang/zhCN.ts` must ship at parity.

## 9. Cross-platform

Per `AGENTS.md`:
- Right-click menu = `contextmenu` event (already platform-correct).
- Any keyboard shortcut shown in the UI uses runtime `navigator.platform` detection (`⌘` on macOS, `Ctrl+` elsewhere).
- File paths produced for default destinations use platform-appropriate separators via Tauri's `path` plugin.
- Line endings in generated SQL/CSV files: `\n` only (consistent with the rest of the codebase).

## 10. What is reused vs. new

**Reused (no changes needed):**
- `ExportWizard`, `ImportWizard`, `MigrationWizard`, `StructureWizard` (step components)
- `WizardStepper`, `TransferStepCard`, `MultiTableSelector`, `TabbedColumnSelector`, `ColumnSelector`, `ProgressPanel`, `ConnectionSelector`, `FileDropZone`, `ResultPanel`, `TableSelector`
- `TaskCard`, `TaskManagerPanel`, `TaskManagerButton` (reskinned inside Activity Center & ActivityDrawer)
- All 10 existing `transfer` Tauri commands

**New:**
- `src/components/transfer/ServerObjectTree.vue`
- `src/components/transfer/ActivityDrawer.vue`
- `src/components/transfer/TransferProfileCard.vue`
- `src/components/transfer/shells/QuickExportModal.vue`
- `src/components/transfer/shells/BackupDatabaseSheet.vue`
- `src/pages/BackupServerPage.vue`
- Backend: 5 new Tauri commands listed in §6

## 11. Phase 1 scope (this PR)

Everything in this document. Specifically:

1. Delete legacy `docs/TRANSFER_DESIGN.md` (done in this commit).
2. Backend: 5 new Tauri commands + profile persistence schema.
3. Frontend: store extensions, `ServerObjectTree`, `ActivityDrawer`, 3 new surface shells (`QuickExportModal`, `BackupDatabaseSheet`, `BackupServerPage`).
4. Right-click integration in `DatabaseBrowser`.
5. `TransferPage` rewritten as Activity Center.
6. Full i18n parity (enUS + zhCN).
7. Full quality gates: `npm run lint:fix`, `npm test`, `cargo fmt`, `cargo clippy`, `cargo test`.
8. Oracle self-review + manual QA matrix.

Profile *editing* UI and *scheduled* runs are explicitly out of scope (Phase 2).

## 12. Open risks

| Risk | Mitigation |
|---|---|
| Whole-server backup can produce huge files. | Stream + chunk; default per-DB file with manifest; expose `--split-size` option. |
| Cancel during multi-DB migration leaves target in partial state. | Each DB is its own transaction boundary; cancel completes the current DB and stops; surfaces "Partial: N of M databases migrated." |
| ActivityDrawer competing with other bottom-pinned UI. | Mount via teleport into `AppLayout`'s reserved bottom slot; document the slot contract. |
| Tristate tree perf on large servers (1000+ tables). | Lazy children, virtualized list inside each schema node, selection-state stored as Sets keyed by composite id. |
| Profile credentials at rest. | Reuse existing `connectionStore` encryption pathway; never persist passwords in plaintext JSON. |

## 13. Manual QA matrix

Before opening PR:

| Scenario | Expected |
|---|---|
| Right-click table → Quick export → CSV | Modal opens preloaded with table; submit → ActivityDrawer shows job → completes → file on disk |
| Right-click database → Back up database | Sheet opens; tree-select 3 of 5 tables; submit → job runs; cancel midway → "Partial" status |
| Right-click server → Back up server (4 DBs, ~30 tables) | Page opens with tree; select all; run; ActivityDrawer aggregates progress; OS notification on completion |
| Migrate database to a fresh target | Source preserved; target tables created; rowcount matches |
| Save backup as profile; close app; reopen; run profile from Activity Center | Profile persisted; run reproduces same output |
| All flows in zh-CN | All strings localized; no `transfer.*` keys leak through |
| Same flows on Windows + Linux | Right-click works; paths use OS separator; shortcuts show `Ctrl+` not `⌘` |
