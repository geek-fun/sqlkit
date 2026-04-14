# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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