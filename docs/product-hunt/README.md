# SqlKit Product Hunt Launch Guide

本目录包含 SqlKit 在 Product Hunt 发布的所有物料与策略。可直接粘贴的文案见 [copy.md](./copy.md)。

> **发布纪律**:所有文案只提已经实现的功能。以下功能未完成,发布时不得提及:
> - ~~Cross-engine data transfer(跨引擎数据迁移)~~ — 未实现
> - 其余文案中出现的功能均已在代码库中确认:AI agent、70+ 数据库、Tauri/Rust 原生、Data Studio MCP(port 9121)、Monaco 编辑器、SSH 隧道、OS keychain 凭据存储。
> - ⚠️ **收费口径(必守)**:营销物料(图 / 社媒帖 / PH 文案)**不主动提及收费**,统一用"开源开放(Apache 2.0)"表述;不得写"全功能免费 / 不需要账号"。被直接问询时,按 copy.md §4 回复模板如实回答(Community 免费开源;AI / MCP 等属 Ultimate 订阅,7 天试用)。

---

## 一、定位策略

SqlKit 在 PH 上的竞品是:Chat2DB(Web/AI 分析)、BlazeSQL(ChatGPT for SQL)、Basedash(AI BI)、TablePlus(付费桌面客户端)。差异化四张牌:

| 卖点 | 一句话 |
|---|---|
| **Agentic AI(内置)** | 描述需求 → agent 读 schema、写 SQL、优化慢查询、可视化执行计划、自动修复报错 |
| **原生 vs 臃肿** | Rust + Tauri v2,无 Electron、无 JRE;单目录安装,原生性能 |
| **70+ 数据库** | PostgreSQL → Oracle → BigQuery,一个客户端全搞定 |
| **Data Studio MCP** | 内置本地 MCP server(localhost:9121),把数据库能力暴露给**任何 coding agent**(Claude Code、Cursor、Codex…)。默认只读;写入/DDL 需显式授权(Full Access + Confirm Destructive) |
| **隐私优先 + 开源(商业化已上线)** | BYOK、凭据只进 OS keychain、零遥测、可离线;Community 版免费开源(Apache 2.0)+ Ultimate 订阅($9.9/mo 或 $99/yr,7 天试用,需账号) |

> **定位雷区**:
> - 不要把 SqlKit 定位成 "AI SQL copilot / ChatGPT for SQL"。那会掉进纯 chat 类竞品的池子里,70+ 库和原生客户端这两个卖点就白费了。
> - 不要叫它 "chatbot"。定位是:完整数据库客户端,AI 和 MCP 是原生能力。

---

## 二、Categories and topics

- **Category(必选)**:`Developer Tools`
- **Topics 标签**:`GitHub`、`database`、`sql`、`open-source`、`ai`
- 不要选 Data Analysis / Data Visualization —— 会把产品推进与 Chat2DB、BlazeSQL 直接对打的池子;竞品比较页面(Alternatives)会显示它们,但我们的差异化是"开发者桌面客户端 + AI + MCP"。

---

## 三、Gallery 图片计划(7 张资产矩阵)

