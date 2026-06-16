<div align="center">

<img src="app-icon.png" width="120" alt="SqlKit Logo"/>

# SqlKit

**Agentic cross-platform SQL database GUI client — your database agent for 40+ databases.**

**Privacy-first. Your data, your keys. Open source.**

[![Release](https://img.shields.io/github/v/release/geek-fun/sqlkit?color=orange&label=release&style=for-the-badge&logo=github)](https://github.com/geek-fun/sqlkit/releases)
[![Downloads](https://img.shields.io/github/downloads/geek-fun/sqlkit/total?color=orange&style=for-the-badge&logo=docusign)](https://github.com/geek-fun/sqlkit/releases)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg?style=for-the-badge&logo=apache)](LICENSE)
[![Stars](https://img.shields.io/github/stars/geek-fun/sqlkit?style=for-the-badge&logo=github)](https://github.com/geek-fun/sqlkit/stargazers)
[![CI](https://github.com/geek-fun/sqlkit/actions/workflows/node.yml/badge.svg?style=for-the-badge)](https://github.com/geek-fun/sqlkit/actions/workflows/node.yml)

<p>
  <img src="https://img.shields.io/badge/macOS-000000?style=for-the-badge&logo=apple&logoColor=white"/>
  <img src="https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white"/>
  <img src="https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black"/>
  <img src="https://img.shields.io/badge/Tauri-FFC131?style=for-the-badge&logo=tauri&logoColor=black"/>
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"/>
</p>

[Download](https://www.geekfun.club/download) · [Docs](docs/DISTRIBUTION.md) · [Website](https://www.geekfun.club) · [Releases](https://github.com/geek-fun/sqlkit/releases)

English · [简体中文](README_zh.md)

</div>

---

SqlKit is an **agentic database client** — it doesn't just execute SQL, it understands your databases and acts on your behalf. Describe what you need in natural language and the agent writes queries, inspects schemas, optimizes SQL, and returns results. Built on Tauri (Rust), not Electron, it replaces heavyweight clients like DBeaver, Navicat, and DataGrip with a single native desktop app.

<p align="center">
  <img src="docs/images/sqlkit-client-ui.png" width="800" alt="SqlKit Client UI"/>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/PostgreSQL-4169E1?logo=postgresql&logoColor=white" />
  <img src="https://img.shields.io/badge/MySQL-4479A1?logo=mysql&logoColor=white" />
  <img src="https://img.shields.io/badge/Oracle-F80000?logo=oracle&logoColor=white" />
  <img src="https://img.shields.io/badge/SQL%20Server-CC2927?logo=microsoftsqlserver&logoColor=white" />
  <img src="https://img.shields.io/badge/SQLite-003B57?logo=sqlite&logoColor=white" />
  <img src="https://img.shields.io/badge/DuckDB-FFF000?logo=duckdb&logoColor=black" />
  <img src="https://img.shields.io/badge/ClickHouse-FFCC01?logo=clickhouse&logoColor=black" />
  <img src="https://img.shields.io/badge/Firebird-E5402B?logo=firebird&logoColor=white" />
  <br/>
  <img src="https://img.shields.io/badge/MariaDB-003545?logo=mariadb&logoColor=white" />
  <img src="https://img.shields.io/badge/CockroachDB-6933FF?logo=cockroachlabs&logoColor=white" />
  <img src="https://img.shields.io/badge/Snowflake-29B5E8?logo=snowflake&logoColor=white" />
  <img src="https://img.shields.io/badge/DB2-052FAD?logo=ibm&logoColor=white" />
  <img src="https://img.shields.io/badge/H2-004080?logoColor=white" />
  <img src="https://img.shields.io/badge/Trino-DD00A1?logo=trino&logoColor=white" />
  <img src="https://img.shields.io/badge/Redshift-8C4FFF?logo=amazonredshift&logoColor=white" />
  <img src="https://img.shields.io/badge/Teradata-F37440?logo=teradata&logoColor=white" />
  <br/>
  <img src="https://img.shields.io/badge/TiDB-DC150B?logo=tidb&logoColor=white" />
  <img src="https://img.shields.io/badge/OceanBase-006AFF?logoColor=white" />
  <img src="https://img.shields.io/badge/OpenGauss-0052CC?logoColor=white" />
  <img src="https://img.shields.io/badge/Doris-0052CC?logoColor=white" />
  <img src="https://img.shields.io/badge/StarRocks-5C2D91?logoColor=white" />
  <img src="https://img.shields.io/badge/TimescaleDB-F2F2F2?logo=timescale&logoColor=black" />
  <img src="https://img.shields.io/badge/Hive-FDEE21?logo=apachehive&logoColor=black" />
  <img src="https://img.shields.io/badge/Databricks-FF3621?logo=databricks&logoColor=white" />
  <img src="https://img.shields.io/badge/BigQuery-669DF6?logo=googlebigquery&logoColor=white" />
  <img src="https://img.shields.io/badge/and%20more...-555555?logoColor=white" />
</p>

## Why SqlKit?

<table>
  <tr>
    <td width="50%">
      <h3>🦀 Tauri Native, Not Electron</h3>
      <p>Built with Rust + Tauri v2 — no Electron bloat, no JRE required. Ships as a small native binary with native performance. DBeaver needs Java; DataGrip needs the JetBrains runtime; TablePlus is macOS-only. SqlKit runs everywhere with nothing extra.</p>
    </td>
    <td width="50%">
      <h3>🤖 Agentic Database Client</h3>
      <p>Describe what you need in natural language — the agent writes queries, inspects schemas, optimizes slow SQL, explains execution plans, and returns results. It's not just a query editor with AI bolted on; it's an agent that understands your database and acts on your behalf. Supports OpenAI, Anthropic, DeepSeek, and Ollama. Bring your own key.</p>
    </td>
  </tr>
  <tr>
    <td>
      <h3>🗄️ 40+ Databases, One Tool</h3>
      <p>PostgreSQL, MySQL, Oracle, SQL Server, SQLite, DuckDB, ClickHouse, Firebird, MariaDB, CockroachDB, Redis, TiDB, OceanBase, Snowflake, DB2, H2, Trino, and more — with native, PG-wire-compatible, MySQL-wire-compatible, JDBC bridge, and HTTP adapters. All from a single interface.</p>
    </td>
    <td>
      <h3>🔒 Privacy-First & Secure</h3>
      <p>Encrypted credential storage backed by your OS keychain. SSH tunnel support for secure remote connections. No telemetry. Fully offline capable — your credentials and queries never leave your machine.</p>
    </td>
  </tr>
</table>

## Features

### Agentic SQL Assistant

Describe what you need in natural language — the agent writes queries, inspects schemas, updates records, optimizes SQL, and returns results. It's a database agent that operates through conversation.

- **Query generation** — natural language to SQL
- **Schema inspection** — agent reads and explains your database schema
- **Data operations** — CRUD, schema exploration, bulk actions through conversation
- **SQL optimization** — rewrite slow queries for better performance
- **Explain plans** — understand query execution visually
- **Error fixing** — agent diagnoses and fixes SQL errors
- **Safety** — destructive operations require confirmation; credentials never exposed to the LLM
- **Supported providers** — OpenAI, Anthropic, DeepSeek, Ollama

### Data Studio

A full-featured SQL editor powered by Monaco (VS Code engine) with syntax highlighting, autocomplete, and multi-tab support.

- **Monaco Editor** — VS Code-grade SQL editing with syntax highlighting and autocomplete
- **Multi-tab** — work on multiple queries simultaneously with tab management
- **Query history** — auto-saved, searchable, replayable
- **Results grid** — paginated, sortable, with inline editing
- **Export** — copy or export results as CSV, JSON, Markdown

### Multi-Database Support

SqlKit supports **40+ databases** across four adapter strategies:

| Strategy | Databases |
|----------|-----------|
| **Native** (Rust) | PostgreSQL, MySQL, SQL Server, SQLite, DuckDB, ClickHouse, Firebird, Oracle, RQLite, Turso |
| **PG-wire compat** (PostgreSQL adapter) | CockroachDB, Redshift, YugabyteDB, TimescaleDB, KingbaseES, GaussDB, HighGo, OpenGauss, GBase8c, QuestDB, Vastbase, YashanDB, and more |
| **MySQL-wire compat** (MySQL adapter) | MariaDB, TiDB, OceanBase, TDSQL, PolarDB, DM8, Doris, SelectDB, StarRocks, Databend, GoldenDB, ManticoreSearch, and more |
| **JDBC bridge** (Java) | DB2, H2, Snowflake, TDengine, Hive, Databricks, Hana, Teradata, Vertica, Exasol, BigQuery, Informix, Cassandra, and more |
| **HTTP bridge** | Trino, Presto |

### Schema Browser

Explore and manage your database structure with a visual tree browser.

- **Database tree** — browse databases, schemas, tables, views, columns, indexes
- **DDL viewer** — view CREATE statements for any object
- **Object search** — quickly find tables, views, and procedures across schemas
- **Table info** — column types, nullable, defaults, primary keys, foreign keys

### Data Operations

- **Import / Export** — transfer data between databases in CSV, JSON formats
- **Data transfer** — migrate data across different database engines
- **Bulk operations** — handle large datasets with batch processing

### Security & Connectivity

- **SSH Tunnel** — connect through SSH with key or password authentication
- **Encrypted storage** — credentials secured by OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **SSL/TLS** — encrypted connections to supported databases
- **Auto-reconnect** — resilient connection handling

### Cross-Platform

- **macOS** (Apple Silicon & Intel) — native `.dmg` installer
- **Windows** — `.exe` installer
- **Linux** — `.AppImage` and `.deb` packages

## Installation

<a href="https://www.geekfun.club/download">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?style=for-the-badge&logo=download&logoColor=white">
    <img src="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?style=for-the-badge&logo=download&logoColor=white" alt="Download">
  </picture>
</a>

**macOS** — `SqlKit_*_universal.dmg` (Apple Silicon & Intel) or `SqlKit_universal.app.tar.gz`

**Windows** — `SqlKit_*_x64-setup.exe` or `SqlKit_*_x64_en-US.msi`

**Linux** — `SqlKit_*_amd64.AppImage` | `SqlKit_*_amd64.deb` | `SqlKit-*_x86_64.rpm`

> All assets are signed and checksum-verified. [Download from the official site](https://www.geekfun.club/download) or [browse GitHub Releases](https://github.com/geek-fun/sqlkit/releases).

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust toolchain](https://www.rust-lang.org/tools/install)

**Linux (Ubuntu/Debian):**

```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev
```

**macOS:**

```bash
xcode-select --install
```

**Windows:** Install [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### Quick Start

```bash
npm install
npm run tauri dev
```

### Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start Vite dev server |
| `npm run build` | Build frontend for production |
| `npm run lint:check` | Run ESLint |
| `npm run lint:fix` | Auto-fix lint issues |
| `npm test` | Run frontend tests |
| `npm run tauri dev` | Start Tauri development |
| `npm run tauri build` | Build Tauri application |

### Build Instructions

See [BUILD.md](BUILD.md) for detailed platform-specific build requirements and troubleshooting.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | [Tauri v2](https://tauri.app/) (Rust) |
| Frontend | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| UI | [shadcn-vue](https://www.shadcn-vue.com/) + [UnoCSS](https://unocss.dev/) |
| Editor | [Monaco Editor](https://microsoft.github.io/monaco-editor/) |
| Database | [sqlx](https://github.com/launchbadge/sqlx) + engine-specific drivers |

## FAQ

<details>
<summary><strong>Is SqlKit free?</strong></summary>
Yes. SqlKit is open source under the Apache 2.0 license. All features are free.
</details>

<details>
<summary><strong>Does SqlKit phone home?</strong></summary>
No. SqlKit does not collect telemetry. The auto-update feature checks GitHub Releases for new versions — you can disable it in settings. Your credentials and queries stay on your machine.
</details>

<details>
<summary><strong>Can I use SqlKit without an internet connection?</strong></summary>
Yes. The desktop app works fully offline. AI features require network access to the model endpoint (or a local model via Ollama).
</details>

<details>
<summary><strong>How is SqlKit different from DBeaver / TablePlus / DataGrip?</strong></summary>
SqlKit is a native Tauri app (Rust) — no Java JRE, no Electron overhead. It includes AI natively (not as a plugin), supports 40+ databases, and runs on macOS, Windows, and Linux from a single binary. Privacy-first with encrypted local credential storage.
</details>

<details>
<summary><strong>What databases are supported?</strong></summary>
PostgreSQL, MySQL, Oracle, SQL Server, SQLite, DuckDB, ClickHouse, Firebird, MariaDB, CockroachDB, Redis, TiDB, OceanBase, Snowflake, DB2, H2, Trino, and 20+ more. See the <a href="#multi-database-support">Multi-Database Support</a> section for the full list.
</details>

<details>
<summary><strong>How do I report a bug or request a feature?</strong></summary>
Open an issue on <a href="https://github.com/geek-fun/sqlkit/issues">GitHub Issues</a>.
</details>

## Community

<p align="center">
  <img src="docs/images/wechat_group.jpg" width="140" alt="WeChat Group">
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <img src="docs/images/wechat_official.png" width="140" alt="WeChat Official Account">
</p>

<p align="center">
  <a href="https://discord.gg/5NSUyPK2E"><img src="https://img.shields.io/badge/Discord-Join-5865F2?logo=discord&logoColor=white&style=for-the-badge" alt="Discord" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://x.com/geekfun_club"><img src="https://img.shields.io/badge/X-Follow-000000?logo=x&logoColor=white&style=for-the-badge" alt="X / Twitter" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://www.youtube.com/@geekfun-club"><img src="https://img.shields.io/badge/YouTube-Subscribe-FF0000?logo=youtube&logoColor=white&style=for-the-badge" alt="YouTube" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://github.com/geek-fun"><img src="https://img.shields.io/badge/GitHub-Follow-181717?logo=github&logoColor=white&style=for-the-badge" alt="GitHub" /></a>
</p>

## Sponsor

<p align="center">
  <img src="docs/images/wechat_ponsor.jpg" width="140" alt="WeChat Sponsor QR">
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <a href="https://github.com/sponsors/geek-fun"><img src="https://img.shields.io/badge/GitHub_Sponsors-%E2%9D%A4_Support-EA4AAA?logo=githubsponsors&logoColor=white&style=for-the-badge" alt="GitHub Sponsors" /></a>
</p>

## Star History

<a href="https://www.star-history.com/?repos=geek-fun%2Fsqlkit&type=date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=geek-fun/sqlkit&type=date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=geek-fun/sqlkit&type=date" />
    <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=geek-fun/sqlkit&type=date" />
  </picture>
</a>

## License

[Apache 2.0](LICENSE) © GEEKFUN
