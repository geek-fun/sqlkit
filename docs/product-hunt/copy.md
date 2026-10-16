# SqlKit Product Hunt Copy (Paste-Ready)

> This copy covers implemented features only. Do not mention cross-engine data transfer anywhere (not built yet). Data Studio MCP is fair game: localhost:9121, `sqlkit__*` tools, read-only by default, writes and DDL behind explicit permission.

---

## 0. PH form quick-fill (field limits: tagline 60 / description 500)

| Field | Value |
|---|---|
| Name | `SqlKit` |
| Tagline (38/60) | `AI-native SQL client for 70+ databases` |
| Link | `https://www.geekfun.club/products/sqlkit/` |
| Is this an open source project? | **Yes** (Apache 2.0) |
| Pricing | Community: free forever (Apache 2.0) · Ultimate: $9.9/mo or $99/yr, 7-day trial (Ultimate requires an account) |
| X account | `geekfun_club` |
| Launch tags | `Database` + `Developer Tools` + `Artificial Intelligence`(表单下拉确认可用;全选 3 个) |
| Description | see §2 "PH form version" (fits the 500-char limit) |
| First comment | see §3 |

**PH form description (495/500, paste this into the form):**

> SqlKit is an agentic SQL client for 70+ databases: PostgreSQL, MySQL, Oracle, SQL Server, SQLite, DuckDB, ClickHouse, Snowflake, BigQuery. Describe what you need. The agent writes queries, optimizes slow SQL, explains execution plans, and fixes errors. BYOK: OpenAI, Anthropic, DeepSeek, Ollama; credentials never leave your machine. Data Studio MCP connects Claude Code and Cursor to your databases. Native Rust + Tauri. Open source (Apache 2.0).

> ⚠️ The old draft description says "50+" and omits Data Studio MCP — do not use it. 70+ is the launch口径 (README + app data). Also: marketing copy never mentions pricing — use "open source (Apache 2.0)" positioning. Never claim "all features free / no account". If asked directly, answer accurately via the §4 templates.

---

## 1. One-liner candidates

| # | One-liner | Notes |
|---|---|---|
| 1 (recommended) | `AI-native SQL client for 70+ databases` | Short, and the number lands |
| 2 | `Your database agent: native AI SQL client, 70+ DBs` | More agentic |
| 3 | `The native SQL client with a built-in AI agent and MCP` | Flags the MCP hook |

---

## 2. Description (PH body text)

> SqlKit is an agentic database client. It replaces DBeaver, Navicat, and DataGrip with one native app. Describe what you need in plain language, and the agent reads your schema, writes the query, optimizes slow SQL, visualizes the execution plan, and fixes errors on the spot. Bring your own key (OpenAI, Anthropic, DeepSeek, or a local model via Ollama). Credentials never leave your machine.
>
> • 70+ databases: PostgreSQL, MySQL, Oracle, SQL Server, SQLite, DuckDB, ClickHouse, Snowflake, BigQuery, and more
> • Native performance: Rust + Tauri v2, no Electron, no Java JRE
> • Data Studio MCP: open your databases to any coding agent (Claude Code, Cursor, Codex...). It runs a local MCP server on localhost:9121. Read access on by default; writes and DDL only after you approve
> • Privacy first: bring your own key, credentials in your OS keychain, no telemetry, works offline
> • A real editor: Monaco editor, schema browser, query history, CSV/JSON/Markdown export
> • Open source (Apache 2.0) — audit the code, fork it, build on it
>
> Built for engineers and data teams who are done wrestling with heavyweight clients.

**PH form version (this is the one to paste in the launch form):**

> SqlKit is an agentic SQL client for 70+ databases: PostgreSQL, MySQL, Oracle, SQL Server, SQLite, DuckDB, ClickHouse, Snowflake, BigQuery. Describe what you need. The agent writes queries, optimizes slow SQL, explains execution plans, and fixes errors. BYOK: OpenAI, Anthropic, DeepSeek, Ollama; credentials never leave your machine. Data Studio MCP connects Claude Code and Cursor to your databases. Native Rust + Tauri. Open source (Apache 2.0).

---

## 3. Maker's first comment

