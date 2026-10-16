# SqlKit 国内社媒文案(中文版)

> 用法:复制对应段落直接发布。主链接统一用 `https://www.geekfun.club/download`,GitHub `https://github.com/geek-fun/sqlkit`。Product Hunt 链接只在长文里作为"海外同发"提一句。
> 图片只发 1-2 张,不要把 8 张全套带上(见文末图片建议)。

---

## 1. 微信朋友圈 / 微信群(短)

> SqlKit 正式发布了 🎉
>
> 开源 AI 原生 SQL 客户端,支持 70+ 数据库:PostgreSQL、MySQL、Oracle、SQL Server、SQLite、DuckDB、ClickHouse、Snowflake、BigQuery……
>
> · Rust + Tauri 原生性能,无 Electron、无 JVM
> · 内置 AI agent:和你的数据库对话,写 SQL、优化慢查询、解释执行计划
> · 本地 MCP server:Claude Code / Cursor 可以直接查询你的数据库
> · 开源开放(Apache 2.0),代码全在 GitHub
> · 凭据只存本地钥匙串,零遥测
>
> 下载:https://www.geekfun.club/download
> GitHub:https://github.com/geek-fun/sqlkit

---

## 2. 微博(≤140 字)

> SqlKit 发布:开源 AI 原生 SQL 客户端,支持 70+ 数据库。Rust 原生,AI Agent 和你的数据库对话,写 SQL、优化慢查询;MCP 直连 Claude Code / Cursor。开源开放,数据不出本机。geekfun.club/download

---

## 3. 知乎 / 掘金 / V2EX(长文)

**标题:开源 SqlKit:支持 70+ 数据库的 AI 原生 SQL 客户端**

> 数据库客户端这个品类已经很卷了,我们还是做了一个,因为有几个痛点一直没人解决干净:
>
> · 老牌客户端太重:DBeaver 依赖 JVM,DataGrip 要订阅,Navicat 价格不低
> · AI 都是"外挂":多数工具把聊天框贴在界面旁边,它并不真正理解你的 schema
> · 数据出不去:很多企业场景不允许把库结构和数据发给云端 AI
>
> 所以我们从零写了 SqlKit:Rust + Tauri 原生,一个安装包,70+ 数据库一套搞定;AI agent 直接读 schema、写查询、优化慢 SQL、可视化执行计划;凭据只存在系统钥匙串,永远不发给 LLM(支持 OpenAI / Anthropic / DeepSeek / Ollama,自带 key,或用本地模型)。
>
> 还有一个我们最喜欢的特性:内置 Data Studio MCP。本地 MCP server(localhost:9121)把数据库暴露给你的 coding agent,Claude Code、Cursor、Codex 都能直接查你的库;默认只读,写操作和 DDL 需要手动授权。
>
> 开源开放(Apache 2.0),代码全部公开:
> https://github.com/geek-fun/sqlkit
> 下载:https://www.geekfun.club/download
>
> (海外同发在 Product Hunt,欢迎去支持:https://www.producthunt.com/products/sqlkit-2)

---

## 4. 小红书

**标题:开源了个 70+ 数据库的客户端**

> 做数据库的应该都懂这种感觉:
>
> MySQL 一个客户端、Redis 一个、ClickHouse 又一个,光切工具就够烦;
> 打开还得先等它把 JVM 或者 Chromium 背起来;
> 想让 AI 帮忙写 SQL,得手动复制表结构,顺便担心连接信息泄露。
>
> 所以我们做了 SqlKit:一个开源的数据库工作台,Rust + Tauri 写的,不套 Electron 壳,一个客户端连 70+ 数据库。
>
> 🗄️ 70+ 数据库,一个客户端
> PostgreSQL、MySQL、Oracle、SQL Server、SQLite、DuckDB、ClickHouse、Snowflake、BigQuery……本地到云端,不用再切来切去。
>
> 🤖 用 AI Agent,和你的数据库对话
> 不用复制表结构。Agent 直接读你的库:生成查询、优化慢 SQL、解释执行计划、报错一键修复。模型用你自己的 key(OpenAI / Anthropic / DeepSeek),也能接本地 Ollama。
>
> 🔌 你的编码 Agent,直连你的数据库
> 内置 Data Studio MCP:Claude Code、Cursor、Codex 都能直接查你的库。默认只读,写操作要你确认,凭据不离开本机。
>
> ⚡ 用起来是真的轻
> 原生 Rust,没有 Electron、没有 JVM。Monaco 编辑器、大结果集虚拟滚动、查询历史、CSV / JSON 导出,日常要的都有。
>
> 🛡️ 数据不出本机
> 凭据存在系统钥匙串,零遥测;SSH 隧道、SSL 加密连接都支持。
>
> macOS / Windows / Linux 都能装,开源开放(Apache 2.0),代码在 GitHub:github.com/geek-fun/sqlkit
> 下载在 geekfun.club/download
>
> 受够了一个数据库一个工具的朋友,欢迎试试～也欢迎来提需求、报 bug。
>
> #数据库 #SQL #程序员 #开源 #开发者工具 #AI工具 #效率工具 #MCP

---

## 5. 评论回复模板(中文)

- **收费吗?** → 核心客户端是 Community 版:永久免费、开源(Apache 2.0),不需要注册。AI agent、MCP、ER 图、SSH 隧道等进阶功能在 Ultimate 订阅版:9.9 美元/月或 99 美元/年,支持 7 天免费试用(无需信用卡)。
- **支持 XX 数据库吗?** → 支持,一共 70+ 种:原生驱动 + 兼容协议 + JDBC 桥三种方式,完整列表在 GitHub README。
- **和 Navicat / DBeaver 什么区别?** → 三点:原生 Rust,没有 JVM/Electron,更轻更快;AI 是内置的不是插件;开源免费。另外我们支持 70+ 库,还有本地 MCP。
- **AI 要额外付费吗?** → AI 功能包含在 Ultimate 订阅内(7 天免费试用);模型用你自己的 key(OpenAI / Anthropic / DeepSeek)或本地 Ollama,不另收模型费用。
- **怎么更新?** → 应用内自动检查更新,也可以直接去官网/GitHub Releases 下载新版本覆盖安装。

---

## 6. 图片建议(3 张中文图,已生成)

**发布组合(小红书 / 微信 / 知乎 通用):**
1. `zh-overview.png` — 封面:70+ 数据库,一个客户端搞定
2. `zh-ai.png` — 用 AI Agent,和你的数据库对话
3. `zh-mcp.png` — 编码 Agent 直连数据库(Claude Code / Cursor)

**动态补充(可选):**群里发 `agentic-ai.gif`(英文界面动图;朋友圈不支持动图)。

**不发:**`data-grid.png`、`er-diagram.png`、`editor-schema.png` 等英文文字图 —— 3 张中文图足够,一次太多反而没人看。

> 英文版全套保留(databases-promo / hero-ui / agentic-ai 等),用于海外渠道(PH / X)。
> 所有图均不含收费信息,统一"开源开放"口径。
