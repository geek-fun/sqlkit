# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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