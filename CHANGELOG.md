# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **DuckDB failure diagnosis and file-format reporting** — DuckDB connection failures are translated into an explanation with the fix attached: file lock held by another process or connection, instance-configuration conflict, and storage-format incompatibility each get their own message, and the two lock cases offer **retry as read-only** inline in the connection form (the connecting modal shows the same explanation on connect). `probe_database_file` additionally reports the DuckDB storage-format version (header bytes 12..20, mapped to the oldest release that can open the file) and whether a sibling `.wal` write-ahead log is present, both shown under the file path.

- **Create DuckDB / SQLite databases from the connection form** — the file-based path field is editable again and gains `New…` / `Open…` actions backed by the native save dialog (extension appended automatically), an inline explanation of what will happen on connect (the engine creates a missing file, the file exists, is empty, is read-only, is a folder, is a different engine's file, or the parent folder is missing with a one-click "Create folder"), and a per-engine recent-files list with reveal/remove actions. DuckDB also gets the SQLite-style **File / In-Memory** tabs; in-memory connections are stored as named instances (`memory:<label>`) because unnamed DuckDB in-memory databases are private to a single connection and would otherwise give every pooled connection its own empty database. A new **read-only** access mode (`access_mode=READ_ONLY`) is meant to let DuckDB share a file with other processes (`~` is expanded for file-based paths, and opening the same DuckDB file a second time with a conflicting access mode is blocked with "switch to that connection" / "open read-only" choices — read-only needs a bridge build with the HikariCP flag alignment below, and the form explains that until it ships). New Tauri commands: `probe_database_file` (existence, size, writability, header sniffing) and `create_database_directory`. `scripts/verify-duckdb-jdbc-urls.sh` records the DuckDB JDBC semantics these decisions rely on.

- **Entitlements & version-lock subscription model (geekfun#56)** — client-side implementation of the Ultimate entitlement contract: the two server-computed fields `ultimateExpiresAt` + `versionLockHorizon` are consumed via a new Rust entitlement module (persisted cache, offline tolerance, failure degradation, 5-minute refresh throttle, `app.releaseDate <= versionLockHorizon` unlock check). Rust command gates return `ENTITLEMENT_REQUIRED` for AI (agent loop/step, compaction, LLM validation), the whole Transfer module (import/export/migration/structure execution), SSH tunnel establishment, and the MCP bridge (config/policy save + auto-start). The frontend adds an entitlement store, Geekfun login entry, paid-feature gates with upgrade guidance on Data Studio, Transfer, AI assistant sidebar, ER diagram actions, AI/MCP settings, SSH tunnel option in the connection form, plus an Account & Plan settings tab showing the version-lock state with Geekfun login and logout (logout clears the local entitlement cache so entitlements never outlive the account session).
- Requesty provider preset in AI settings (OpenAI compatible, base URL `https://router.requesty.ai/v1`)
- Opper provider preset in AI settings (OpenAI compatible, base URL `https://api.opper.ai/v3/compat`)

### Changed

- **JDBC connections are much faster (DuckDB "Test connection" above all)** — measured per connect before: an Adoptium JRE update check (1.4-2.5 s, on *every* connect), two Maven Central metadata round trips (0.6-0.8 s each), a fresh bridge JVM (DuckDB maps a ~107 MB native library, ~1.4 s) and a new driver class loader per connection. Now: the JRE check is memoised for 6 hours, a cached driver JAR skips the Maven resolution on the Rust side (the Java side follows in the next bridge build), bridge downloads use a shared HTTP client with a bounded connect timeout instead of `reqwest::get` (which had none), the bridge process is shared across connections (`shared_launcher`), and a connection releases only its own pool through the `disconnect` RPC instead of killing the process. Verified with `src-tauri/tests/duckdb_bridge_integration.rs`: a warm connect went from ~4.3 s to **5.5 ms** with a bridge build that also caches the driver class loader; with the currently released bridge the same test measures ~1.4 s per warm connect.


### Fixed

- **DuckDB read-only mode is rejected by the released JDBC bridge** — DuckDB refuses `Connection.setReadOnly()`, and HikariCP calls it whenever its own read-only flag disagrees with the opened connection (it defaults to false), so a read-only URL failed with `Can't change read-only status on connection level`. The bridge now derives HikariCP's flag from the URL (`ConnectionManager.isReadOnlyUrl`); until a bridge build with that change ships, the connection form explains the limitation and offers **retry as read-write**. Verified against a patched bridge in `src-tauri/tests/duckdb_bridge_integration.rs`.

- **Untranslated keys could render in the UI** — the new-connection menu displayed `pages.connections.newMenu.*` because those strings were added under the wrong node of the locale files; `src/__tests__/i18nKeys.test.ts` now scans every literal `t('…')` in `src/` and fails when a key does not resolve in `enUS` or `zhCN`, so this class of regression is caught by `npm test` instead of by a screenshot. The same scan surfaced two latent gaps, both fixed: `common.cancel` (used by the device-replace dialog, now the existing `common.buttons.cancel`) and the missing `zhCN` value for `components.serverForm.placeholders.encryptionKey`.

- **DuckDB database files whose path contains `;` were mis-parsed** — the DuckDB JDBC driver splits its URL on `;`, so `jdbc:duckdb:/path/semi;colon.duckdb` failed with `Invalid URL entry` (verified against `duckdb_jdbc` 1.5.6); such paths are now rejected with an explanation in the connection form and in the backend instead of surfacing a confusing driver error, and read-only mode is never silently dropped for them.

- **DuckDB connections fail with "The following options were not recognized: ssl" (issue #158)** — embedded/file-based engines (DuckDB, SQLite, SQLCipher, Access) have no SSL settings in the UI, so their stored ssl mode is just the `Prefer` default; the JDBC bridge now receives no ssl mode for such types instead of injecting `ssl=true`, which DuckDB's driver rejects as an unknown option; the bridge's property mapper also stops blindly setting `ssl=true` for drivers without explicit SSL support (H2 and Firebird previously fell into that fallback).

- **Deep-link sign-in on cold start** — a `sqlkit://auth` link that launches the app no longer drops the token: the payload is parked in a pending-auth slot during startup and the frontend pulls it via `consume_pending_auth` once mounted; running-instance links double-write the same slot, closing the startup window too.

## [0.8.7] - 2026-08-17

### Added

- DBA capabilities for session, slow-query, and privilege management (#138)

### Fixed

- Use Pinia appStore as the single theme source for the editor (#137)

## [0.8.6] - 2026-08-11

### Added

- Guide agents to sqlkit tools for database access (#134)
- Advertise a discovery tool per connection in `list_connections` (#135)

### Changed

- Bump data-studio-agent to v0.1.6

## [0.8.5] - 2026-08-08

### Added

- Editable query results and empty SELECT rendering (#133)

### Fixed

- Resolve saved-but-inactive connections in the MCP bridge (#132)

## [0.8.4] - 2026-08-07

### Added

- Schema-aware autocomplete with dialect switching (#131)
- Primary button opens editor when connected
- Toggle connect button label/icon by connection state
- Disconnect action for connected connection cards
- Database context menu with dismiss on outside click

### Fixed

- Use power icon for close connection action
- Label the action 'Close Connection' instead of 'Disconnect'
- Scrollable edit dialogs for wide tables
- Generate executable top-N SQL per database dialect
- Transparently reconnect lost or evicted sessions

## [0.8.2] - 2026-07-30

### Added

- Improve default export filename with connection-db-schema-table-timestamp
- Soft-split SQL statements at 2+ blank lines when `;` is missing

### Fixed

- Match refresh button style to other toolbar icons (ghost)
- Properly separate refresh button from columns dropdown (was nested)
- Change refresh button to outline variant so it's visible
- Use correct icon for refresh button (i-carbon-renew)
- Make table view refresh button visible with text label
- Align SQL execute gutter icon to actual statement start line (#123)

### Changed

- Move search/filter icons inside input boxes, remove external icons and separator

## [0.8.1] - 2026-07-27

### Fixed

- Fix sidebar navigation buttons not responding to clicks (#122)
- Fix DataGrid row selection triggering on cell click instead of checkbox only (#122)
- Fix DataGrid row actions hidden until hover, now always visible (#122)

### Changed

- Unify result panel icon styles with editor toolbar — semantic colors, consistent sizing, lucide icons (#122)

## [0.8.0] - 2026-07-27

### Added

- Add SQLCipher integration — encrypted SQLite variant with passphrase support (#121)

## [0.7.9] - 2026-07-27

### Fixed

- Fix auto-update restart by registering tauri-plugin-process (#120)

## [0.7.8] - 2026-07-27

### Changed

- Reduce frontend bundle by 57% by limiting Monaco Editor to SQL languages only (#97)

## [0.7.7] - 2026-07-23

### Fixed

- Fix auto-update install failure not visible: enable F12 devtools in release builds, show persistent error in update notification with retry option

## [0.7.6] - 2026-07-23

### Added

- Add GitHub repo link to sidebar

### Fixed

- Fix updater failure on startup
- Fix lint issues

## [0.7.5] - 2026-07-23

### Added

- Add ER diagram with orthogonal routing, zoom controls, focus mode, and info badges
- Add ER diagram button to database toolbar and sidebar
- Wire ER diagram button to open in query tab
- Add backend SQL construction service for ER diagram queries
- Add About page to settings with version, license, and community links

### Fixed

- Fix ER diagram routing (orthogonal, bezier), interaction, layout, and pan behavior
- Fix schema fetching in ER diagram to directly invoke backend
- Fix ER diagram Select empty value and zoom button ordering
- Fix ER diagram node dragging and layout issues
- Restore ER diagram context menu
- Show all schemas in database-level ER diagram
- Fix input border contrast for better visibility
- Fix database selector icon consistency

### Changed

- Refactor ER diagram to single TB layout with Record position tracking
- Replace ER diagram SVG rendering with pure HTML for better performance
- Adopt DBX-style compact toolbar layout
- Refactor sidebar: move ER diagram button to database actions dropdown

## [0.7.4] - 2026-07-01

### Added

- Add floating update notification card with skip version support
- Add success color tokens to CSS design system

### Changed

- Refactor update notification to use CSS variables and module-scoped composable state

## [0.7.3] - 2026-06-25

### Added

- Add sidebar redesign with modular components (ConnectionSelector, DatabaseSelectorRow, SchemaTree, SavedQueriesPanel, SidebarSplitView, TreeGroup)
- Add PostgreSQL materialized view support across backend and frontend
- Add 9 database CRUD actions: Create Database/Schema/Table/View/Function/Procedure, Drop Database, Backup, Export
- Add Create Database dialog with MySQL charset/collation and PostgreSQL encoding/locale options
- Add Create Table visual designer with column grid, type selector, MySQL ENGINE option, SQL preview
- Add MySQL PK detection using INFORMATION_SCHEMA.KEY_COLUMN_USAGE as fallback for charset-safe column reads
- Add Lucide icons, category-specific tree group icons (DBeaver-style)
- Add global thin scrollbar styles
- Add SSL/TLS support across all adapter types including mTLS, self-healing, and bridge forwarding (#113)
- Add OceanBase Oracle as separate JDBC bridge database type
- Add connection resilience layer — health guardian, trait dispatch, LRU cache
- Add SearchableSelect component integrated in connection form
- Add custom window controls with draggable regions

### Fixed

- Fix treeview loading flicker — absolute positioned overlay only during initial fetch
- Fix table view edit/delete on tables without primary key — falls back to all-column identification
- Fix MySQL `list_columns` PK detection — silent `FromValue<String>` failure on INFORMATION_SCHEMA virtual tables
- Fix editor toolbar icons — explain icon now renders (was non-existent `carbon-wand`), each button has distinct color
- Fix DataTableView pagination overflow — min-h-0 + shrink-0
- Fix saved queries layout — compact two-row layout with icon dates, double-click to open
- Fix database switching for MySQL, ClickHouse, JDBC bridge, Rqlite, Turso in get_table_data/get_table_count
- Fix procedure/function DDL display — async fetch via getObjectDdl
- Fix ApiResponse error checking — was silently swallowing all SQL errors (tagged enum, not `success` field)
- Fix CREATE/DROP DATABASE quoting per database type (backticks, double quotes, square brackets)
- Fix crash on NULL values and hanging connections with OceanBase
- Fix http-adapters to respect connect_timeout_secs config
- Fix MySQL list_schemas regression — return only requested database
- Fix connection P0 connect timeout and P1 evict_idle logic bug
- Fix agent single-tool hang, JDBC bridge timeouts, connection loss during long queries
- Fix SSL blacklist after review — three-tier approach with exhaustive type mapping
- Fix connection store types, i18n, tests, and YashanDB formatter

### Changed

- Refactor sidebar from monolithic DatabaseBrowser.vue to 11 focused, composable components
- Replace Carbon icons with Lucide in treeview
- Upgrade to Tauri v2 with enhanced IPC and window management
- Consolidate DM8 and DM8Oracle into single Dameng database type
- Enhance Oracle connection form with 3 connection methods and JRE download
- Enrich ServerCard with version badges, strategy icons, Oracle URL

## [0.7.2] - 2026-06-21

### Added

- Consolidate DM8 and DM8Oracle into a single Dameng database type
- Add connection resilience layer — health guardian, trait dispatch, LRU cache, JDBC driver expansion (#109)
- Add custom window controls with draggable regions across platforms
- Add Oracle Cloud Wallet (ATP/ADW) connection support
- Enhance Oracle connection form with 3 connection methods and robust JRE download

### Fixed

- Aggregate multi-statement SQL results properly in JDBC bridge
- Enable schema browser for JDBC databases and show Actions column in query results
- Split multi-statement SQL in JDBC bridge and make list_connections resilient
- Fix Oracle DB listDatabases returning empty causing broken DB browser
- Fix agent self-healing, tool timeout, JDBC bridge hang, and UI polish
- Expand download_jdbc_driver_direct to all 22 JDBC databases
- Complete trait migration for remaining match arms in browse and explain_query
- Fix MySQL list_schemas regression — return only requested database
- Fix connection P0 connect timeout and P1 evict_idle logic bug
- Fix Oracle connection form issues and show full TNS alias names as-is
- Fix window dragging behavior across platforms
- Fix cross-database audit issues in JDBC bridge

### Changed

- Update READMEs for Dameng DM8 consolidation

## [0.7.1] - 2026-06-19

### Added

- Read `APP_VERSION` from `package.json` at compile time to prevent version drift between Cargo.toml and package.json
- Add `--version` flag to bridge JAR for download validation
- Prefer managed JRE (auto-downloaded from Adoptium) over system Java; only fall back to system Java when download fails and version is 25+
- Validate bridge JAR download with HTTP 200 check, minimum file size, and `java -jar --version` verification with automatic retry

### Fixed

- Capture stderr from crashed JDBC bridge subprocess — includes actual JVM error in the error message instead of opaque `exit status: 1`
- Prevent OS pipe deadlock by draining bridge stderr via background reader thread
- Show actual parsed Java version (e.g. `25.x`) in settings instead of literal `"system"`
- Fix stale version references from 21 to 25

## [0.7.0] - 2026-06-18

### Added

- Migrate TLS from OpenSSL (native-tls) to rustls for pure Rust TLS across all platforms (#105)
- Standardize result panel data contract and improve toolbar UX

### Fixed

- Render object cell values as JSON in DataGrid instead of `[object Object]`
- Fix i18n scope issues across DataGrid and agent UI — use `$t` instead of `t()`, add locale validation and fallback
- Make markdown table borders visible in dark mode
- Fix agent auto-connect so it connects to databases automatically on adapter resolve
- Resolve connection_id type mismatch — agents now use UUID strings instead of i64
- Resolve Unknown connection_id errors with UUID string comparison and dual-key connections map
- Align agent architecture with Dockit pattern — ChatFormatter trait, message loading, tool schemas, capabilities
- Resolve Data Studio UI bugs — duplicate status text, permission buttons, cancel action, source persistence, connection filtering (#103)
- Align Dockit UI across DataStudio, dialogs, and sidebar (#99)
- Use data-studio-agent from GitHub release instead of local path

## [0.6.5] - 2026-06-17

### Added

- Export `dbTypeFromBackend` for database icon resolution in JRE/Drivers section
- Redesign JRE/Drivers settings with icon buttons and compact layout

### Changed

- Remove JDBC toggle — JDBC support is now always enabled
- Align UnoCSS and CSS theme config with dockit approach

### Fixed

- Remove duplicate jdbc-bridge.jar from CI releases
- Fix lint issues
- Add cursor-pointer to interactive UI elements (dropdown items, toggle buttons)

## [0.6.4] - 2026-06-17

### Added

- Redesign JDBC management panel with bridge status display and database toggle
- Update frontend API layer with new JDBC types and methods
- Add JDBC management commands with automatic gate for non-JDBC connections
- Add two-phase driver resolution with JRE auto-update on connect
- Add ResolveDriver protocol, Adoptium JRE integration, and versioned bridge JAR
- Add Java-side Maven driver resolver using okhttp3
- Simplify drivers.toml and driver registry, remove fallback chains

### Changed

- Flatten bridge JAR storage, use versioned filenames instead of subdirectories

### Fixed

- Address review issues — fix classifier parsing, race condition, dead code, and code consolidation
- Upgrade JRE from 21 to 25 LTS for bridge JAR and Adoptium
- Use JDK 21 for bridge JAR, sync version with app

## [0.6.3] - 2026-06-17

### Added

- Support 70+ databases (up from 55) with 12 new wire-protocol compat databases: Greenplum, EnterpriseDB, CrateDB, Materialize, AlloyDB, CloudSQLPG, FujitsuPG, SingleStore/MemSQL, CloudSQLMySQL (#98)
- Dedicated SVG icons for Firebird, Derby, RQLite, Turso, TDengine and 7 new databases

### Changed

- Migrate DuckDB, Firebird, Oracle from native Rust adapters to JDBC bridge — reduces binary size by removing bundled C libraries (#98)
- Native pure Rust adapters now limited to PostgreSQL, MySQL, SQL Server, SQLite
- ClickHouse, RQLite, Turso consolidated under HTTP bridge
- README updated with current adapter strategy and 70+ database support

### Fixed

- CI: Replace deprecated `macos-13` runner with `macos-15-intel` for JRE builds
- CI: Add missing `actions/checkout` to publish job to fix JRE asset upload
- Fix pre-existing test compilation errors in JDBC registry, SSH transport, and agent loop

### Removed

- Babelfish (PG feature, not standalone database)
- RisingWave (dropped from scope)
- NDB Cluster (MySQL storage engine, not a separate product)

## [0.6.2] - 2026-06-17

### Added

- Port Dockit chat components — markdown-render, model-picker, context-indicator, agent-message-bubble (#96)
- Add i18n translations for Firebird, RQLite, Turso, TDengine

### Changed

- Reduce frontend bundle size by 57%
- Empty provider list by default with dropdown selector for provider type
- Replace raw SVGs with Carbon icon classes across app UI and database browser
- Sort database types by DB-Engines rank and remove Native/JDBC grouping

### Fixed

- Frontend bundle optimization and JRE auto-download for Oracle connections
- Dockit alignment — permission trigger CSS, button component, toolbar-center, chat input layout, model-picker panel styling
- Lazy-load highlight.js CSS to prevent FOUC at startup
- i18n overhaul — add 67 missing zhCN keys, fix Dialog crash, resolve HMR stale references, add HMR handler
- Resolve app freeze when navigating to Data Studio
- Wire data pipeline — progress, stopReason, auto-scroll
- Make i18n module safe for Jest by wrapping browser API access in try/catch
- Auto-fill Display Name from preset provider name
- Remove dead code causing Rust compiler startup warnings
- Various fixes — lint issues, test mock issues, ModelPicker visibility, ContextIndicator display, CSS consistency

## [0.6.1] - 2026-06-16

### Added

- Visual query execution plan as structured tree with cost highlighting (#94)

## [0.6.0] - 2026-06-16

### Added

- Firebird, RQLite, Turso, and TDengine database adapters (#93)
- SSH tunnel and transport layer support (#11, #91)

## [0.5.5] - 2026-06-16

### Added

- SQL formatting with configurable dialect and style (#92)

### Changed

- Format shortcut from Cmd+Shift+F to Shift+Alt+F for cross-platform consistency

## [0.5.4] - 2026-06-15

### Added

- Support for 12 new JDBC databases: database type variants with strategy routing, driver registry entries, string-to-enum parsing, driver download and fallback support, frontend DatabaseType entries, database icons, connection form dropdown entries, and i18n translations

### Fixed

- Review issues and code quality improvements

## [0.5.3] - 2026-06-15

### Added

- Virtual-scrolled data grid with sort, filter, action-based editing, and copy (#65, #89)

## [0.5.2] - 2026-06-15

### Changed

- Version bump only — no functional changes

## [0.5.1] - 2026-06-15

### Added

- ER diagram visualization with interactive canvas (#85)

## [0.5.0] - 2026-06-15

### Added

- JDBC driver management with registry, fallback chains, and JRE auto-detection (#87)
- Schema browser with views, procedures, functions, and enhanced table view with sub-pages (#84)

### Fixed

- Various review issues and code quality improvements

## [0.4.1] - 2026-06-15

### Changed

- Agent module fixes and database adapter improvements (#86)

## [0.4.0] - 2026-06-15

### Added

- Inline row data search across all columns in table view (#83)
- Confirmation dialogs for UI destructive actions in data grid and sidebar (#78)
- HTTP proxy configuration for LLM provider connections (#74)

### Fixed

- Windows MSVC build compatibility by setting CXXFLAGS for Visual C++ build tools

## [0.3.0] - 2026-06-14

### Added

- AI assistant sidebar, task manager, agent configuration, and provider alignment with Dockit (#63)
- 30+ SQL database support via protocol aliasing, JDBC bridge, and native drivers (#61)
- AI provider configuration UI in settings (#60)
- AI agent with SQL capabilities in Data Studio (#58)
- Data Studio agent improvements and transfer enhancements (#62)
- Scope-based architecture for Transfer module with 4-tab layout: Import, Export, Structure, Migration (#56)
- Migration wizard for cross-engine data transfer with column auto-mapping
- Structure tab: DDL generation for selected tables + SQL file execution against a target database
- New Tauri commands: `preview_migration`, `execute_migration`, `auto_map_migration_columns`, `generate_ddl_for_objects`, `execute_sql_content`
- `execute_sql_content` honors `onError` strategy: `stop` (default) and `skipAndContinue`
- DDL generation reconnects to the requested database for PostgreSQL and SQL Server when it differs from the active connection
- Full zh-CN i18n coverage for Transfer surfaces
- Consolidated refresh button in database browser (refreshes databases, tables, views, and saved queries)
- New Query (+) icon in saved queries section header

### Fixed

- tabStore API aligned with connectionId-first signatures (#53)
- PostgreSQL type conversion for NUMERIC/DECIMAL (exact precision via rust_decimal), UUID, ENUM, and INT2
- MySQL NULL value handling in column metadata (panic when INFORMATION_SCHEMA returns NULL values)
- SQL Server identity column update error by excluding auto-increment columns
- DECIMAL/NUMERIC precision preserved as string instead of lossy f64 conversion

### Changed

- Codebase simplification and cleanup
- Transfer module redesigned with scope-based architecture (#56)

### Known Limitations

- Migration backend currently emits PostgreSQL-flavored DDL; full cross-engine SQL dialect translation is pending
- DDL options for indexes, foreign keys, and target-engine type mapping are partially honored
- `execute_sql_content` cannot perform true transactional rollback: the database adapters acquire a fresh pooled connection per call, so BEGIN/COMMIT issued by this command would not span the executed statements. The `rollback` `onError` strategy is therefore implemented as "abort on first error" (same as `stop`). For atomic multi-statement batches, use the Query editor (single session).
- Pre-existing baseline issues outside Transfer scope are untouched: `cargo clippy` warnings in `commands/converter.rs` (PI literals), and 9 failing tests in `tests/store/tabStore.test.ts`

## [0.2.0] - 2025-04-15

### Added

- Saved queries feature with file list in DatabaseBrowser sidebar
- Context menu for saved queries (Open, Delete, Reveal in Finder)
- Auto-refresh saved query list after saving files
- Connection-prefixed default filenames (lowercase, hyphen-separated)
- Platform-aware keyboard shortcuts display (⌘ on Mac, Ctrl elsewhere)
- Connecting modal with spinner, error display, and retry/cancel actions
- Database logos for PostgreSQL, MySQL, MariaDB, MSSQL, SQLite in connection cards
- Spinner component with sm/md/lg sizes
- useMinLoadingTime composable for better loading UX
- usePlatform composable for OS detection
- useDatabaseIcon composable for database-specific logos

### Fixed

- False unsaved indicator when switching tabs in SQL editor
- SQLite in-memory database connection pooling issues
- Sidebar selector overflow for long hostnames
- AlertDialogAction button styling conflicts
- Connection state reset when switching connections
- SQLite in-memory connection test and save error handling

### Changed

- Reorder navigation: Connections → Queries → Import/Export → History
- Hide Data Studio from navigation
- Double-click on connection card now connects and navigates to editor
- Enforce `type` over `interface` declarations in TypeScript
- Migrate ConnectionManager to async RwLock for better concurrency
- Refactor get_table_data command to use TableDataQuery struct

## [0.1.1] - 2025-03-31

### Added

- Auto-update checker with manual check in settings
- SSL/TLS support for database connections
- Comprehensive loading UX improvements

### Fixed

- Various UI and connection handling improvements

## [0.1.0] - 2025-03-25

### Added

- Initial release
- Support for PostgreSQL, MySQL, MariaDB, MSSQL, SQLite
- SQL editor with syntax highlighting and auto-completion
- Database browser with tables, views, and columns
- Query execution with results display
- Connection management with saved connections
- History tracking for executed queries