PH 相册建议 5-8 张。统一规范:**2560×1440(16:9)**、深色底(**#0B0F16 → #10151E**)、橙色信号色 **#F97316 / #FB923C**(与已生成图一致)、hairline 1px 边框、18px 圆角、大写微标签(tracking 0.2em)、系统字体栈;边缘留白 ≥110px(PH 5:3 裁剪安全);不用 emoji。

| # | 图片 | 主题文案 | 素材来源 | 状态 |
|---|---|---|---|---|
| 0 | `databases-promo.png` | SUPPORT 70+ DATABASES / ALL SQL IN ONE CLIENT | 63 个 db logo(已内联) | ✅ 已生成 |
| 1 | `hero-ui.png` | Every database. One native app.(连接管理器 Hero) | geekfun-site `sqlkit-client-ui.png`(3424×2224) | ✅ 已生成 |
| 2 | `agentic-ai.png` | Describe it. SqlKit writes it. (NL → SQL → 结果 → 执行计划) | 运行 app 的 Data Studio 对话截图 `sqlkit-data-studio-ui.png`(1440×935)参考 | ✅ 已生成 |
| 3 | `data-studio-mcp.png` | Your coding agent, connected to your databases(agent → Data Studio MCP → SqlKit SQL :9121 / DocKit NoSQL :9120 → 3 排 × 8 个 DB 图标) | geekfun-site `agent-logos/` + dockit `app-icon.png` + SQL/NoSQL db-logos | ✅ 已生成 |
| 4 | `editor-schema.png` | Write SQL in a real editor.(Monaco + schema 树) | geekfun-site `sqlkit-sql-editor-ui.png`(3424×2224) | ✅ 已生成 |
| 5 | `data-grid.png` | Browse, edit, export.(虚拟滚动网格) | geekfun-site `sqlkit-data-view-table-ui.png`(3424×2224) | ✅ 已生成 |
| 6 | `er-diagram.png` | See how your data connects.(ER 图) | geekfun-site `sqlkit-er-diagram-ui.png`(3424×2224) | ✅ 已生成 |
| 7 | `agentic-ai.gif` | 动画演示:自然语言 → SQL → 结果(动图) | 合成(6 帧 + ffmpeg palette) | ✅ 已生成 |

> **中文社媒版(3 张,用于小红书/微信/知乎)**:`zh-overview.png`(70+ 数据库封面)、`zh-ai.png`(用 AI Agent 和你的数据库对话)、`zh-mcp.png`(编码 Agent 直连数据库)—— 详见 `copy-zh.md` §6。无收费信息,统一"开源开放"口径。

**生产 pipeline(统一)**:
1. 一次性 HTML 模板(`docs/product-hunt/src/` + 共享 CSS tokens),所有图复用同一套主题变量 → 保证整套图风格一致
2. 4K 截图直接作为主体,不做 mock;背景光晕/hairline/标签由模板叠加
3. Playwright 截图 2560×1440(scale: device),每张图程序化检查:边距 ≥110px、无文字裁切、控制台 0 报错

**注意(GEEKFUN 设计规范的两条红线)**:
- **One Lamp Rule**:amber 只做单信号,≤10% 画面;不要大面积橙色
- **不做 hero-metric 排版**:大数字+小标签(现有 `databases-promo.png` 的 "70+" 属于此类,为 PH 营销图可保留,但若要求严格统一可改为"文字型 headline")

> ⛔ **红线提醒**:任何时候不得在图中出现跨引擎数据迁移(Cross-engine data transfer)相关内容——该功能未实现。所有图只呈现代码库中已确认的功能:AI agent、70+ 数据库、原生 Tauri、Data Studio MCP(端口 9121)、Monaco 编辑器、SSH 隧道、凭据 keychain 存储。



---

## 四、发布时机与策略

1. **最佳时机**:周二/周三,美西 00:01 AM(北京时间下午 3 点左右)上线,当天有完整 24h 曝光池;避开周五/周末。
2. **预热**:先用 GitHub 账号建立 product page,提前 1–2 周攒 followers(目标 100+);上线当天首波助力来自这批老粉丝。
3. **上线流程**:
   - 00:01 PT 发布 → 立即贴 Maker First Comment(copy.md 已备好)→ 通知社区
   - **前 4 小时是决定排名窗口**:每条评论 15 分钟内回复
   - 回复带信息量、可顺带放下载链接 `https://www.geekfun.club/download`
4. **评论纪律**:只邀请**真实用过**的用户 upvote。PH 反作弊严格,买量/机器人/互投群会被降权甚至移除产品。
5. **辅助渠道**(发布当日):X(@geekfun_club)、LinkedIn、Reddit(r/database、r/SQL)、Hacker News "Show HN"、Discord、微信/社区群、YouTube 演示视频。

---

## 五、发布前 Checklist

- [ ] Product page 建立、GitHub 账号关联、团队头像设置
- [ ] One-liner 选定(见 copy.md 候选,推荐候选 1)
- [ ] Description 终稿(copy.md 已按"无跨引擎迁移 + 含 Data Studio MCP"更新)
- [ ] 5 张 Gallery 图 + 1 张 GIF(AI 对话演示)
- [ ] Maker First Comment 终稿
- [ ] 5 个评论回复模板(见 copy.md)
- [ ] 核对以下链接可用:`geekfun.club/download`、GitHub Releases(tag 已打)、`docs/DISTRIBUTION.md`、GitHub Star 数
- [ ] 1 分钟演示视频(可选,PH 支持 video 位)
- [ ] 发布当天排班:谁盯评论区(前 4h 密集)、谁通知社区、谁做 X/Reddit 帖

---

## 六、文件清单

| 文件 | 内容 |
|---|---|
| [copy.md](./copy.md) | One-liner、Description、Maker 首评、回复模板 —— 可直接粘贴 |
| [copy-zh.md](./copy-zh.md) | 国内社媒中文文案(微信/微博/知乎/小红书)+ 图片精简建议 |
| [README.md](./README.md) | 本文件:策略、定位、图片、时间线、checklist |
