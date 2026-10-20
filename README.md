<div align="center">

<img src="app-icon.png" width="120" alt="SqlKit Logo"/>

# SqlKit

**Agentic cross-platform SQL database GUI client — your database agent for 70+ databases.**

**Privacy-first. Your data, your keys. Open source.**

[![Release](https://img.shields.io/github/v/release/geek-fun/sqlkit?color=orange&label=release&logo=github)](https://github.com/geek-fun/sqlkit/releases)
[![Downloads](https://img.shields.io/github/downloads/geek-fun/sqlkit/total?color=orange&logo=docusign)](https://github.com/geek-fun/sqlkit/releases)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg?logo=apache)](LICENSE)
[![Stars](https://img.shields.io/github/stars/geek-fun/sqlkit?logo=github)](https://github.com/geek-fun/sqlkit/stargazers)
[![CI](https://github.com/geek-fun/sqlkit/actions/workflows/node.yml/badge.svg)](https://github.com/geek-fun/sqlkit/actions/workflows/node.yml)
[![MCP Toplist](https://mcptoplist.com/badge/io.github.geek-fun%2Fdata-studio.svg)](https://mcptoplist.com/server/io.github.geek-fun%2Fdata-studio)

<p>
  <img src="https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white"/>
  <img src="https://img.shields.io/badge/Windows-0078D6?logo=windows&logoColor=white"/>
  <img src="https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black"/>
  <img src="https://img.shields.io/badge/Tauri-FFC131?logo=tauri&logoColor=black"/>
  <img src="https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white"/>
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
  <br/>
  <img src="https://img.shields.io/badge/SQLCipher-003B57?logoColor=white" alt="SQLCipher" />
  <img src="https://img.shields.io/badge/YugabyteDB-FF6E42?logoColor=white" alt="YugabyteDB" />
  <img src="https://img.shields.io/badge/QuestDB-FF4B64?logoColor=white" alt="QuestDB" />
  <img src="https://img.shields.io/badge/Greenplum-43B02A?logoColor=white" alt="Greenplum" />
  <img src="https://img.shields.io/badge/EnterpriseDB-003B71?logoColor=white" alt="EnterpriseDB" />
  <img src="https://img.shields.io/badge/CrateDB-009DC7?logoColor=white" alt="CrateDB" />
  <img src="https://img.shields.io/badge/Materialize-7B61FF?logoColor=white" alt="Materialize" />
  <img src="https://img.shields.io/badge/AlloyDB-4285F4?logo=googlecloud&logoColor=white" alt="AlloyDB" />
  <br/>
  <img src="https://img.shields.io/badge/SelectDB-22C1C3?logoColor=white" alt="SelectDB" />
  <img src="https://img.shields.io/badge/Databend-5B47F5?logoColor=white" alt="Databend" />
  <img src="https://img.shields.io/badge/SingleStore-AA00FF?logo=singlestore&logoColor=white" alt="SingleStore" />
  <img src="https://img.shields.io/badge/Manticore%20Search-E94B35?logoColor=white" alt="Manticore Search" />
  <img src="https://img.shields.io/badge/TDSQL-0052D9?logoColor=white" alt="TDSQL" />
  <img src="https://img.shields.io/badge/PolarDB-FF6A00?logoColor=white" alt="PolarDB" />
  <img src="https://img.shields.io/badge/GoldenDB-C69214?logoColor=white" alt="GoldenDB" />
  <img src="https://img.shields.io/badge/TDengine-2F6FFF?logoColor=white" alt="TDengine" />
  <br/>
  <img src="https://img.shields.io/badge/Dameng-3857FF?logoColor=white" alt="Dameng" />
  <img src="https://img.shields.io/badge/GaussDB-E60012?logoColor=white" alt="GaussDB" />
  <img src="https://img.shields.io/badge/KingbaseES-003B8E?logoColor=white" alt="KingbaseES" />
  <img src="https://img.shields.io/badge/HighGo-005BAC?logoColor=white" alt="HighGo" />
  <img src="https://img.shields.io/badge/UXDB-0052CC?logoColor=white" alt="UXDB" />
  <img src="https://img.shields.io/badge/Vastbase-0052CC?logoColor=white" alt="Vastbase" />
  <img src="https://img.shields.io/badge/YashanDB-0052CC?logoColor=white" alt="YashanDB" />
  <img src="https://img.shields.io/badge/XuguDB-0052CC?logoColor=white" alt="XuguDB" />
  <br/>
  <img src="https://img.shields.io/badge/GBase%208a-0052CC?logoColor=white" alt="GBase 8a" />
  <img src="https://img.shields.io/badge/GBase%208c-0052CC?logoColor=white" alt="GBase 8c" />
  <img src="https://img.shields.io/badge/SAP%20HANA-0FAAFF?logo=sap&logoColor=white" alt="SAP HANA" />
  <img src="https://img.shields.io/badge/Vertica-005A9C?logoColor=white" alt="Vertica" />
  <img src="https://img.shields.io/badge/Exasol-005F71?logoColor=white" alt="Exasol" />
  <img src="https://img.shields.io/badge/Informix-052FAD?logoColor=white" alt="Informix" />
  <img src="https://img.shields.io/badge/InterSystems%20IRIS-00A5A8?logoColor=white" alt="InterSystems IRIS" />
  <img src="https://img.shields.io/badge/Access-A4373A?logoColor=white" alt="Access" />
  <br/>
  <img src="https://img.shields.io/badge/Apache%20Derby-D22128?logoColor=white" alt="Apache Derby" />
  <img src="https://img.shields.io/badge/Apache%20Kylin-F09D13?logoColor=white" alt="Apache Kylin" />
  <img src="https://img.shields.io/badge/Cassandra-1287B1?logo=apachecassandra&logoColor=white" alt="Cassandra" />
  <img src="https://img.shields.io/badge/Presto-5890FF?logo=presto&logoColor=white" alt="Presto" />
  <img src="https://img.shields.io/badge/rqlite-007D9C?logoColor=white" alt="rqlite" />
  <img src="https://img.shields.io/badge/Turso-4FF8D2?logo=turso&logoColor=black" alt="Turso" />
  <img src="https://img.shields.io/badge/Cloud%20SQL%20for%20PostgreSQL-4285F4?logo=googlecloud&logoColor=white" alt="Cloud SQL for PostgreSQL" />
  <img src="https://img.shields.io/badge/Cloud%20SQL%20for%20MySQL-4285F4?logo=googlecloud&logoColor=white" alt="Cloud SQL for MySQL" />
  <img src="https://img.shields.io/badge/Fujitsu%20Enterprise%20Postgres-E60012?logoColor=white" alt="Fujitsu Enterprise Postgres" />
  <img src="https://img.shields.io/badge/and%20more...-555555?logoColor=white" />
</p>

## Installation

<a href="https://www.geekfun.club/download">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?logo=download&logoColor=white">
    <img src="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?logo=download&logoColor=white" alt="Download">
  </picture>
</a>
&nbsp;
<a href="https://github.com/geek-fun/sqlkit/releases">
  <img src="https://img.shields.io/badge/Releases-GitHub-lightgrey?logo=github" alt="Releases">
</a>
&nbsp;
<a href="https://www.geekfun.club/products/sqlkit/">
  <img src="https://img.shields.io/badge/Website-geekfun.club-blue?logo=google-chrome&logoColor=white" alt="Website">
</a>

## Features

### Agentic Data Studio

Describe what you need in natural language — the agent reads your schema, writes queries, optimizes slow SQL, explains execution plans visually, and fixes errors on the spot. Supports OpenAI, Anthropic, DeepSeek, and Ollama. Bring your own key.

- **AI query generation** — natural language to SQL with schema-aware context
- **SQL optimization** — rewrite slow queries and visualize execution plans
- **Error fixing** — agent diagnoses and fixes SQL errors automatically
- **Safety** — destructive operations require confirmation; credentials never exposed to the LLM

### Lightweight & Native Performance

Built with Rust + Tauri v2 — no Electron, no JRE, no bundled Chromium. Ships as a small native binary with native performance across macOS, Windows, and Linux. What other apps need a Java runtime or a JetBrains license for, SqlKit does in a single download.

### All Your Databases, One App

SqlKit supports **70+ databases** across four adapter strategies:

| Strategy | Databases |
|----------|-----------|
| **Native** (Rust) | PostgreSQL, MySQL, SQL Server, SQLite |
| **PG-wire compat** | CockroachDB, Redshift, YugabyteDB, TimescaleDB, QuestDB, Vastbase, YashanDB, KingbaseES, GaussDB, HighGo, UXDB, OpenGauss, GBase8c, Greenplum, EnterpriseDB, CrateDB, Materialize, AlloyDB, CloudSQLPG, FujitsuPG |
| **MySQL-wire compat** | MariaDB, TiDB, OceanBase, TDSQL, PolarDB, Doris, SelectDB, StarRocks, Databend, GoldenDB, ManticoreSearch, SingleStore, CloudSQLMySQL |
| **JDBC bridge** | Oracle, DuckDB, Firebird, DB2, H2, Snowflake, TDengine, Derby, Hive, Databricks, Hana, Teradata, Vertica, Exasol, BigQuery, Informix, Kylin, Cassandra, Iris, Access, Dameng, XuguDB, GBase8a |
| **HTTP bridge** | ClickHouse, Trino, Presto, RQLite, Turso |

### Product-Grade Editor

Powered by Monaco (VS Code engine) with full SQL syntax highlighting, autocomplete, and multi-tab support. Browse schemas visually, inspect DDL, and search objects across databases.

- **Monaco Editor** — VS Code-grade SQL editing with syntax highlighting and autocomplete
- **Multi-tab** — work on multiple queries simultaneously with tab management
- **Schema Browser** — tree view of databases, schemas, tables, columns, indexes
- **Query history** — auto-saved, searchable, replayable
- **Results grid** — paginated, sortable, with inline editing and CSV/JSON/Markdown export
- **DDL viewer** — view CREATE statements for any object
- **Object search** — quickly find tables, views, and procedures across schemas

### Large Data Transfer

Move data between any supported engines — PostgreSQL to ClickHouse, Oracle to SQL Server, or MySQL to BigQuery. No intermediate files required.

- **Cross-engine transfer** — migrate data between different database types with automatic type mapping
- **Import / Export** — CSV, JSON, JSONL formats
- **Bulk operations** — handle millions of records with batch processing

### Security & Connectivity

- **SSH Tunnel** — connect through SSH with key or password authentication
- **Encrypted storage** — credentials secured by OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **SSL/TLS** — encrypted connections to supported databases
- **Auto-reconnect** — resilient connection handling

### Cross-Platform

- **macOS** (Apple Silicon & Intel) — native `.dmg` installer
- **Windows** — `.exe` installer
- **Linux** — `.AppImage` and `.deb` packages

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

Yes — the **Community edition** is free and open source under the Apache 2.0 license. Connections, SQL editing, schema browsing, data editing, query history and result export are all free, with no artificial limits.

Advanced capabilities — AI agent (bring your own LLM), MCP server, ER diagrams, bulk import/export and cross-engine data transfer — are part of the paid **GEEKFUN Data Studio Ultimate** subscription ($9.9/month or $99/year, with a 7-day free trial). Every version released while you're subscribed stays yours forever, even if you cancel later.

[See pricing →](https://www.geekfun.club/pricing)
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
SqlKit is a native Tauri app (Rust) — no Java JRE, no Electron overhead. It includes AI natively (not as a plugin), supports 70+ databases, and runs on macOS, Windows, and Linux from a single binary. Privacy-first with encrypted local credential storage.
</details>

<details>
<summary><strong>What databases are supported?</strong></summary>
PostgreSQL, MySQL, Oracle, SQL Server, SQLite, DuckDB, ClickHouse, Firebird, MariaDB, CockroachDB, TiDB, OceanBase, Snowflake, DB2, H2, Trino, Greenplum, CrateDB, SingleStore, and 60+ more. See the <a href="#multi-database-support">Multi-Database Support</a> section for the full list.
</details>

<details>
<summary><strong>How do I report a bug or request a feature?</strong></summary>
Open an issue on <a href="https://github.com/geek-fun/sqlkit/issues">GitHub Issues</a>.
</details>

## Community

<p align="center">
  <img src="docs/images/wechat_official.png" width="300" alt="WeChat Official Account">
</p>

<p align="center">
  <a href="https://x.com/geekfun_club"><img src="https://img.shields.io/badge/X-Follow-000000?logo=x&logoColor=white" alt="X / Twitter" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://www.youtube.com/@geekfun-club"><img src="https://img.shields.io/badge/YouTube-Subscribe-FF0000?logo=youtube&logoColor=white" alt="YouTube" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://github.com/geek-fun"><img src="https://img.shields.io/badge/GitHub-Follow-181717?logo=github&logoColor=white" alt="GitHub" /></a>
</p>

## Sponsor

<p align="center">
  <img src="docs/images/wechat_ponsor.jpg" width="140" alt="WeChat Sponsor QR">
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <a href="https://github.com/sponsors/geek-fun"><img src="https://img.shields.io/badge/GitHub_Sponsors-%E2%9D%A4_Support-EA4AAA?logo=githubsponsors&logoColor=white" alt="GitHub Sponsors" /></a>
</p>

## License

[Apache 2.0](LICENSE) © GEEKFUN
