# Transfer Feature Design

## Overview

The **Transfer** feature replaces the placeholder "Import / Export" page with a comprehensive data transfer system supporting data export/import, database structure generation, SQL file execution, and cross-engine data migration.

**Feature Name**: Transfer
**Route**: `/transfer` (replaces `/import-export`)
**Sidebar Label**: Transfer

### Design Principles

- **Step-based inline wizards** (not modal dialogs) for each operation
- **Streaming/chunked processing** for large datasets (100k+ rows)
- **Progress reporting** via Tauri events for real-time UI feedback
- **Background task system** — all long-running operations run as background tasks; users can navigate away and return to check progress or restore task state (modeled after dockit's pattern)
- **Best-practice defaults** — format options use sensible defaults (comma delimiter, UTF-8, include header, etc.) instead of exposing every knob to the user; advanced users can expand an optional "Advanced" section if needed
- **Cross-platform** file dialogs using Tauri's native dialog plugin
- **All strings use i18n** for internationalization

---

## Information Architecture

### Top-Level Tabs

```
┌────────────────────────────────────────────────────────────────────────────┐
│  Transfer                                                    [🔔 Tasks]   │
├──────────┬──────────┬─────────────┬──────────────┬─────────────────────────┤
│  Export  │  Import  │  Structure  │  Migration   │                         │
├──────────┴──────────┴─────────────┴──────────────┴─────────────────────────┤
│                                                                            │
│  (Tab content area — wizard steps render here)                             │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

The **Tasks** button in the top-right opens a slide-out Task Manager panel showing all running and completed transfer tasks.

| Tab | Purpose | Steps |
|-----|---------|-------|
| **Export** | Export table data to file | 4 steps |
| **Import** | Import file data into a table | 4 steps |
| **Structure** | Generate DDL / Run SQL files | 2 sub-tabs, 2-4 steps each |
| **Migration** | Cross-engine data migration | 5 steps |

---

## Feature Specifications

### 1. Data Export

Export data from a table to a file in various formats.

#### Supported Formats

| Format | Extension | Crate | Best-Practice Defaults |
|--------|-----------|-------|------------------------|
| CSV | `.csv` | `csv` | Comma delimiter, double-quote, UTF-8, include header, LF line ending |
| JSONL | `.jsonl` | `serde_json` | One JSON object per line, compact, UTF-8, ISO 8601 dates |
| SQL | `.sql` | (built-in) | Auto-filled target table, batch size 1000, include CREATE TABLE |
| Excel | `.xlsx` | `rust_xlsxwriter` | Include header, auto-fit columns, freeze header row |

> **Design decision**: Format options use best-practice defaults automatically. No per-format options panels are shown. An optional "Advanced Options" expandable section is available for power users who need to override defaults.

#### Wizard Steps

**Step 1 — Source Selection**

```
┌─ Step 1: Source ──────────────────────────────────────────────────────────┐
│                                                                           │
│  Connection:    [▼ my-postgres-server           ]                         │
│  Database:      [▼ mydb                         ]                         │
│  Schema:        [▼ public                       ]                         │
│                                                                           │
│  Table:         [▼ users                        ]                         │
│  Columns:       ☑ id  ☑ name  ☑ email  ☐ password  ☑ created_at          │
│                 [Select All] [Deselect All]                                │
│  WHERE:         [age > 18                       ] (optional)              │
│  ORDER BY:      [created_at DESC                ] (optional)              │
│  LIMIT:         [1000                           ] (optional)              │
│                                                                           │
│                                                       [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 2 — Format Selection**

```
┌─ Step 2: Format ──────────────────────────────────────────────────────────┐
│                                                                           │
│  Format:  ○ CSV    ○ JSONL    ○ SQL    ○ Excel                            │
│                                                                           │
│  ┌─ Defaults Applied ────────────────────────────────────────────────┐   │
│  │  ✓ Comma delimiter, double-quote, UTF-8 encoding                  │   │
│  │  ✓ Header row included                                            │   │
│  │  ✓ LF line endings                                                │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  ▸ Advanced Options                                                       │
│  ┌─ (expanded, only when clicked) ───────────────────────────────────┐   │
│  │  Delimiter:       [▼ Comma (,)      ]                             │   │
│  │  Encoding:        [▼ UTF-8          ]                             │   │
│  │  ☑ Include header row                                              │   │
│  │  (format-specific overrides shown based on selection)              │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                            [← Back]  [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 3 — Preview**

```
┌─ Step 3: Preview ─────────────────────────────────────────────────────────┐
│                                                                           │
│  Source: my-postgres-server / mydb / public.users                         │
│  Format: CSV  |  Rows: 1,000 (estimated)  |  Columns: 4                  │
│                                                                           │
│  ┌─ Preview (first 10 rows) ─────────────────────────────────────────┐   │
│  │ id,name,email,created_at                                          │   │
│  │ 1,"Alice","alice@example.com","2024-01-15T10:30:00Z"              │   │
│  │ 2,"Bob","bob@example.com","2024-01-16T14:22:00Z"                  │   │
│  │ 3,"Charlie","charlie@example.com","2024-01-17T09:15:00Z"          │   │
│  │ ...                                                                │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  Output File: [/Users/me/exports/users.csv   ] [Browse...]               │
│                                                                           │
│                                            [← Back]  [Export →]           │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 4 — Execution**

```
┌─ Step 4: Exporting ───────────────────────────────────────────────────────┐
│                                                                           │
│  Status: Exporting...                                                     │
│                                                                           │
│  ████████████████████░░░░░░░░░░  65%                                      │
│                                                                           │
│  Rows exported: 650 / 1,000                                               │
│  Elapsed: 2.3s                                                            │
│  Estimated remaining: 1.2s                                                │
│                                                                           │
│                                   [Run in Background]  [Cancel]           │
│                                                                           │
│  ── After completion ──                                                   │
│                                                                           │
│  ✓ Export completed successfully                                          │
│  File: /Users/me/exports/users.csv (45.2 KB)                             │
│  Rows exported: 1,000                                                     │
│  Duration: 3.5s                                                           │
│                                                                           │
│                              [Open File]  [Open Folder]  [Export Again]   │
└───────────────────────────────────────────────────────────────────────────┘
```

Clicking **Run in Background** detaches the task from the wizard UI and adds it to the Task Manager. The user can navigate away and return later to check progress.

---

### 2. Data Import

Import data from a file into a database table.

#### Wizard Steps

**Step 1 — File Selection**

```
┌─ Step 1: Select File ─────────────────────────────────────────────────────┐
│                                                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                                                                     │  │
│  │          ┌──────────┐                                               │  │
│  │          │  📄 ↑    │   Drag & drop a file here                     │  │
│  │          └──────────┘   or click to browse                          │  │
│  │                                                                     │  │
│  │          Supported: CSV, JSONL, SQL, Excel (.xlsx)                    │  │
│  │                                                                     │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                           │
│  ── After file selected ──                                                │
│                                                                           │
│  File: users_export.csv (45.2 KB)                                         │
│  Detected Format: CSV                                                     │
│  Detected Encoding: UTF-8                                                 │
│  Rows (estimated): 1,000                                                  │
│                                                                           │
│  Parse settings auto-detected. Adjust only if needed:                     │
│  ▸ Advanced Parse Options                                                 │
│  ┌─ (expanded, only when clicked) ───────────────────────────────────┐   │
│  │  Delimiter:       [▼ Comma (,)      ]  (auto-detected)            │   │
│  │  Encoding:        [▼ UTF-8          ]  (auto-detected)            │   │
│  │  ☑ First row is header                                             │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                                       [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 2 — Target & Column Mapping**

```
┌─ Step 2: Target & Mapping ────────────────────────────────────────────────┐
│                                                                           │
│  Connection:    [▼ my-postgres-server           ]                         │
│  Database:      [▼ mydb                         ]                         │
│  Schema:        [▼ public                       ]                         │
│  Table:         [▼ users                        ]                         │
│                 ☐ Create table if not exists                               │
│                                                                           │
│  ┌─ Column Mapping ──────────────────────────────────────────────────┐   │
│  │                                                                    │   │
│  │  Source Column    →   Target Column      Type          Status      │   │
│  │  ─────────────────────────────────────────────────────────────     │   │
│  │  id               →   [▼ id           ]  INTEGER       ✓ Mapped   │   │
│  │  name             →   [▼ name         ]  VARCHAR(255)  ✓ Mapped   │   │
│  │  email            →   [▼ email        ]  VARCHAR(255)  ✓ Mapped   │   │
│  │  created_at       →   [▼ created_at   ]  TIMESTAMP     ✓ Mapped   │   │
│  │  phone            →   [▼ (skip)       ]  —             ⊘ Skipped  │   │
│  │                                                                    │   │
│  │  [Auto-Map by Name]  [Clear All]                                   │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                            [← Back]  [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 3 — Options & Preview**

```
┌─ Step 3: Options & Preview ───────────────────────────────────────────────┐
│                                                                           │
│  ┌─ Import Options ──────────────────────────────────────────────────┐   │
│  │  On Conflict:     [▼ Skip duplicates  ]                           │   │
│  │                     ├─ Skip duplicates                             │   │
│  │                     ├─ Replace existing                            │   │
│  │                     ├─ Update existing (upsert)                    │   │
│  │                     └─ Abort on error                              │   │
│  │  Batch Size:      [5000              ] rows per transaction        │   │
│  │  ☐ Truncate table before import                                    │   │
│  │  ☐ Dry run (validate without inserting)                            │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  ┌─ Data Preview (first 5 rows as mapped) ───────────────────────────┐   │
│  │  id  │  name     │  email              │  created_at              │   │
│  │  ────┼───────────┼─────────────────────┼──────────────────────    │   │
│  │  1   │  Alice    │  alice@example.com  │  2024-01-15 10:30:00     │   │
│  │  2   │  Bob      │  bob@example.com    │  2024-01-16 14:22:00     │   │
│  │  3   │  Charlie  │  charlie@ex...      │  2024-01-17 09:15:00     │   │
│  │  ...                                                               │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                            [← Back]  [Import →]          │
└───────────────────────────────────────────────────────────────────────────┘
```

> **Simplification**: "Disable indexes during import" option has been removed — the backend automatically handles index optimization for large imports (>10k rows) when the engine supports it.

**Step 4 — Execution**

```
┌─ Step 4: Importing ───────────────────────────────────────────────────────┐
│                                                                           │
│  Status: Importing...                                                     │
│                                                                           │
│  ████████████████████░░░░░░░░░░  65%                                      │
│                                                                           │
│  Rows imported: 650 / 1,000                                               │
│  Rows skipped:  3 (duplicates)                                            │
│  Errors: 0                                                                │
│  Elapsed: 4.1s                                                            │
│                                                                           │
│                                   [Run in Background]  [Cancel]           │
│                                                                           │
│  ── After completion ──                                                   │
│                                                                           │
│  ✓ Import completed successfully                                          │
│  Rows imported: 997  |  Skipped: 3  |  Errors: 0                         │
│  Duration: 6.2s                                                           │
│                                                                           │
│  ┌─ Error Log (if any) ─────────────────────────────────────────────┐   │
│  │  Row 45: Duplicate key violation on column 'email'                │   │
│  │  Row 102: NULL value for non-nullable column 'name'               │   │
│  │  Row 339: Data truncation on column 'phone' (max 20 chars)        │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                         [View Table]  [Import Again]      │
└───────────────────────────────────────────────────────────────────────────┘
```

---

### 3. Structure

Two sub-tabs: **Generate DDL** and **Run SQL File**.

```
┌─ Structure ───────────────────────────────────────────────────────────────┐
│  ┌──────────────┬────────────────┐                                        │
│  │ Generate DDL │  Run SQL File  │                                        │
│  └──────────────┴────────────────┘                                        │
│  (sub-tab content below)                                                  │
└───────────────────────────────────────────────────────────────────────────┘
```

#### 3A. Generate DDL

Generate DDL (Data Definition Language) scripts from existing database objects.

**Step 1 — Object Selection**

```
┌─ Step 1: Select Objects ──────────────────────────────────────────────────┐
│                                                                           │
│  Connection:    [▼ my-postgres-server           ]                         │
│  Database:      [▼ mydb                         ]                         │
│  Schema:        [▼ public                       ]                         │
│                                                                           │
│  ┌─ Objects ─────────────────────────────────────────────────────────┐   │
│  │  ☑ users                    TABLE    12 columns   1,200 rows      │   │
│  │  ☑ orders                   TABLE    8 columns    45,000 rows     │   │
│  │  ☐ order_items              TABLE    6 columns    120,000 rows    │   │
│  │  ☐ products                 TABLE    10 columns   500 rows        │   │
│  │  ☑ user_summary_view        VIEW     5 columns    —               │   │
│  │                                                                    │   │
│  │  [Select All] [Deselect All] [Tables Only] [Views Only]            │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                                       [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 2 — DDL Options**

```
┌─ Step 2: DDL Options ─────────────────────────────────────────────────────┐
│                                                                           │
│  Target Engine:  [▼ Same as source (PostgreSQL) ]                         │
│                                                                           │
│  ┌─ Include ─────────────────────────────────────────────────────────┐   │
│  │  ☑ CREATE TABLE statements                                        │   │
│  │  ☑ Primary keys                                                    │   │
│  │  ☑ Foreign keys                                                    │   │
│  │  ☑ Indexes                                                         │   │
│  │  ☑ Constraints (UNIQUE, CHECK, NOT NULL)                           │   │
│  │  ☐ Comments / descriptions                                         │   │
│  │  ☐ Tablespace / storage options                                    │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  ┌─ Behavior ────────────────────────────────────────────────────────┐   │
│  │  ☑ Include DROP IF EXISTS before CREATE                            │   │
│  │  ☑ Include IF NOT EXISTS on CREATE                                 │   │
│  │  ☐ Include INSERT DATA (export structure + data)                   │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                            [← Back]  [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 3 — Preview & Export**

```
┌─ Step 3: Preview & Export ────────────────────────────────────────────────┐
│                                                                           │
│  Objects: 3 selected  |  Target: PostgreSQL                               │
│                                                                           │
│  ┌─ DDL Preview (Monaco editor, read-only) ──────────────────────────┐   │
│  │  -- Generated by SQLKit on 2024-03-15                              │   │
│  │  -- Source: my-postgres-server / mydb / public                     │   │
│  │                                                                    │   │
│  │  DROP TABLE IF EXISTS "users" CASCADE;                             │   │
│  │  CREATE TABLE IF NOT EXISTS "users" (                              │   │
│  │      "id" SERIAL PRIMARY KEY,                                      │   │
│  │      "name" VARCHAR(255) NOT NULL,                                 │   │
│  │      "email" VARCHAR(255) NOT NULL UNIQUE,                         │   │
│  │      "created_at" TIMESTAMP WITH TIME ZONE DEFAULT NOW()           │   │
│  │  );                                                                │   │
│  │                                                                    │   │
│  │  CREATE INDEX "idx_users_email" ON "users" ("email");              │   │
│  │  ...                                                               │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│              [Copy to Clipboard]  [Save to File...]  [Execute on Server]  │
└───────────────────────────────────────────────────────────────────────────┘
```

#### 3B. Run SQL File

Open and execute SQL files against a database connection.

**Step 1 — File & Connection**

```
┌─ Step 1: Select File & Connection ────────────────────────────────────────┐
│                                                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │          ┌──────────┐                                               │  │
│  │          │  📄 ↑    │   Drag & drop a .sql file here                │  │
│  │          └──────────┘   or click to browse                          │  │
│  │                                                                     │  │
│  │          Supported: .sql files                                      │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                           │
│  File: schema_backup.sql (128 KB, 2,400 statements)                      │
│                                                                           │
│  Connection:    [▼ my-postgres-server           ]                         │
│  Database:      [▼ mydb                         ]                         │
│                                                                           │
│  ┌─ Execution Options ───────────────────────────────────────────────┐   │
│  │  ☑ Wrap in transaction                                             │   │
│  │  On Error:       [▼ Rollback all     ]                             │   │
│  │                    ├─ Rollback all                                  │   │
│  │                    ├─ Skip and continue                             │   │
│  │                    └─ Stop execution                                │   │
│  │  ☐ Dry run (parse only, don't execute)                             │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  ┌─ File Preview (first 50 lines) ───────────────────────────────────┐   │
│  │  -- Database schema backup                                         │   │
│  │  CREATE TABLE users ( ... );                                       │   │
│  │  CREATE TABLE orders ( ... );                                      │   │
│  │  INSERT INTO users VALUES (1, 'Alice', ...);                       │   │
│  │  ...                                                               │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                                       [Execute →]         │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 2 — Execution & Results**

```
┌─ Step 2: Execution ───────────────────────────────────────────────────────┐
│                                                                           │
│  Status: Executing...                                                     │
│                                                                           │
│  ████████████████████░░░░░░░░░░  65%                                      │
│                                                                           │
│  Statements: 1,560 / 2,400                                                │
│  Succeeded: 1,558  |  Failed: 2                                           │
│  Elapsed: 12.4s                                                           │
│                                                                           │
│                                   [Run in Background]  [Cancel]           │
│                                                                           │
│  ── After completion ──                                                   │
│                                                                           │
│  ✓ Execution completed                                                    │
│  Total: 2,400  |  Succeeded: 2,398  |  Failed: 2                         │
│  Duration: 19.1s                                                          │
│                                                                           │
│  ┌─ Execution Log ──────────────────────────────────────────────────┐    │
│  │  ✓ Statement 1: CREATE TABLE users — OK                          │    │
│  │  ✓ Statement 2: CREATE TABLE orders — OK                         │    │
│  │  ✗ Statement 145: INSERT INTO ... — ERROR: duplicate key         │    │
│  │  ✗ Statement 892: ALTER TABLE ... — ERROR: column exists         │    │
│  │  ...                                                              │    │
│  │  [Show Errors Only] [Copy Log]                                    │    │
│  └──────────────────────────────────────────────────────────────────┘    │
│                                                                           │
│                                                       [Run Again]         │
└───────────────────────────────────────────────────────────────────────────┘
```

---

### 4. Data Migration

Cross-engine data migration (e.g., MySQL to PostgreSQL).

#### Wizard Steps

**Step 1 — Source Connection**

```
┌─ Step 1: Source ──────────────────────────────────────────────────────────┐
│                                                                           │
│  Source Connection:  [▼ my-mysql-server              ]                     │
│  Database:           [▼ production_db                ]                     │
│  Schema:             [▼ (default)                    ]                     │
│                                                                           │
│  Available Tables:                                                        │
│  ☑ users             TABLE    12 columns   1,200 rows                     │
│  ☑ orders            TABLE    8 columns    45,000 rows                    │
│  ☑ products          TABLE    10 columns   500 rows                       │
│  ☐ audit_log         TABLE    6 columns    2,000,000 rows                 │
│  ☐ temp_data         TABLE    3 columns    50 rows                        │
│                                                                           │
│  [Select All] [Deselect All]                                              │
│                                                                           │
│  Selected: 3 tables, ~46,700 rows                                         │
│                                                                           │
│                                                       [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 2 — Target Connection**

```
┌─ Step 2: Target ──────────────────────────────────────────────────────────┐
│                                                                           │
│  Target Connection:  [▼ my-postgres-server           ]                    │
│  Database:           [▼ new_production               ]                    │
│  Schema:             [▼ public                       ]                    │
│                                                                           │
│  ☑ Create target tables if not exist                                      │
│  ☐ Drop target tables before migration                                    │
│                                                                           │
│  Migration Direction:                                                     │
│  ┌─────────────┐         ┌─────────────┐                                  │
│  │   MySQL     │  ───→   │ PostgreSQL  │                                  │
│  │ production  │         │ new_prod    │                                  │
│  │ 3 tables    │         │ public      │                                  │
│  └─────────────┘         └─────────────┘                                  │
│                                                                           │
│                                            [← Back]  [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 3 — Schema & Type Mapping**

```
┌─ Step 3: Schema & Type Mapping ───────────────────────────────────────────┐
│                                                                           │
│  ┌─ Table: users ────────────────────────────────────────────────────┐   │
│  │                                                                    │   │
│  │  Source (MySQL)         →   Target (PostgreSQL)        Status      │   │
│  │  ─────────────────────────────────────────────────────────────     │   │
│  │  id INT AUTO_INCREMENT  →   id SERIAL                  ✓ Auto     │   │
│  │  name VARCHAR(255)      →   name VARCHAR(255)          ✓ Auto     │   │
│  │  email VARCHAR(255)     →   email VARCHAR(255)         ✓ Auto     │   │
│  │  bio TEXT               →   bio TEXT                   ✓ Auto     │   │
│  │  data JSON              →   data JSONB                 ⚠ Mapped   │   │
│  │  created DATETIME       →   created TIMESTAMP          ⚠ Mapped   │   │
│  │  active TINYINT(1)      →   active BOOLEAN             ⚠ Mapped   │   │
│  │                                                                    │   │
│  │  [Edit Mapping]  [Reset to Auto]                                   │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  ┌─ Table: orders ───────────────────────────────────────────────────┐   │
│  │  (similar mapping grid)                                            │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  ⚠ 5 columns require type conversion (auto-mapped)                       │
│  ✓ 19 columns map directly                                                │
│                                                                           │
│                                            [← Back]  [Next →]            │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 4 — Configure**

```
┌─ Step 4: Configure ───────────────────────────────────────────────────────┐
│                                                                           │
│  ┌─ Migration Options ───────────────────────────────────────────────┐   │
│  │  Batch Size:      [5000              ] rows per batch              │   │
│  │  On Error:        [▼ Skip row and continue  ]                      │   │
│  │  ☑ Migrate indexes                                                 │   │
│  │  ☑ Migrate foreign keys                                            │   │
│  │  ☑ Migrate constraints                                             │   │
│  │  ☐ Disable foreign key checks during migration                     │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  ┌─ Migration Plan Summary ──────────────────────────────────────────┐   │
│  │  Source: MySQL (my-mysql-server / production_db)                    │   │
│  │  Target: PostgreSQL (my-postgres-server / new_production / public) │   │
│  │  Tables: 3                                                         │   │
│  │  Total Rows: ~46,700                                               │   │
│  │  Type Conversions: 5                                               │   │
│  │  Estimated Time: ~30 seconds                                       │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                            [← Back]  [Start Migration →]  │
└───────────────────────────────────────────────────────────────────────────┘
```

**Step 5 — Execution**

```
┌─ Step 5: Migrating ───────────────────────────────────────────────────────┐
│                                                                           │
│  Overall Progress:                                                        │
│  ████████████████░░░░░░░░░░░░░░  52%                                      │
│                                                                           │
│  ┌─ Per-Table Progress ──────────────────────────────────────────────┐   │
│  │  ✓ users      1,200 / 1,200    ████████████████████  100%  1.2s   │   │
│  │  ● orders     18,500 / 45,000  ████████░░░░░░░░░░░░  41%   8.3s   │   │
│  │  ○ products   0 / 500          ░░░░░░░░░░░░░░░░░░░░  0%    —      │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  Phase: Migrating data (2/3 tables)                                       │
│  Errors: 0  |  Elapsed: 9.5s  |  Remaining: ~8s                          │
│                                                                           │
│                                   [Run in Background]  [Cancel]           │
│                                                                           │
│  ── After completion ──                                                   │
│                                                                           │
│  ✓ Migration completed successfully                                       │
│                                                                           │
│  ┌─ Results ─────────────────────────────────────────────────────────┐   │
│  │  Table       Rows      Status    Duration                         │   │
│  │  ──────────────────────────────────────────────                    │   │
│  │  users       1,200     ✓ OK      1.2s                             │   │
│  │  orders      45,000    ✓ OK      14.8s                            │   │
│  │  products    500       ✓ OK      0.3s                             │   │
│  │  ──────────────────────────────────────────────                    │   │
│  │  Total       46,700              16.3s                             │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│                                                    [Migrate Again]        │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## Background Task System

All long-running transfer operations (export, import, SQL file execution, migration) run as **background tasks**. This allows users to navigate away from the Transfer page and return later to check progress or restore task state.

**Architecture**: Task management is **frontend-only** (no Rust backend task registry). The Tauri commands block until completion, but the frontend wraps each invocation in an async call tracked by the Pinia store. Progress is synced from Tauri events to the corresponding task entry.

### Task Lifecycle

```
User clicks "Export" / "Import" / etc.
  ↓
Frontend creates BackgroundTask { id, kind, status: 'running', config: snapshot }
  ↓
Frontend calls Tauri command (async, non-blocking from UI perspective)
  ↓
Tauri command emits progress events → store.syncProgressToTask(taskId)
  ↓
User clicks "Run in Background" → store.detachActiveTask(kind)
  ↓
User navigates away (task continues running in store)
  ↓
User opens Task Manager → sees task card with live progress
  ↓
User clicks "Go to task" → router navigates to /transfer?tab=export&taskId=xyz
  ↓
store.openTask(taskId) → restores full form state from task.config
  ↓
Tauri command completes → store.updateTaskStatus(taskId, 'completed')
```

### Task Manager Panel

The Task Manager is a slide-out sidebar panel accessible from the Transfer page header. It shows all running and completed tasks.

```
┌─ Task Manager ──────────────────────────────────┐
│  Tasks (3)                    [Clear Completed]  │
│                                                  │
│  ┌─ Export ─────────────────────────────────┐    │
│  │  📤 Export users → CSV                   │    │
│  │  ● Running                               │    │
│  │  ████████████████░░░░  78%               │    │
│  │  780 / 1,000 rows                        │    │
│  │  Started: 2 min ago                      │    │
│  │                          [Go to Task →]  │    │
│  └──────────────────────────────────────────┘    │
│                                                  │
│  ┌─ Import ─────────────────────────────────┐    │
│  │  📥 Import orders.csv                    │    │
│  │  ✓ Completed                             │    │
│  │  ████████████████████  100%              │    │
│  │  45,000 rows  |  3 skipped               │    │
│  │  Duration: 12.4s                         │    │
│  │               [Dismiss]  [Go to Task →]  │    │
│  └──────────────────────────────────────────┘    │
│                                                  │
│  ┌─ Migration ──────────────────────────────┐    │
│  │  🔄 MySQL → PostgreSQL (3 tables)        │    │
│  │  ✗ Failed                                │    │
│  │  ████████████░░░░░░░░  58%               │    │
│  │  Error: Connection lost to target        │    │
│  │               [Dismiss]  [Go to Task →]  │    │
│  └──────────────────────────────────────────┘    │
│                                                  │
└──────────────────────────────────────────────────┘
```

**Status colors**: Running = blue, Completed = green, Failed = red, Pending = yellow.

**Task card actions**:
- **Go to Task**: Navigates to the Transfer page with the task's tab active and restores the full wizard state from the task's config snapshot
- **Dismiss**: Removes the task card (only shown for non-running tasks)
- **Clear Completed**: Removes all completed/failed task cards

### Task Types

```typescript
// src/types/transfer.ts (additions)

export type TaskKind = 'export' | 'import' | 'sqlFile' | 'migration'

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed'

export type TaskRuntime = {
  complete: number
  total: number
  skipped: number
  errorCount: number
}

export type ExportTaskConfig = {
  connectionId: string
  database?: string
  schema?: string
  table: string
  columns: string[]
  whereClause?: string
  orderBy?: string
  limit?: number
  format: ExportFormat
  outputPath: string
}

export type ImportTaskConfig = {
  connectionId: string
  database?: string
  schema?: string
  table: string
  filePath: string
  format: ImportFormat
  conflictStrategy: ConflictStrategy
}

export type SqlFileTaskConfig = {
  connectionId: string
  database?: string
  filePath: string
  onError: SqlFileErrorStrategy
}

export type MigrationTaskConfig = {
  sourceConnectionId: string
  sourceDatabase?: string
  targetConnectionId: string
  targetDatabase?: string
  tables: string[]
}

export type TaskConfig = ExportTaskConfig | ImportTaskConfig | SqlFileTaskConfig | MigrationTaskConfig

export type BackgroundTask = {
  id: string
  kind: TaskKind
  status: TaskStatus
  progress: { complete: number; total: number }
  config: TaskConfig
  runtime: TaskRuntime
  label: string
  startTime: Date
  endTime?: Date
  error?: string
}
```

### Store Task Management Methods

```typescript
// Added to useTransferStore (see Pinia Store section below)

// ── Task State ──────────────────────────────────
const runningTasks = ref<BackgroundTask[]>([])
const activeExportTaskId = ref<string | null>(null)
const activeImportTaskId = ref<string | null>(null)
const activeSqlFileTaskId = ref<string | null>(null)
const activeMigrationTaskId = ref<string | null>(null)

// ── Task Getters ────────────────────────────────
const taskCount = computed(() => runningTasks.value.length)
const hasRunningTasks = computed(() =>
  runningTasks.value.some(t => t.status === 'running')
)

// ── Task Actions ────────────────────────────────
const addRunningTask = (task: BackgroundTask) => {
  runningTasks.value = [...runningTasks.value, task]
}

const updateTaskRuntime = (taskId: string, runtime: Partial<TaskRuntime>) => {
  runningTasks.value = runningTasks.value.map(t =>
    t.id === taskId
      ? { ...t, runtime: { ...t.runtime, ...runtime }, progress: { complete: runtime.complete ?? t.progress.complete, total: runtime.total ?? t.progress.total } }
      : t
  )
}

const updateTaskStatus = (taskId: string, status: TaskStatus, error?: string) => {
  runningTasks.value = runningTasks.value.map(t =>
    t.id === taskId
      ? { ...t, status, endTime: status === 'completed' || status === 'failed' ? new Date() : undefined, error }
      : t
  )
}

const removeTask = (taskId: string) => {
  runningTasks.value = runningTasks.value.filter(t => t.id !== taskId)
}

const clearCompletedTasks = () => {
  runningTasks.value = runningTasks.value.filter(t =>
    t.status === 'running' || t.status === 'pending'
  )
}

const openTask = (taskId: string) => {
  const task = runningTasks.value.find(t => t.id === taskId)
  if (!task) return
  // Restore form state from task.config back to the active wizard
  // Implementation depends on task.kind — sets the appropriate step fields
  activeTab.value = taskKindToTab(task.kind)
  // ... restore config fields to wizard state
}

const detachActiveTask = (kind: TaskKind) => {
  // Clears the active task ID without stopping the operation
  // Allows user to navigate away while task continues
  switch (kind) {
    case 'export': activeExportTaskId.value = null; break
    case 'import': activeImportTaskId.value = null; break
    case 'sqlFile': activeSqlFileTaskId.value = null; break
    case 'migration': activeMigrationTaskId.value = null; break
  }
}

const syncProgressToTask = (taskId: string, progress: TransferProgress) => {
  updateTaskRuntime(taskId, {
    complete: progress.processedRows,
    total: progress.totalRows ?? 0,
    skipped: progress.skippedRows,
    errorCount: progress.errorCount,
  })
}
```

### Task Creation Flow (Export Example)

```typescript
// In ExportExecuteStep.vue

import { ulid } from 'ulidx'

const handleStartExport = async () => {
  const taskId = ulid()
  const configSnapshot: ExportTaskConfig = {
    connectionId: transferStore.exportRequest.connectionId!,
    database: transferStore.exportRequest.database,
    schema: transferStore.exportRequest.schema,
    table: transferStore.exportRequest.source.table,
    columns: transferStore.exportRequest.source.columns,
    format: transferStore.exportRequest.format!,
    outputPath: transferStore.exportRequest.outputPath!,
  }

  transferStore.addRunningTask({
    id: taskId,
    kind: 'export',
    status: 'running',
    progress: { complete: 0, total: estimatedRows.value },
    config: configSnapshot,
    runtime: { complete: 0, total: estimatedRows.value, skipped: 0, errorCount: 0 },
    label: `Export ${configSnapshot.table} → ${configSnapshot.format.toUpperCase()}`,
    startTime: new Date(),
  })
  transferStore.activeExportTaskId = taskId

  try {
    const result = await executeExport(transferStore.exportRequest as ExportRequest)
    transferStore.updateTaskStatus(taskId, 'completed')
    transferStore.completeOperation(result)
  } catch (err) {
    transferStore.updateTaskStatus(taskId, 'failed', String(err))
  }
}
```

### Task Manager Navigation

When the user clicks **"Go to Task"** in the Task Manager, the router navigates with query params:

```typescript
router.push({
  path: '/transfer',
  query: { tab: task.kind, taskId: task.id },
})
```

The `TransferPage.vue` watches for `taskId` in the route query and calls `transferStore.openTask(taskId)` to restore the wizard to the task's state (showing progress or results).

---

## Cross-Engine Type Mapping Matrix

The migration feature requires automatic type mapping between database engines. Below is the mapping matrix used for cross-engine translation.

### PostgreSQL ↔ MySQL

| PostgreSQL | MySQL | Notes |
|------------|-------|-------|
| `SERIAL` | `INT AUTO_INCREMENT` | Auto-increment PK |
| `BIGSERIAL` | `BIGINT AUTO_INCREMENT` | Large auto-increment PK |
| `SMALLINT` | `SMALLINT` | Direct |
| `INTEGER` | `INT` | Direct |
| `BIGINT` | `BIGINT` | Direct |
| `REAL` | `FLOAT` | Direct |
| `DOUBLE PRECISION` | `DOUBLE` | Direct |
| `NUMERIC(p,s)` | `DECIMAL(p,s)` | Direct |
| `BOOLEAN` | `TINYINT(1)` | MySQL lacks native boolean |
| `VARCHAR(n)` | `VARCHAR(n)` | Direct |
| `TEXT` | `TEXT` / `LONGTEXT` | TEXT if ≤64KB, LONGTEXT otherwise |
| `CHAR(n)` | `CHAR(n)` | Direct |
| `BYTEA` | `LONGBLOB` | Binary data |
| `TIMESTAMP` | `DATETIME` | MySQL DATETIME lacks timezone |
| `TIMESTAMPTZ` | `DATETIME` | Timezone info lost |
| `DATE` | `DATE` | Direct |
| `TIME` | `TIME` | Direct |
| `INTERVAL` | `VARCHAR(255)` | No MySQL equivalent |
| `JSON` | `JSON` | Direct |
| `JSONB` | `JSON` | MySQL lacks binary JSON |
| `UUID` | `CHAR(36)` | MySQL lacks native UUID |
| `INET` | `VARCHAR(45)` | No MySQL equivalent |
| `CIDR` | `VARCHAR(45)` | No MySQL equivalent |
| `MACADDR` | `VARCHAR(17)` | No MySQL equivalent |
| `ARRAY` | `JSON` | MySQL lacks native arrays |
| `POINT` | `POINT` | Spatial type (both support) |

### PostgreSQL ↔ SQLite

| PostgreSQL | SQLite | Notes |
|------------|--------|-------|
| `SERIAL` / `BIGSERIAL` | `INTEGER PRIMARY KEY` | SQLite auto-increment via ROWID |
| `SMALLINT` / `INTEGER` / `BIGINT` | `INTEGER` | SQLite has single integer type |
| `REAL` / `DOUBLE PRECISION` | `REAL` | Direct |
| `NUMERIC(p,s)` | `REAL` | SQLite lacks fixed-point |
| `BOOLEAN` | `INTEGER` | 0/1 convention |
| `VARCHAR(n)` / `TEXT` | `TEXT` | SQLite ignores length constraints |
| `BYTEA` | `BLOB` | Direct |
| `TIMESTAMP` / `TIMESTAMPTZ` | `TEXT` | ISO 8601 string |
| `DATE` / `TIME` | `TEXT` | ISO 8601 string |
| `JSON` / `JSONB` | `TEXT` | Plain text storage |
| `UUID` | `TEXT` | 36-char string |

### PostgreSQL ↔ SQL Server

| PostgreSQL | SQL Server | Notes |
|------------|------------|-------|
| `SERIAL` | `INT IDENTITY(1,1)` | Auto-increment |
| `BIGSERIAL` | `BIGINT IDENTITY(1,1)` | Large auto-increment |
| `SMALLINT` | `SMALLINT` | Direct |
| `INTEGER` | `INT` | Direct |
| `BIGINT` | `BIGINT` | Direct |
| `REAL` | `REAL` | Direct |
| `DOUBLE PRECISION` | `FLOAT` | Direct |
| `NUMERIC(p,s)` | `DECIMAL(p,s)` | Direct |
| `BOOLEAN` | `BIT` | 0/1 |
| `VARCHAR(n)` | `NVARCHAR(n)` | Unicode by default |
| `TEXT` | `NVARCHAR(MAX)` | Unicode by default |
| `CHAR(n)` | `NCHAR(n)` | Unicode by default |
| `BYTEA` | `VARBINARY(MAX)` | Binary data |
| `TIMESTAMP` | `DATETIME2` | Higher precision |
| `TIMESTAMPTZ` | `DATETIMEOFFSET` | With timezone |
| `DATE` | `DATE` | Direct |
| `TIME` | `TIME` | Direct |
| `JSON` / `JSONB` | `NVARCHAR(MAX)` | SQL Server lacks native JSON type |
| `UUID` | `UNIQUEIDENTIFIER` | Native support |
| `XML` | `XML` | Native support |

### MySQL ↔ SQL Server

| MySQL | SQL Server | Notes |
|-------|------------|-------|
| `INT AUTO_INCREMENT` | `INT IDENTITY(1,1)` | Auto-increment |
| `TINYINT(1)` | `BIT` | Boolean |
| `TINYINT` | `TINYINT` | Direct |
| `VARCHAR(n)` | `NVARCHAR(n)` | Unicode |
| `TEXT` | `NVARCHAR(MAX)` | Unicode |
| `LONGTEXT` | `NVARCHAR(MAX)` | Unicode |
| `BLOB` / `LONGBLOB` | `VARBINARY(MAX)` | Binary |
| `DATETIME` | `DATETIME2` | Higher precision |
| `TIMESTAMP` | `DATETIME2` | Meaning differs |
| `JSON` | `NVARCHAR(MAX)` | No native type |
| `ENUM(...)` | `NVARCHAR(255)` + CHECK | No native ENUM |
| `SET(...)` | `NVARCHAR(MAX)` | No native SET |

---

## Backend Architecture

### New Module Structure

```
src-tauri/src/
├── transfer/                      # New module
│   ├── mod.rs                     # Module exports
│   ├── types.rs                   # Transfer-specific types
│   ├── defaults.rs                # Best-practice default configs per format
│   ├── export.rs                  # Export logic (CSV, JSONL, SQL, Excel)
│   ├── import.rs                  # Import logic (parse, validate, insert)
│   ├── ddl.rs                     # DDL generation trait + implementations
│   ├── migration.rs               # Cross-engine migration orchestrator
│   ├── type_mapping.rs            # Cross-engine type mapping matrix
│   └── progress.rs                # Progress reporting via Tauri events
├── commands/
│   ├── transfer.rs                # New: Tauri commands for transfer
│   └── ...
└── ...
```

### Rust Types

```rust
// transfer/types.rs

use serde::{Deserialize, Serialize};

// ── Export ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Csv,
    Jsonl,
    Sql,
    Excel,
}

/// CSV options — all fields have sensible defaults.
/// The frontend sends these only when the user explicitly overrides via "Advanced Options".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvExportOptions {
    #[serde(default = "default_comma")]
    pub delimiter: char,
    #[serde(default = "default_double_quote")]
    pub quote_char: char,
    #[serde(default = "default_utf8")]
    pub encoding: String,
    #[serde(default = "default_true")]
    pub include_header: bool,
    #[serde(default)]
    pub quote_all: bool,
    #[serde(default = "default_lf")]
    pub line_ending: String,
}

/// JSONL (JSON Lines) options — one JSON object per line, optimized for large datasets.
/// Simpler than JSON: no structure choice, no pretty print (always compact, one line per record).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonlExportOptions {
    #[serde(default = "default_iso8601")]
    pub date_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlExportOptions {
    pub target_table: String,
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
    #[serde(default = "default_true")]
    pub include_create_table: bool,
    #[serde(default)]
    pub include_drop_table: bool,
    pub target_engine: Option<String>,
}

/// Excel options — best-practice defaults, no user customization needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelExportOptions {
    #[serde(default = "default_sheet_name")]
    pub sheet_name: String,
    #[serde(default = "default_true")]
    pub include_header: bool,
    #[serde(default = "default_true")]
    pub auto_fit_columns: bool,
    #[serde(default = "default_true")]
    pub freeze_header: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub connection_id: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub source: ExportSource,
    pub format: ExportFormat,
    pub csv_options: Option<CsvExportOptions>,
    pub jsonl_options: Option<JsonlExportOptions>,
    pub sql_options: Option<SqlExportOptions>,
    pub excel_options: Option<ExcelExportOptions>,
    pub output_path: String,
}

/// Export source is always a table (Custom Query removed for simplicity).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSource {
    pub table: String,
    pub columns: Vec<String>,
    pub where_clause: Option<String>,
    pub order_by: Option<String>,
    pub limit: Option<u64>,
}

// ── Import ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportFormat {
    Csv,
    Jsonl,
    Sql,
    Excel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    pub source_column: String,
    pub target_column: Option<String>,  // None = skip
    pub target_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictStrategy {
    Skip,
    Replace,
    Upsert,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub connection_id: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub table: String,
    pub file_path: String,
    pub format: ImportFormat,
    pub column_mappings: Vec<ColumnMapping>,
    pub conflict_strategy: ConflictStrategy,
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
    pub create_table: bool,
    pub truncate_before: bool,
    pub dry_run: bool,
    pub csv_options: Option<CsvImportOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportOptions {
    #[serde(default = "default_comma")]
    pub delimiter: char,
    #[serde(default = "default_utf8")]
    pub encoding: String,
    #[serde(default = "default_true")]
    pub has_header: bool,
}

// ── DDL ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DdlRequest {
    pub connection_id: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub tables: Vec<String>,
    pub target_engine: Option<String>,
    pub options: DdlOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DdlOptions {
    pub include_create_table: bool,
    pub include_primary_keys: bool,
    pub include_foreign_keys: bool,
    pub include_indexes: bool,
    pub include_constraints: bool,
    pub include_comments: bool,
    pub include_storage: bool,
    pub include_drop_if_exists: bool,
    pub include_if_not_exists: bool,
    pub include_data: bool,
}

// ── Run SQL File ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSqlFileRequest {
    pub connection_id: String,
    pub database: Option<String>,
    pub file_path: String,
    pub wrap_in_transaction: bool,
    pub on_error: SqlFileErrorStrategy,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SqlFileErrorStrategy {
    Rollback,
    SkipAndContinue,
    Stop,
}

// ── Migration ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationMapping {
    pub source_column: String,
    pub source_type: String,
    pub target_column: String,
    pub target_type: String,
    pub conversion: MigrationConversion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MigrationConversion {
    Direct,      // Types are compatible, no conversion needed
    Mapped,      // Automatic type mapping applied
    Custom,      // User-defined mapping
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationTablePlan {
    pub source_table: String,
    pub target_table: String,
    pub column_mappings: Vec<MigrationMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRequest {
    pub source_connection_id: String,
    pub source_database: Option<String>,
    pub source_schema: Option<String>,
    pub target_connection_id: String,
    pub target_database: Option<String>,
    pub target_schema: Option<String>,
    pub table_plans: Vec<MigrationTablePlan>,
    pub batch_size: u32,
    pub on_error: MigrationErrorStrategy,
    pub create_tables: bool,
    pub drop_tables: bool,
    pub migrate_indexes: bool,
    pub migrate_foreign_keys: bool,
    pub migrate_constraints: bool,
    pub disable_fk_checks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MigrationErrorStrategy {
    SkipRow,
    SkipTable,
    Abort,
}

// ── Progress ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub operation: String,          // "export" | "import" | "ddl" | "sql_file" | "migration"
    pub phase: String,              // "preparing" | "processing" | "finalizing"
    pub current_table: Option<String>,
    pub total_rows: Option<u64>,
    pub processed_rows: u64,
    pub skipped_rows: u64,
    pub error_count: u64,
    pub percent: f32,               // 0.0–100.0
    pub elapsed_ms: u64,
    pub estimated_remaining_ms: Option<u64>,
    pub message: Option<String>,
}

// ── Results ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferResult {
    pub success: bool,
    pub total_rows: u64,
    pub processed_rows: u64,
    pub skipped_rows: u64,
    pub error_count: u64,
    pub duration_ms: u64,
    pub output_path: Option<String>,
    pub output_size_bytes: Option<u64>,
    pub errors: Vec<TransferError>,
    pub table_results: Option<Vec<TableTransferResult>>,  // For migration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableTransferResult {
    pub table: String,
    pub rows: u64,
    pub success: bool,
    pub duration_ms: u64,
    pub errors: Vec<TransferError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferError {
    pub row_number: Option<u64>,
    pub statement_number: Option<u64>,
    pub message: String,
    pub sql: Option<String>,
}

// ── Preview / Detection ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDetectionResult {
    pub format: ImportFormat,
    pub encoding: String,
    pub estimated_rows: Option<u64>,
    pub file_size_bytes: u64,
    pub columns: Vec<String>,
    pub csv_delimiter: Option<char>,
    pub has_header: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub total_rows_estimate: Option<u64>,
    pub formatted_preview: String,  // First N rows in target format
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeMappingSuggestion {
    pub source_column: String,
    pub source_type: String,
    pub target_type: String,
    pub conversion: MigrationConversion,
    pub warning: Option<String>,  // e.g., "Precision loss possible"
}

// ── Default value functions ───────────────────────────────

fn default_comma() -> char { ',' }
fn default_double_quote() -> char { '"' }
fn default_utf8() -> String { "UTF-8".to_string() }
fn default_lf() -> String { "lf".to_string() }
fn default_true() -> bool { true }
fn default_iso8601() -> String { "iso8601".to_string() }
fn default_batch_size() -> u32 { 1000 }
fn default_sheet_name() -> String { "Sheet1".to_string() }
```

### Best-Practice Defaults

```rust
// transfer/defaults.rs

use super::types::*;

/// Returns default CSV export options (best-practice).
pub fn csv_defaults() -> CsvExportOptions {
    CsvExportOptions {
        delimiter: ',',
        quote_char: '"',
        encoding: "UTF-8".to_string(),
        include_header: true,
        quote_all: false,
        line_ending: "lf".to_string(),
    }
}

/// Returns default JSONL export options (best-practice).
/// JSONL is always compact (one JSON object per line), no structure/pretty-print choice.
pub fn jsonl_defaults() -> JsonlExportOptions {
    JsonlExportOptions {
        date_format: "iso8601".to_string(),
    }
}

/// Returns default SQL export options (best-practice).
pub fn sql_defaults(table_name: &str) -> SqlExportOptions {
    SqlExportOptions {
        target_table: table_name.to_string(),
        batch_size: 1000,
        include_create_table: true,
        include_drop_table: false,
        target_engine: None,
    }
}

/// Returns default Excel export options (best-practice).
pub fn excel_defaults() -> ExcelExportOptions {
    ExcelExportOptions {
        sheet_name: "Sheet1".to_string(),
        include_header: true,
        auto_fit_columns: true,
        freeze_header: true,
    }
}

/// Returns default CSV import options (best-practice).
pub fn csv_import_defaults() -> CsvImportOptions {
    CsvImportOptions {
        delimiter: ',',
        encoding: "UTF-8".to_string(),
        has_header: true,
    }
}
```

### DDL Generator Trait

```rust
// transfer/ddl.rs

use async_trait::async_trait;
use crate::database::types::ColumnInfo;

#[async_trait]
pub trait DdlGenerator: Send + Sync {
    /// Generate CREATE TABLE statement for the given columns.
    fn generate_create_table(
        &self,
        schema: Option<&str>,
        table: &str,
        columns: &[ColumnInfo],
        options: &DdlOptions,
    ) -> String;

    /// Generate DROP TABLE statement.
    fn generate_drop_table(
        &self,
        schema: Option<&str>,
        table: &str,
        cascade: bool,
    ) -> String;

    /// Generate CREATE INDEX statements.
    fn generate_indexes(
        &self,
        schema: Option<&str>,
        table: &str,
        indexes: &[IndexInfo],
    ) -> Vec<String>;

    /// Map a source column type to this engine's equivalent.
    fn map_type(&self, source_type: &str, source_engine: &str) -> String;

    /// Generate INSERT statement for a batch of rows.
    fn generate_insert(
        &self,
        schema: Option<&str>,
        table: &str,
        columns: &[String],
        rows: &[Vec<String>],
    ) -> String;

    /// Get the engine name (e.g., "PostgreSQL", "MySQL").
    fn engine_name(&self) -> &str;

    /// Quote an identifier for this engine.
    fn quote_identifier(&self, name: &str) -> String;
}
```

Implementations:
- `PostgresDdlGenerator`
- `MySqlDdlGenerator`
- `SqliteDdlGenerator`
- `SqlServerDdlGenerator`

### Tauri Commands (12 total)

```rust
// commands/transfer.rs

use tauri::State;
use crate::state::AppState;
use crate::transfer::types::*;

// ── Export Commands ───────────────────────────────────────

/// Preview export data (first N rows in target format).
#[tauri::command]
pub async fn preview_export(
    request: ExportRequest,
    preview_rows: u32,
    state: State<'_, AppState>,
) -> Result<ExportPreview, String> { ... }

/// Execute data export to file.
#[tauri::command]
pub async fn execute_export(
    request: ExportRequest,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> { ... }

// ── Import Commands ──────────────────────────────────────

/// Detect file format, encoding, columns, and delimiter.
#[tauri::command]
pub async fn detect_file(
    file_path: String,
) -> Result<FileDetectionResult, String> { ... }

/// Preview parsed file data (first N rows with column mapping applied).
#[tauri::command]
pub async fn preview_import(
    file_path: String,
    format: ImportFormat,
    csv_options: Option<CsvImportOptions>,
    preview_rows: u32,
) -> Result<ExportPreview, String> { ... }

/// Execute data import from file into table.
#[tauri::command]
pub async fn execute_import(
    request: ImportRequest,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> { ... }

// ── DDL Commands ─────────────────────────────────────────

/// Generate DDL for selected objects.
#[tauri::command]
pub async fn generate_ddl(
    request: DdlRequest,
    state: State<'_, AppState>,
) -> Result<String, String> { ... }

/// Execute DDL/SQL against a connection.
#[tauri::command]
pub async fn execute_ddl(
    connection_id: String,
    database: Option<String>,
    sql: String,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> { ... }

// ── Run SQL File Commands ────────────────────────────────

/// Parse SQL file and return statement count + preview.
#[tauri::command]
pub async fn parse_sql_file(
    file_path: String,
) -> Result<SqlFileInfo, String> { ... }

/// Execute SQL file against a connection.
#[tauri::command]
pub async fn execute_sql_file(
    request: RunSqlFileRequest,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> { ... }

// ── Migration Commands ───────────────────────────────────

/// Suggest type mappings for source→target migration.
#[tauri::command]
pub async fn suggest_type_mappings(
    source_connection_id: String,
    source_database: Option<String>,
    source_schema: Option<String>,
    source_tables: Vec<String>,
    target_engine: String,
    state: State<'_, AppState>,
) -> Result<Vec<MigrationTablePlan>, String> { ... }

/// Execute cross-engine data migration.
#[tauri::command]
pub async fn execute_migration(
    request: MigrationRequest,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferResult, String> { ... }

// ── Shared Commands ──────────────────────────────────────

/// Cancel a running transfer operation.
#[tauri::command]
pub async fn cancel_transfer(
    operation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> { ... }
```

### Progress Reporting

Progress is reported via Tauri events so the frontend receives real-time updates without polling.

```rust
// transfer/progress.rs

use tauri::Emitter;

pub const TRANSFER_PROGRESS_EVENT: &str = "transfer-progress";

pub fn emit_progress(
    app_handle: &tauri::AppHandle,
    progress: &TransferProgress,
) {
    let _ = app_handle.emit(TRANSFER_PROGRESS_EVENT, progress);
}
```

### Recommended Rust Crates

| Crate | Purpose | Version |
|-------|---------|---------|
| `csv` | CSV reading/writing | Latest |
| `serde_json` | JSONL reading/writing (one JSON object per line) | (already in deps) |
| `calamine` | Excel reading (.xlsx, .xls) | Latest |
| `rust_xlsxwriter` | Excel writing (.xlsx) | Latest |
| `encoding_rs` | Character encoding detection/conversion | Latest |
| `chardetng` | Automatic charset detection | Latest |

### Streaming / Chunking Strategy

For large datasets (100k+ rows), all operations use batched processing:

1. **Export**: Query rows in pages (`LIMIT batch_size OFFSET n`), write each page to the output file incrementally.
2. **Import**: Read the file in chunks (e.g., 5,000 rows per batch), execute batched INSERTs within a transaction per batch.
3. **Migration**: Read from source in batches, type-convert in memory, write to target in batches. Per-table sequential, within-table batched.

**Default batch size**: 5,000 rows. Configurable by the user.

**Memory bound**: At most one batch worth of rows is held in memory at any time.

---

## Frontend Architecture

### Vue Component Hierarchy

```
src/
├── pages/
│   └── TransferPage.vue                     # Top-level page (replaces ImportExportPage.vue)
├── components/
│   └── transfer/
│       ├── index.ts                          # Module exports
│       │
│       ├── TransferTabs.vue                  # Top-level tab container (Export|Import|Structure|Migration)
│       │
│       ├── shared/
│       │   ├── ConnectionSelector.vue        # Connection + database + schema dropdowns
│       │   ├── TableSelector.vue             # Table list with checkboxes
│       │   ├── ColumnSelector.vue            # Column list with checkboxes
│       │   ├── WizardStepper.vue             # Step indicator bar
│       │   ├── ProgressPanel.vue             # Progress bar + stats + cancel + "Run in Background"
│       │   ├── ResultPanel.vue               # Completion summary + error log
│       │   ├── FileDropZone.vue              # Drag-and-drop file area
│       │   └── FormatPreview.vue             # Preview formatted data (Monaco read-only)
│       │
│       ├── export/
│       │   ├── ExportWizard.vue              # Export wizard container (4 steps)
│       │   ├── ExportSourceStep.vue          # Step 1: Table + columns + filters
│       │   ├── ExportFormatStep.vue          # Step 2: Format selection (defaults applied)
│       │   ├── ExportPreviewStep.vue         # Step 3: Preview
│       │   └── ExportExecuteStep.vue         # Step 4: Execute + results
│       │
│       ├── import/
│       │   ├── ImportWizard.vue              # Import wizard container (4 steps)
│       │   ├── ImportFileStep.vue            # Step 1: File selection (auto-detect)
│       │   ├── ImportMappingStep.vue         # Step 2: Target & column mapping
│       │   ├── ImportOptionsStep.vue         # Step 3: Options & preview
│       │   └── ImportExecuteStep.vue         # Step 4: Execute + results
│       │
│       ├── structure/
│       │   ├── StructureTabs.vue             # Sub-tab container (Generate DDL | Run SQL File)
│       │   ├── DdlWizard.vue                # DDL wizard container (3 steps)
│       │   ├── DdlObjectStep.vue            # Step 1: Object selection
│       │   ├── DdlOptionsStep.vue           # Step 2: DDL options
│       │   ├── DdlPreviewStep.vue           # Step 3: Preview & export
│       │   ├── SqlFileWizard.vue             # SQL file wizard (2 steps)
│       │   ├── SqlFileSelectStep.vue         # Step 1: File & connection
│       │   └── SqlFileExecuteStep.vue        # Step 2: Execution & results
│       │
│       ├── migration/
│       │   ├── MigrationWizard.vue           # Migration wizard container (5 steps)
│       │   ├── MigrationSourceStep.vue       # Step 1: Source connection + tables
│       │   ├── MigrationTargetStep.vue       # Step 2: Target connection
│       │   ├── MigrationMappingStep.vue      # Step 3: Schema & type mapping
│       │   ├── MigrationConfigStep.vue       # Step 4: Options & summary
│       │   └── MigrationExecuteStep.vue      # Step 5: Execute + results
│       │
│       └── tasks/
│           ├── TaskManagerPanel.vue          # Slide-out sidebar (400px) with task list
│           ├── TaskCard.vue                  # Individual task card with progress + actions
│           └── TaskManagerButton.vue         # Header button showing task count badge
│
├── store/
│   └── transferStore.ts                      # Pinia store (wizard state + task management)
│
├── datasources/
│   └── transferApi.ts                        # Tauri invoke wrappers
│
└── types/
    └── transfer.ts                           # TypeScript types
```

### TypeScript Types

```typescript
// src/types/transfer.ts

// ── Export ────────────────────────────────────────────────

export type ExportFormat = 'csv' | 'jsonl' | 'sql' | 'excel'

export type ExportSource = {
  table: string
  columns: string[]
  whereClause?: string
  orderBy?: string
  limit?: number
}

export type ExportRequest = {
  connectionId: string
  database?: string
  schema?: string
  source: ExportSource
  format: ExportFormat
  csvOptions?: CsvExportOptions
  jsonlOptions?: JsonlExportOptions
  sqlOptions?: SqlExportOptions
  excelOptions?: ExcelExportOptions
  outputPath: string
}

// Format options — only sent when user explicitly overrides via "Advanced Options"

export type CsvExportOptions = {
  delimiter?: string       // default: ','
  quoteChar?: string       // default: '"'
  encoding?: string        // default: 'UTF-8'
  includeHeader?: boolean  // default: true
  quoteAll?: boolean       // default: false
  lineEnding?: 'lf' | 'crlf'  // default: 'lf'
}

export type JsonlExportOptions = {
  dateFormat?: string      // default: 'iso8601'
}

export type SqlExportOptions = {
  targetTable: string
  batchSize?: number          // default: 1000
  includeCreateTable?: boolean  // default: true
  includeDropTable?: boolean    // default: false
  targetEngine?: string
}

export type ExcelExportOptions = {
  sheetName?: string          // default: 'Sheet1'
  includeHeader?: boolean     // default: true
  autoFitColumns?: boolean    // default: true
  freezeHeader?: boolean      // default: true
}

// ── Import ────────────────────────────────────────────────

export type ImportFormat = 'csv' | 'jsonl' | 'sql' | 'excel'

export type ColumnMapping = {
  sourceColumn: string
  targetColumn?: string
  targetType?: string
}

export type ConflictStrategy = 'skip' | 'replace' | 'upsert' | 'abort'

export type CsvImportOptions = {
  delimiter?: string     // default: auto-detected or ','
  encoding?: string      // default: auto-detected or 'UTF-8'
  hasHeader?: boolean    // default: true
}

export type ImportRequest = {
  connectionId: string
  database?: string
  schema?: string
  table: string
  filePath: string
  format: ImportFormat
  columnMappings: ColumnMapping[]
  conflictStrategy: ConflictStrategy
  batchSize: number
  createTable: boolean
  truncateBefore: boolean
  dryRun: boolean
  csvOptions?: CsvImportOptions
}

// ── DDL ───────────────────────────────────────────────────

export type DdlOptions = {
  includeCreateTable: boolean
  includePrimaryKeys: boolean
  includeForeignKeys: boolean
  includeIndexes: boolean
  includeConstraints: boolean
  includeComments: boolean
  includeStorage: boolean
  includeDropIfExists: boolean
  includeIfNotExists: boolean
  includeData: boolean
}

export type DdlRequest = {
  connectionId: string
  database?: string
  schema?: string
  tables: string[]
  targetEngine?: string
  options: DdlOptions
}

// ── Run SQL File ──────────────────────────────────────────

export type SqlFileErrorStrategy = 'rollback' | 'skipAndContinue' | 'stop'

export type RunSqlFileRequest = {
  connectionId: string
  database?: string
  filePath: string
  wrapInTransaction: boolean
  onError: SqlFileErrorStrategy
  dryRun: boolean
}

export type SqlFileInfo = {
  filePath: string
  fileSizeBytes: number
  statementCount: number
  previewLines: string[]
}

// ── Migration ─────────────────────────────────────────────

export type MigrationConversion = 'direct' | 'mapped' | 'custom'

export type MigrationMapping = {
  sourceColumn: string
  sourceType: string
  targetColumn: string
  targetType: string
  conversion: MigrationConversion
}

export type MigrationTablePlan = {
  sourceTable: string
  targetTable: string
  columnMappings: MigrationMapping[]
}

export type MigrationErrorStrategy = 'skipRow' | 'skipTable' | 'abort'

export type MigrationRequest = {
  sourceConnectionId: string
  sourceDatabase?: string
  sourceSchema?: string
  targetConnectionId: string
  targetDatabase?: string
  targetSchema?: string
  tablePlans: MigrationTablePlan[]
  batchSize: number
  onError: MigrationErrorStrategy
  createTables: boolean
  dropTables: boolean
  migrateIndexes: boolean
  migrateForeignKeys: boolean
  migrateConstraints: boolean
  disableFkChecks: boolean
}

// ── Progress ──────────────────────────────────────────────

export type TransferProgress = {
  operation: string
  phase: string
  currentTable?: string
  totalRows?: number
  processedRows: number
  skippedRows: number
  errorCount: number
  percent: number
  elapsedMs: number
  estimatedRemainingMs?: number
  message?: string
}

// ── Results ───────────────────────────────────────────────

export type TransferError = {
  rowNumber?: number
  statementNumber?: number
  message: string
  sql?: string
}

export type TableTransferResult = {
  table: string
  rows: number
  success: boolean
  durationMs: number
  errors: TransferError[]
}

export type TransferResult = {
  success: boolean
  totalRows: number
  processedRows: number
  skippedRows: number
  errorCount: number
  durationMs: number
  outputPath?: string
  outputSizeBytes?: number
  errors: TransferError[]
  tableResults?: TableTransferResult[]
}

// ── Detection ─────────────────────────────────────────────

export type FileDetectionResult = {
  format: ImportFormat
  encoding: string
  estimatedRows?: number
  fileSizeBytes: number
  columns: string[]
  csvDelimiter?: string
  hasHeader?: boolean
}

export type ExportPreview = {
  columns: string[]
  sampleRows: string[][]
  totalRowsEstimate?: number
  formattedPreview: string
}

export type TypeMappingSuggestion = {
  sourceColumn: string
  sourceType: string
  targetType: string
  conversion: MigrationConversion
  warning?: string
}

// ── Background Tasks ──────────────────────────────────────

export type TaskKind = 'export' | 'import' | 'sqlFile' | 'migration'

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed'

export type TaskRuntime = {
  complete: number
  total: number
  skipped: number
  errorCount: number
}

export type ExportTaskConfig = {
  connectionId: string
  database?: string
  schema?: string
  table: string
  columns: string[]
  whereClause?: string
  orderBy?: string
  limit?: number
  format: ExportFormat
  outputPath: string
}

export type ImportTaskConfig = {
  connectionId: string
  database?: string
  schema?: string
  table: string
  filePath: string
  format: ImportFormat
  conflictStrategy: ConflictStrategy
}

export type SqlFileTaskConfig = {
  connectionId: string
  database?: string
  filePath: string
  onError: SqlFileErrorStrategy
}

export type MigrationTaskConfig = {
  sourceConnectionId: string
  sourceDatabase?: string
  targetConnectionId: string
  targetDatabase?: string
  tables: string[]
}

export type TaskConfig = ExportTaskConfig | ImportTaskConfig | SqlFileTaskConfig | MigrationTaskConfig

export type BackgroundTask = {
  id: string
  kind: TaskKind
  status: TaskStatus
  progress: { complete: number; total: number }
  config: TaskConfig
  runtime: TaskRuntime
  label: string
  startTime: Date
  endTime?: Date
  error?: string
}
```

### Pinia Store

```typescript
// src/store/transferStore.ts

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  ExportRequest,
  ImportRequest,
  MigrationRequest,
  TransferProgress,
  TransferResult,
  BackgroundTask,
  TaskKind,
  TaskStatus,
  TaskRuntime,
} from '@/types/transfer'

export const useTransferStore = defineStore('transfer', () => {
  // ── Active Tab ────────────────────────────────
  const activeTab = ref<'export' | 'import' | 'structure' | 'migration'>('export')

  // ── Progress Tracking ─────────────────────────
  const isRunning = ref(false)
  const progress = ref<TransferProgress | null>(null)
  const lastResult = ref<TransferResult | null>(null)
  const operationId = ref<string | null>(null)

  // ── Export State ──────────────────────────────
  const exportStep = ref(0)
  const exportRequest = ref<Partial<ExportRequest>>({})

  // ── Import State ──────────────────────────────
  const importStep = ref(0)
  const importRequest = ref<Partial<ImportRequest>>({})

  // ── Migration State ───────────────────────────
  const migrationStep = ref(0)
  const migrationRequest = ref<Partial<MigrationRequest>>({})

  // ── Background Task State ─────────────────────
  const runningTasks = ref<BackgroundTask[]>([])
  const activeExportTaskId = ref<string | null>(null)
  const activeImportTaskId = ref<string | null>(null)
  const activeSqlFileTaskId = ref<string | null>(null)
  const activeMigrationTaskId = ref<string | null>(null)

  // ── Computed ──────────────────────────────────
  const progressPercent = computed(() => progress.value?.percent ?? 0)
  const canCancel = computed(() => isRunning.value && operationId.value !== null)
  const taskCount = computed(() => runningTasks.value.length)
  const hasRunningTasks = computed(() =>
    runningTasks.value.some(t => t.status === 'running')
  )
  const activeTaskId = computed(() => {
    switch (activeTab.value) {
      case 'export': return activeExportTaskId.value
      case 'import': return activeImportTaskId.value
      case 'structure': return activeSqlFileTaskId.value
      case 'migration': return activeMigrationTaskId.value
      default: return null
    }
  })

  // ── Tab Actions ───────────────────────────────
  const setActiveTab = (tab: typeof activeTab.value) => {
    activeTab.value = tab
  }

  // ── Progress Actions ──────────────────────────
  const updateProgress = (p: TransferProgress) => {
    progress.value = p
  }

  const startOperation = (id: string) => {
    operationId.value = id
    isRunning.value = true
    progress.value = null
    lastResult.value = null
  }

  const completeOperation = (result: TransferResult) => {
    isRunning.value = false
    lastResult.value = result
    progress.value = null
    operationId.value = null
  }

  // ── Reset Actions ─────────────────────────────
  const resetExport = () => {
    exportStep.value = 0
    exportRequest.value = {}
    lastResult.value = null
  }

  const resetImport = () => {
    importStep.value = 0
    importRequest.value = {}
    lastResult.value = null
  }

  const resetMigration = () => {
    migrationStep.value = 0
    migrationRequest.value = {}
    lastResult.value = null
  }

  // ── Task Management Actions ───────────────────

  const addRunningTask = (task: BackgroundTask) => {
    runningTasks.value = [...runningTasks.value, task]
  }

  const updateTaskRuntime = (taskId: string, runtime: Partial<TaskRuntime>) => {
    runningTasks.value = runningTasks.value.map(t =>
      t.id === taskId
        ? {
            ...t,
            runtime: { ...t.runtime, ...runtime },
            progress: {
              complete: runtime.complete ?? t.progress.complete,
              total: runtime.total ?? t.progress.total,
            },
          }
        : t
    )
  }

  const updateTaskStatus = (taskId: string, status: TaskStatus, error?: string) => {
    runningTasks.value = runningTasks.value.map(t =>
      t.id === taskId
        ? {
            ...t,
            status,
            endTime: status === 'completed' || status === 'failed' ? new Date() : undefined,
            error,
          }
        : t
    )
  }

  const removeTask = (taskId: string) => {
    runningTasks.value = runningTasks.value.filter(t => t.id !== taskId)
  }

  const clearCompletedTasks = () => {
    runningTasks.value = runningTasks.value.filter(t =>
      t.status === 'running' || t.status === 'pending'
    )
  }

  const syncProgressToTask = (taskId: string, p: TransferProgress) => {
    updateTaskRuntime(taskId, {
      complete: p.processedRows,
      total: p.totalRows ?? 0,
      skipped: p.skippedRows,
      errorCount: p.errorCount,
    })
  }

  const detachActiveTask = (kind: TaskKind) => {
    switch (kind) {
      case 'export': activeExportTaskId.value = null; break
      case 'import': activeImportTaskId.value = null; break
      case 'sqlFile': activeSqlFileTaskId.value = null; break
      case 'migration': activeMigrationTaskId.value = null; break
    }
  }

  const openTask = (taskId: string) => {
    const task = runningTasks.value.find(t => t.id === taskId)
    if (!task) return

    // Map task kind to tab
    const tabMap: Record<TaskKind, typeof activeTab.value> = {
      export: 'export',
      import: 'import',
      sqlFile: 'structure',
      migration: 'migration',
    }
    activeTab.value = tabMap[task.kind]

    // Restore form state from task.config
    // The specific restoration logic depends on task.kind
    // and populates the corresponding wizard step fields
    switch (task.kind) {
      case 'export':
        activeExportTaskId.value = taskId
        // Restore exportRequest from task.config
        break
      case 'import':
        activeImportTaskId.value = taskId
        // Restore importRequest from task.config
        break
      case 'sqlFile':
        activeSqlFileTaskId.value = taskId
        break
      case 'migration':
        activeMigrationTaskId.value = taskId
        // Restore migrationRequest from task.config
        break
    }
  }

  return {
    // Tab
    activeTab,
    setActiveTab,

    // Progress
    isRunning,
    progress,
    lastResult,
    operationId,
    progressPercent,
    canCancel,
    updateProgress,
    startOperation,
    completeOperation,

    // Wizard state
    exportStep,
    exportRequest,
    importStep,
    importRequest,
    migrationStep,
    migrationRequest,
    resetExport,
    resetImport,
    resetMigration,

    // Task management
    runningTasks,
    activeExportTaskId,
    activeImportTaskId,
    activeSqlFileTaskId,
    activeMigrationTaskId,
    taskCount,
    hasRunningTasks,
    activeTaskId,
    addRunningTask,
    updateTaskRuntime,
    updateTaskStatus,
    removeTask,
    clearCompletedTasks,
    syncProgressToTask,
    detachActiveTask,
    openTask,
  }
})
```

### Frontend API Wrapper

```typescript
// src/datasources/transferApi.ts

import { invoke } from '@tauri-apps/api/core'
import type {
  ExportRequest,
  ExportPreview,
  ImportRequest,
  CsvImportOptions,
  ImportFormat,
  DdlRequest,
  RunSqlFileRequest,
  SqlFileInfo,
  MigrationRequest,
  MigrationTablePlan,
  FileDetectionResult,
  TransferResult,
} from '@/types/transfer'

// ── Export ────────────────────────────────────────────────

export const previewExport = (request: ExportRequest, previewRows = 10) =>
  invoke<ExportPreview>('preview_export', { request, previewRows })

export const executeExport = (request: ExportRequest) =>
  invoke<TransferResult>('execute_export', { request })

// ── Import ────────────────────────────────────────────────

export const detectFile = (filePath: string) =>
  invoke<FileDetectionResult>('detect_file', { filePath })

export const previewImport = (
  filePath: string,
  format: ImportFormat,
  csvOptions?: CsvImportOptions,
  previewRows = 10,
) => invoke<ExportPreview>('preview_import', { filePath, format, csvOptions, previewRows })

export const executeImport = (request: ImportRequest) =>
  invoke<TransferResult>('execute_import', { request })

// ── DDL ───────────────────────────────────────────────────

export const generateDdl = (request: DdlRequest) =>
  invoke<string>('generate_ddl', { request })

export const executeDdl = (connectionId: string, database: string | undefined, sql: string) =>
  invoke<TransferResult>('execute_ddl', { connectionId, database, sql })

// ── Run SQL File ──────────────────────────────────────────

export const parseSqlFile = (filePath: string) =>
  invoke<SqlFileInfo>('parse_sql_file', { filePath })

export const executeSqlFile = (request: RunSqlFileRequest) =>
  invoke<TransferResult>('execute_sql_file', { request })

// ── Migration ─────────────────────────────────────────────

export const suggestTypeMappings = (
  sourceConnectionId: string,
  sourceDatabase: string | undefined,
  sourceSchema: string | undefined,
  sourceTables: string[],
  targetEngine: string,
) => invoke<MigrationTablePlan[]>('suggest_type_mappings', {
  sourceConnectionId,
  sourceDatabase,
  sourceSchema,
  sourceTables,
  targetEngine,
})

export const executeMigration = (request: MigrationRequest) =>
  invoke<TransferResult>('execute_migration', { request })

// ── Shared ────────────────────────────────────────────────

export const cancelTransfer = (operationId: string) =>
  invoke<void>('cancel_transfer', { operationId })
```

### Progress Event Listener

```typescript
// Usage in component (e.g., ExportExecuteStep.vue)

import { listen } from '@tauri-apps/api/event'
import { useTransferStore } from '@/store/transferStore'
import type { TransferProgress } from '@/types/transfer'

const transferStore = useTransferStore()

const unlisten = await listen<TransferProgress>('transfer-progress', event => {
  transferStore.updateProgress(event.payload)

  // Sync progress to background task if one is active
  const taskId = transferStore.activeExportTaskId
  if (taskId) {
    transferStore.syncProgressToTask(taskId, event.payload)
  }
})

// Cleanup on unmount
onUnmounted(() => {
  unlisten()
})
```

---

## i18n Keys

```json
{
  "transfer.title": "Transfer",
  "transfer.subtitle": "Import, export, and migrate your data",

  "transfer.tabs.export": "Export",
  "transfer.tabs.import": "Import",
  "transfer.tabs.structure": "Structure",
  "transfer.tabs.migration": "Migration",

  "transfer.export.step.source": "Source",
  "transfer.export.step.format": "Format",
  "transfer.export.step.preview": "Preview",
  "transfer.export.step.execute": "Export",
  "transfer.export.columns.selectAll": "Select All",
  "transfer.export.columns.deselectAll": "Deselect All",
  "transfer.export.where": "WHERE clause (optional)",
  "transfer.export.orderBy": "ORDER BY (optional)",
  "transfer.export.limit": "LIMIT (optional)",

  "transfer.format.csv": "CSV (.csv)",
  "transfer.format.jsonl": "JSONL (.jsonl)",
  "transfer.format.sql": "SQL (.sql)",
  "transfer.format.excel": "Excel (.xlsx)",
  "transfer.format.defaults.csv": "Comma delimiter, double-quote, UTF-8, include header, LF line ending",
  "transfer.format.defaults.jsonl": "One JSON object per line, compact, UTF-8, ISO 8601 dates",
  "transfer.format.defaults.sql": "Auto-filled target table, batch size 1000, include CREATE TABLE",
  "transfer.format.defaults.excel": "Include header, auto-fit columns, freeze header row",
  "transfer.format.advancedOptions": "Advanced Options",

  "transfer.import.step.file": "Select File",
  "transfer.import.step.mapping": "Target & Mapping",
  "transfer.import.step.options": "Options & Preview",
  "transfer.import.step.execute": "Import",
  "transfer.import.dropzone.title": "Drag & drop a file here",
  "transfer.import.dropzone.subtitle": "or click to browse",
  "transfer.import.dropzone.supported": "Supported: CSV, JSONL, SQL, Excel (.xlsx)",
  "transfer.import.detected.format": "Detected Format",
  "transfer.import.detected.encoding": "Detected Encoding",
  "transfer.import.detected.rows": "Rows (estimated)",
  "transfer.import.advancedParseOptions": "Advanced Parse Options",
  "transfer.import.createTable": "Create table if not exists",
  "transfer.import.autoMap": "Auto-Map by Name",
  "transfer.import.clearAll": "Clear All",
  "transfer.import.conflict": "On Conflict",
  "transfer.import.conflict.skip": "Skip duplicates",
  "transfer.import.conflict.replace": "Replace existing",
  "transfer.import.conflict.upsert": "Update existing (upsert)",
  "transfer.import.conflict.abort": "Abort on error",
  "transfer.import.truncateBefore": "Truncate table before import",
  "transfer.import.dryRun": "Dry run (validate without inserting)",

  "transfer.structure.tabs.ddl": "Generate DDL",
  "transfer.structure.tabs.sqlFile": "Run SQL File",
  "transfer.structure.ddl.step.objects": "Select Objects",
  "transfer.structure.ddl.step.options": "DDL Options",
  "transfer.structure.ddl.step.preview": "Preview & Export",
  "transfer.structure.ddl.targetEngine": "Target Engine",
  "transfer.structure.ddl.sameAsSource": "Same as source",
  "transfer.structure.ddl.includeCreate": "CREATE TABLE statements",
  "transfer.structure.ddl.includePk": "Primary keys",
  "transfer.structure.ddl.includeFk": "Foreign keys",
  "transfer.structure.ddl.includeIndexes": "Indexes",
  "transfer.structure.ddl.includeConstraints": "Constraints (UNIQUE, CHECK, NOT NULL)",
  "transfer.structure.ddl.includeComments": "Comments / descriptions",
  "transfer.structure.ddl.includeStorage": "Tablespace / storage options",
  "transfer.structure.ddl.includeDrop": "Include DROP IF EXISTS before CREATE",
  "transfer.structure.ddl.includeIfNotExists": "Include IF NOT EXISTS on CREATE",
  "transfer.structure.ddl.includeData": "Include INSERT DATA",
  "transfer.structure.ddl.copyClipboard": "Copy to Clipboard",
  "transfer.structure.ddl.saveFile": "Save to File",
  "transfer.structure.ddl.executeServer": "Execute on Server",
  "transfer.structure.sqlFile.step.select": "Select File & Connection",
  "transfer.structure.sqlFile.step.execute": "Execution",
  "transfer.structure.sqlFile.dropzone.title": "Drag & drop a .sql file here",
  "transfer.structure.sqlFile.dropzone.supported": "Supported: .sql files",
  "transfer.structure.sqlFile.wrapTransaction": "Wrap in transaction",
  "transfer.structure.sqlFile.onError": "On Error",
  "transfer.structure.sqlFile.onError.rollback": "Rollback all",
  "transfer.structure.sqlFile.onError.skip": "Skip and continue",
  "transfer.structure.sqlFile.onError.stop": "Stop execution",
  "transfer.structure.sqlFile.dryRun": "Dry run (parse only)",

  "transfer.migration.step.source": "Source",
  "transfer.migration.step.target": "Target",
  "transfer.migration.step.mapping": "Mapping",
  "transfer.migration.step.configure": "Configure",
  "transfer.migration.step.execute": "Migrate",
  "transfer.migration.createTables": "Create target tables if not exist",
  "transfer.migration.dropTables": "Drop target tables before migration",
  "transfer.migration.batchSize": "Batch Size",
  "transfer.migration.onError": "On Error",
  "transfer.migration.onError.skipRow": "Skip row and continue",
  "transfer.migration.onError.skipTable": "Skip table and continue",
  "transfer.migration.onError.abort": "Abort migration",
  "transfer.migration.migrateIndexes": "Migrate indexes",
  "transfer.migration.migrateFk": "Migrate foreign keys",
  "transfer.migration.migrateConstraints": "Migrate constraints",
  "transfer.migration.disableFkChecks": "Disable foreign key checks during migration",
  "transfer.migration.editMapping": "Edit Mapping",
  "transfer.migration.resetAuto": "Reset to Auto",
  "transfer.migration.conversion.direct": "Direct",
  "transfer.migration.conversion.mapped": "Auto-mapped",
  "transfer.migration.conversion.custom": "Custom",

  "transfer.progress.exporting": "Exporting...",
  "transfer.progress.importing": "Importing...",
  "transfer.progress.migrating": "Migrating...",
  "transfer.progress.executing": "Executing...",
  "transfer.progress.elapsed": "Elapsed",
  "transfer.progress.remaining": "Estimated remaining",
  "transfer.progress.rows": "Rows",
  "transfer.progress.exported": "exported",
  "transfer.progress.imported": "imported",
  "transfer.progress.skipped": "skipped",
  "transfer.progress.errors": "errors",
  "transfer.progress.statements": "Statements",
  "transfer.progress.succeeded": "Succeeded",
  "transfer.progress.failed": "Failed",
  "transfer.progress.cancel": "Cancel",
  "transfer.progress.runInBackground": "Run in Background",

  "transfer.result.success": "completed successfully",
  "transfer.result.partial": "completed with errors",
  "transfer.result.failed": "failed",
  "transfer.result.duration": "Duration",
  "transfer.result.file": "File",
  "transfer.result.openFile": "Open File",
  "transfer.result.openFolder": "Open Folder",
  "transfer.result.exportAgain": "Export Again",
  "transfer.result.importAgain": "Import Again",
  "transfer.result.migrateAgain": "Migrate Again",
  "transfer.result.runAgain": "Run Again",
  "transfer.result.viewTable": "View Table",
  "transfer.result.showErrorsOnly": "Show Errors Only",
  "transfer.result.copyLog": "Copy Log",

  "transfer.tasks.title": "Tasks",
  "transfer.tasks.clearCompleted": "Clear Completed",
  "transfer.tasks.goToTask": "Go to Task",
  "transfer.tasks.dismiss": "Dismiss",
  "transfer.tasks.noTasks": "No transfer tasks",
  "transfer.tasks.status.pending": "Pending",
  "transfer.tasks.status.running": "Running",
  "transfer.tasks.status.completed": "Completed",
  "transfer.tasks.status.failed": "Failed",
  "transfer.tasks.startedAgo": "Started {time} ago",

  "transfer.common.connection": "Connection",
  "transfer.common.database": "Database",
  "transfer.common.schema": "Schema",
  "transfer.common.table": "Table",
  "transfer.common.selectAll": "Select All",
  "transfer.common.deselectAll": "Deselect All",
  "transfer.common.tablesOnly": "Tables Only",
  "transfer.common.viewsOnly": "Views Only",
  "transfer.common.back": "Back",
  "transfer.common.next": "Next",
  "transfer.common.browse": "Browse..."
}
```

---

## Router & Sidebar Changes

### Router Update

```typescript
// src/router/index.ts — change:
// { path: '/import-export', ... }
// to:
{
  path: '/transfer',
  name: 'transfer',
  component: () => import('@/pages/TransferPage.vue'),
}
```

### Sidebar Update

```typescript
// In AppSidebar.vue — change:
// { label: t('sidebar.importExport'), icon: ArrowLeftRight, to: '/import-export' }
// to:
{ label: t('sidebar.transfer'), icon: ArrowLeftRight, to: '/transfer' }
```

### i18n Sidebar Key

```json
{
  "sidebar.transfer": "Transfer"
}
```

---

## Implementation Phases

### Phase 1 — MVP (Data Export + Import + Task System)

**Scope**: Export and Import tabs with CSV and JSONL support, plus background task infrastructure.

**Deliverables**:
- `src-tauri/src/transfer/` module: `mod.rs`, `types.rs`, `defaults.rs`, `export.rs`, `import.rs`, `progress.rs`
- `src-tauri/src/commands/transfer.rs`: `preview_export`, `execute_export`, `detect_file`, `preview_import`, `execute_import`, `cancel_transfer`
- `src/pages/TransferPage.vue` replacing `ImportExportPage.vue`
- `src/components/transfer/` — all shared components + export/ + import/ wizards + tasks/
- `src/store/transferStore.ts` (including full task management)
- `src/datasources/transferApi.ts`
- `src/types/transfer.ts`
- Router + sidebar updates
- i18n keys for export/import/tasks
- Formats: CSV, JSONL only (SQL, Excel deferred)
- Background task system with Task Manager panel

**Estimated effort**: 3-4 weeks

### Phase 2 — Full Export/Import + Structure

**Scope**: Complete format support + Structure tab.

**Deliverables**:
- SQL, Excel export/import support (using best-practice defaults)
- `src-tauri/src/transfer/ddl.rs` — `DdlGenerator` trait + per-engine implementations
- `src-tauri/src/commands/transfer.rs`: `generate_ddl`, `execute_ddl`, `parse_sql_file`, `execute_sql_file`
- `src/components/transfer/structure/` — DDL wizard + SQL file wizard
- i18n keys for structure tab
- New Cargo dependencies: `rust_xlsxwriter`, `calamine`, `encoding_rs`, `chardetng`

**Estimated effort**: 2-3 weeks

### Phase 3 — Cross-Engine Migration

**Scope**: Migration tab with full cross-engine support.

**Deliverables**:
- `src-tauri/src/transfer/migration.rs` — migration orchestrator
- `src-tauri/src/transfer/type_mapping.rs` — cross-engine type mapping matrix
- `src-tauri/src/commands/transfer.rs`: `suggest_type_mappings`, `execute_migration`
- `src/components/transfer/migration/` — all 5 migration wizard steps
- i18n keys for migration tab
- Per-table progress tracking with rollback support

**Estimated effort**: 2-3 weeks

---

## UI Components Used

All UI components come from the existing shadcn-vue component library in `src/components/ui/`:

| Component | Usage |
|-----------|-------|
| `Button` | Wizard navigation, actions, "Run in Background" |
| `Card` | Step containers, summary panels, task cards |
| `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent` | Top-level tabs, structure sub-tabs |
| `Select` | Connection, database, schema, format dropdowns |
| `Input` | Text inputs (table name, file path, etc.) |
| `Checkbox` | Column selection, boolean options |
| `RadioGroup` | Format selection |
| `Label` | Form labels |
| `Table` | Column mapping grid, results table |
| `Progress` | Progress bar during execution, task cards |
| `Badge` | Status indicators (mapped, skipped, error), task status |
| `Spinner` | Loading states |
| `Dialog` | Confirmation dialogs (cancel, truncate) |
| `AlertDialog` | Destructive action confirmations |
| `Tooltip` | Help text on options |
| `DropdownMenu` | Additional actions menus |
| `Notification` | Success/error toast notifications |
| `Sheet` | Task Manager slide-out panel |
| `Collapsible` | "Advanced Options" expandable sections |

---

## Revision History

| Date | Change | Reason |
|------|--------|--------|
| v1.0 | Initial design | Comprehensive Transfer feature spec |
| v2.0 | Added Background Task System | Modeled after dockit's frontend-only task management pattern |
| v2.0 | Removed Custom Query export source | Simplification — table-only export covers primary use cases |
| v2.0 | Removed XML options panel | Simplification — XML uses fixed best-practice defaults |
| v2.0 | Simplified Format & Options step | Best-practice defaults applied automatically; "Advanced Options" collapsible for power users |
| v2.0 | Removed "Disable indexes during import" option | Backend handles automatically for large imports |
| v2.0 | Added Task Manager UI | Slide-out panel for tracking background tasks with progress, status, and navigation |
| v2.0 | Added `tasks/` component directory | TaskManagerPanel, TaskCard, TaskManagerButton |
| v2.0 | Updated Pinia store | Added full task lifecycle management (add, update, sync, detach, open, clear) |
| v2.0 | Phase 1 scope expanded | Includes background task infrastructure from the start |
| v3.0 | Removed XML format | Dropped XML from both export and import — reduces complexity and crate dependencies |
| v3.0 | Replaced JSON with JSONL | JSON Lines format (`.jsonl`) for better streaming and size efficiency with large datasets |
| v3.0 | Removed SQL-specific options panel | SQL export uses auto-filled defaults (target table from source, batch size 1000, include CREATE TABLE) — no dedicated UI section |