> Hey Product Hunt. We're the team behind GEEKFUN, the makers of SqlKit.
>
> SqlKit started with a simple annoyance: every database client we tried was heavy (DataGrip needs a JVM), slow (Electron), or paid (TablePlus), and none of them knew anything about our schema. So we built the client we wanted, and we've kept building it in public ever since.
>
> What makes it different:
> • An AI agent that reads your schema. Ask in plain language; it writes the SQL, optimizes slow queries, explains execution plans, and fixes errors on the spot. Bring your own key: OpenAI, Anthropic, DeepSeek, or Ollama for local models.
> • 70+ databases in one native app (Rust + Tauri, so there's no Java or Chromium to install). PostgreSQL, Oracle, BigQuery, and 60+ more.
> • Data Studio MCP. A local MCP server on localhost:9121 connects your coding agent (Claude Code, Cursor, Codex...) to your databases. Reads on by default; writes and DDL only after you approve.
> • Your credentials stay in the OS keychain and never touch the LLM. We don't collect telemetry.
> • Open source under Apache 2.0 — audit it, fork it, build on it.
>
> Try it on macOS, Windows, or Linux, and tell us what to improve next.
>
> Download: https://www.geekfun.club/download | GitHub: https://github.com/geek-fun/sqlkit

---

## 4. Comment reply templates

### Feature question
> Yes. [answer]. The agent works with any OpenAI-compatible endpoint, Anthropic's API, or Ollama for fully local models. If you hit an edge case with a specific engine, open an issue on GitHub; we ship fixes weekly.

### License / pricing question
> The Community edition is free forever and open source (Apache 2.0); the core client needs no account. Ultimate ($9.9/mo or $99/yr) unlocks the AI agent, MCP server, ER diagrams, and SSH tunnels, with a 7-day free trial (no credit card required).

### "How is this different from X?" (TablePlus / DataGrip / DBeaver / Chat2DB)
> It's native Rust + Tauri, so no JVM and no Electron. The AI agent is part of the client, not a plugin. It covers 70+ databases in one app. Data Studio MCP lets your coding agent query your databases directly. And it's privacy first (bring your own key, credentials never leave your keychain); the source is open (Apache 2.0).

### Praise / support
> Thanks, that means a lot. If anything feels off, or you want a feature we don't have, file it at github.com/geek-fun/sqlkit/issues. We read everything and ship fixes fast.

### Bug report
> Thanks for flagging it. Could you share your OS, database engine, and version? We'll try to reproduce it today. Appreciate you helping us harden it.

---

## 5. Launch tags recap

- PH launch tags (up to 3): `Database` + `Developer Tools` + `Artificial Intelligence` — select all three (confirmed available in the launch form dropdown)
- Why: Database = most targeted pool (DB-tool followers); Developer Tools = core dev audience; Artificial Intelligence = agentic AI + MCP differentiator, highest-traffic pool
- Category placement: Developer Tools (avoid Data Analysis / Visualization pools)

---

## 6. Shoutouts (PH form — each becomes a founder review on that product's page)

All four products verified to have active PH pages. Search these exact names in the form: `Tauri`, `Ollama`, `GitHub`, `shadcn/ui`.

**Tauri** (core framework — must):
> We built SqlKit on Tauri v2 (Rust backend, Vue frontend). No bundled Chromium, a small binary, and native performance on macOS, Windows, and Linux. The updater, store, and dialog plugins saved us weeks of plumbing. Best decision we made for our desktop stack.

**Ollama** (local models — our privacy story):
> SqlKit's agent supports Ollama for fully local models. Point it at your machine and your schema and queries never leave your network: no model API key, no cloud account. It's the least-friction way we found to ship AI features with a real privacy story.

**GitHub** (where the product lives):
> SqlKit lives on GitHub: source, issues, releases, and CI. The Releases + auto-updater flow means our users get fixes the day we ship them. Hard to imagine building an open-source product without it.

**shadcn/ui** (UI system — bonus):
> SqlKit's UI is built with shadcn-vue, the Vue port of shadcn/ui. Owning the component code instead of fighting a library is what let the app look like us, not like a framework. The accessibility work is a bonus we didn't have to do ourselves.

---

## 7. Social posts (X / 微信)

**X — announcement (post right after scheduling; attach hero-ui.png; pin it):**
> We're launching SqlKit on @ProductHunt — Sept 16.
>
> Open-source agentic SQL client for 70+ databases. Rust + Tauri, AI agent built in, and a local MCP server that connects Claude Code & Cursor to your databases.
>
> Get notified → [PH upcoming link]

**X — launch day (post the moment it goes live, 15:00 Beijing = 00:00 PT; attach agentic-ai.gif; pin it; ~120 chars incl. link):**
> SqlKit is live on Product Hunt 🚀
>
> AI-native SQL client for 70+ databases.
>
> We'd love your feedback ↓
> https://www.producthunt.com/products/sqlkit-2?utm_source=twitter&utm_medium=social

**X — thread follow-up (reply to your own tweet):**
> What's inside:
> • 70+ databases, one native app (no Electron, no JVM)
> • AI agent: natural language → SQL, optimizes slow queries, fixes errors
> • Data Studio MCP: Claude Code / Cursor connect to your databases (localhost:9121)
> • Credentials stay in your OS keychain, never sent to the LLM
> • Open source (Apache 2.0) — audit it, fork it

**中文版 (微信 / 社群):**
> SqlKit 将于 9 月 16 日登陆 Product Hunt。
> 开源 agentic SQL 客户端,支持 70+ 数据库;Rust + Tauri 原生;内置 AI agent 和本地 MCP server,Claude Code / Cursor 可直接查询你的数据库。
> 欢迎关注 → [PH 链接]

**Rules:** never write "please upvote" (violates PH anti-manipulation rules). Use "follow / get notified / check it out". Cadence: announcement → day-before reminder → launch-day post.
