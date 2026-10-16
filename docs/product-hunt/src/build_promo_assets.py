#!/usr/bin/env python3
"""Build the SqlKit Product Hunt marketing HTMLs into docs/product-hunt/src/.

Emits: data-studio-mcp.html (reworked diagram), hero-ui.html, editor-schema.html,
data-grid.html, er-diagram.html. Raster assets are inlined as data URIs; the shared
promo-base.css provides tokens/components. Screenshots are downscaled only (never upscaled).

Run:  python3 docs/product-hunt/src/build_promo_assets.py
"""
import base64
import os
import re

ROOT = "/Users/blankll/Documents/devs/geekfun/sqlkit"
OUT = os.path.join(ROOT, "docs/product-hunt/src")

APP_ICON = os.path.join(ROOT, "app-icon.png")
DOCKIT_ICON = "/Users/blankll/Documents/devs/geekfun/dockit/app-icon.png"
SITE_PUBLIC = "/Users/blankll/Documents/devs/geekfun/geekfun-site/docs/public"
AGENT_LOGOS = os.path.join(SITE_PUBLIC, "agent-logos")
SQL_DB_ICONS = os.path.join(ROOT, "src/assets/images/database-icons")
SQLKIT_UI = os.path.join(SITE_PUBLIC, "sqlkit-data-studio-ui.png")


def b64(path):
    with open(path, "rb") as f:
        return base64.b64encode(f.read()).decode()


APP_B64 = b64(APP_ICON)
DOCKIT_B64 = b64(DOCKIT_ICON)
SQLKIT_UI_B64 = b64(SQLKIT_UI)

AGENTS = [
    ("Claude Code", "claude-code-logo.png"),
    ("Cursor", "cursor-logo.png"),
    ("Codex", "codex-logo.png"),
    ("Windsurf", "windsurf-logo.png"),
    ("Trae", "trae-logo.png"),
    ("Cline", "cline-logo.png"),
    ("OpenCode", "opencode-logo.png"),
    ("GitHub Copilot", "github-copilot-logo.png"),
]
AGENT_B64 = {name: b64(os.path.join(AGENT_LOGOS, fn)) for name, fn in AGENTS}


def clean_svg(raw, ns, recolor=None):
    s = re.sub(r"<\?xml[^?]*\?>\s*", "", raw)
    s = re.sub(r"<!--.*?-->\s*", "", s, flags=re.S)
    if recolor:
        for old, new in recolor.items():
            s = s.replace(old, new)
    s = re.sub(r'id="([^"]+)"', lambda m: f'id="{ns}-{m.group(1)}"', s)
    s = re.sub(r"url\(#([^)]+)\)", lambda m: f"url(#{ns}-{m.group(1)})", s)
    s = re.sub(r'\sclass="icon"', "", s)
    s = re.sub(r'\sclass="elastic-logo"', "", s)
    return s.strip()


SQL_ROW1 = [
    ("PostgreSQL", "postgresql-logo.svg"),
    ("MySQL", "mysql-logo.svg"),
    ("SQL Server", "sqlserver-logo.svg"),
    ("SQLite", "sqlite-logo.svg"),
    ("Oracle", "oracle-logo.svg"),
    ("ClickHouse", "clickhouse-logo.svg"),
    ("Snowflake", "snowflake-logo.svg"),
    ("BigQuery", "bigquery-logo.svg"),
]
SQL_ROW2 = [
    ("DuckDB", "duckdb-logo.svg"),
    ("MariaDB", "mariadb-logo.svg"),
    ("CockroachDB", "cockroachdb-logo.svg"),
    ("TiDB", "tidb-logo.svg"),
    ("Amazon Redshift", "redshift-logo.svg"),
    ("Trino", "trino-logo.svg"),
    ("Databricks SQL", "databricks-logo.svg"),
    ("Teradata", "teradata-logo.svg"),
]
# Last row is 8 tiles to match the two SQL rows above. kind: "svg"/"png" read from
# SITE_PUBLIC, "sqlsvg" reads from SqlKit's database-icons.
LAST_ROW = [
    ("MongoDB", "db-mongodb.svg", "svg"),
    ("Elasticsearch", "db-elasticsearch.svg", "svg"),
    ("OpenSearch", "db-opensearch.png", "png"),
    ("DynamoDB", "db-dynamodb.svg", "svg"),
    ("EasySearch", "db-easysearch.svg", "svg"),
    ("Apache Cassandra", "cassandra-logo.svg", "sqlsvg"),
    ("Apache Hive", "hive-logo.svg", "sqlsvg"),
    ("SAP HANA", "hana-logo.svg", "sqlsvg"),
]

_inline = {}


def icon_html(name, fn, kind, ns):
    if (name, fn) in _inline:
        return _inline[(name, fn)]
    if kind == "png":
        html = ('<img src="data:image/png;base64,%s" alt="%s">'
                % (b64(os.path.join(SITE_PUBLIC, fn)), name))
    elif kind == "sqlsvg":
        html = clean_svg(open(os.path.join(SQL_DB_ICONS, fn)).read(), ns)
    elif name == "EasySearch":
        # source icon is a single near-black fill (#231815); lighten so it reads on the dark tile
        svg = clean_svg(open(os.path.join(SITE_PUBLIC, fn)).read(), ns,
                        recolor={"#231815": "#E5E7EB"})
        html = svg
    else:
        html = clean_svg(open(os.path.join(SITE_PUBLIC, fn)).read(), ns)
    _inline[(name, fn)] = html
    return html


def sql_icon(name, fn, ns):
    if (name, fn) in _inline:
        return _inline[(name, fn)]
    html = clean_svg(open(os.path.join(SQL_DB_ICONS, fn)).read(), ns)
    _inline[(name, fn)] = html
    return html


def tile(logo_html, name, cls):
    return ('<div class="%s"><div class="tile-logo">%s</div>'
            '<div class="tile-name small">%s</div></div>' % (cls, logo_html, name))


def build_agentic():
    sub = ("Natural language to SQL. It reads your schema, optimizes slow queries, "
           "explains execution plans, and fixes errors. Bring your own key: "
           "OpenAI, Anthropic, DeepSeek, or Ollama.")
    html = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<link rel="icon" href="data:,">
<link rel="stylesheet" href="promo-base.css">
<title>SqlKit — Agentic Data Studio</title>
<style>
  html, body { width: 2560px; height: 1440px; }
  .stage { width: 2560px; height: 1440px; }

  .left {
    position: absolute; left: 0; top: 0; width: 1110px; height: 100%;
    padding: 0 40px 0 128px;
    display: flex; flex-direction: column; justify-content: center;
  }
  .brand { margin-bottom: 66px; }
  .brand img { width: 64px; height: 64px; }
  .eyebrow { margin-bottom: 30px; }
  .headline { font-size: 88px; }
  .headline .l1 { display: block; }
  .headline .l2 { display: block; margin-top: 4px; }
  .headline .l2 .accent {
    background: linear-gradient(180deg, #ffe0bd 0%, #ffb469 30%, #fb923c 62%, #f97316 90%);
    -webkit-background-clip: text; background-clip: text; color: transparent;
  }
  .sub { margin-top: 46px; max-width: 930px; }
  .chips { margin-top: 58px; }

  .right {
    position: absolute; left: 1140px; top: 0; width: 1310px; height: 100%;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
  }
  .shot {
    border-radius: 18px; overflow: hidden; width: 1290px;
    border: 1px solid rgba(255, 255, 255, 0.10);
    box-shadow: 0 0 0 1px rgba(0,0,0,0.55), 0 34px 90px rgba(0,0,0,0.55),
                0 0 110px rgba(249, 115, 22, 0.16), inset 0 1px 0 rgba(255,255,255,0.04);
  }
  .shot img { width: 1290px; height: auto; display: block; }
  .caption { margin-top: 26px; }
</style>
</head>
<body>
  <div class="stage">
    <div class="topglow"></div>
    <div class="divider" style="left:1092px; top:64px; bottom:64px;"></div>
    <div class="divider-dot" style="left:1084px; top:712px;"></div>

    <div class="left">
      <div class="brand">
        <img src="data:image/png;base64,__APP__" alt="SqlKit">
        <div>
          <div class="brand-name">SqlKit</div>
          <div class="brand-sub">DATABASE AGENT</div>
        </div>
      </div>
      <div class="eyebrow">Agentic Data Studio</div>
      <h1 class="headline">
        <span class="l1">DESCRIBE IT.</span>
        <span class="l2"><span class="accent">SQLKIT WRITES IT.</span></span>
      </h1>
      <p class="sub">__SUB__</p>
      <div class="chips">
        <div class="chip">Native · <b>Rust + Tauri</b></div>
        <div class="chip">BYOK · <b>Credentials stay local</b></div>
      </div>
    </div>

    <div class="right">
      <div class="shot">
        <img src="data:image/png;base64,__UI__" alt="SqlKit Agentic Data Studio">
      </div>
      <div class="caption">Agentic Data Studio — describe it, it writes and runs the SQL</div>
    </div>
  </div>
</body>
</html>
"""
    html = (html.replace("__APP__", APP_B64)
                .replace("__UI__", SQLKIT_UI_B64)
                .replace("__SUB__", sub))
    with open(os.path.join(OUT, "agentic-ai.html"), "w") as f:
        f.write(html)


def build_mcp():
    agent_tiles = "".join(
        '<div class="agent"><div class="tile-logo"><img src="data:image/png;base64,%s" alt="%s"></div>'
        '<div class="tile-name small">%s</div></div>' % (AGENT_B64[name], name, name)
        for name, fn in AGENTS
    )
    row1 = "".join(tile(sql_icon(n, f, f"r1{i}"), n, "db") for i, (n, f) in enumerate(SQL_ROW1))
    row2 = "".join(tile(sql_icon(n, f, f"r2{i}"), n, "db") for i, (n, f) in enumerate(SQL_ROW2))
    row3 = "".join(
        tile(icon_html(n, f, k, f"ns{i}"), n, "db")
        for i, (n, f, k) in enumerate(LAST_ROW)
    )

    sub = ("One local MCP server connects your coding agent to both SqlKit "
           "(SQL · localhost:9121) and DocKit (NoSQL · localhost:9120). Claude Code, Cursor, "
           "Codex, or any MCP client can list tables, inspect schemas, and run queries. "
           "Reads on by default; writes and DDL only after you approve.")
    html = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<link rel="icon" href="data:,">
<link rel="stylesheet" href="promo-base.css">
<title>SqlKit — Data Studio MCP</title>
<style>
  html, body { width: 2560px; height: 1440px; }
  .stage { width: 2560px; height: 1440px; }

  .left {
    position: absolute; left: 0; top: 0; width: 1110px; height: 100%;
    padding: 0 40px 0 128px;
    display: flex; flex-direction: column; justify-content: center;
  }
  .brand { margin-bottom: 56px; }
  .brand img { width: 64px; height: 64px; }
  .eyebrow { margin-bottom: 28px; }
  .headline { font-size: 82px; }
  .headline span { display: block; }
  .headline .accent {
    background: linear-gradient(180deg, #ffe0bd 0%, #ffb469 30%, #fb923c 62%, #f97316 90%);
    -webkit-background-clip: text; background-clip: text; color: transparent;
  }
  .sub { margin-top: 38px; max-width: 930px; font-size: 27px; }
  .chips { margin-top: 50px; }

  .right { position: absolute; left: 1140px; top: 0; width: 1310px; height: 100%; }
  .diagram { position: absolute; left: 0; top: 190px; width: 1310px; height: 1080px; }
  .lines { position: absolute; left: 0; top: 0; pointer-events: none; }

  .agent-row, .db-row {
    position: absolute; left: 50%; transform: translateX(-50%);
    display: flex; gap: 12px; justify-content: center;
  }
  .agent-row { top: 0; width: 1044px; }
  .agent {
    width: 120px; height: 130px; border-radius: 18px;
    background: linear-gradient(180deg, rgba(255,255,255,0.045), rgba(255,255,255,0.015));
    border: 1px solid rgba(255,255,255,0.085);
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.05), 0 10px 24px rgba(0,0,0,0.18);
    display: flex; flex-direction: column; align-items: center; justify-content: center;
  }
  .agent .tile-logo { width: 100%; height: 62px; display: flex; align-items: center; justify-content: center; }
  .agent img { width: 44px; height: 44px; object-fit: contain; }
  .agent .tile-name { margin-top: 7px; }

  .mcp {
    position: absolute; left: 50%; transform: translateX(-50%); top: 196px;
    width: 520px; height: 104px; border-radius: 20px;
    background: linear-gradient(180deg, rgba(249,115,22,0.16), rgba(249,115,22,0.05));
    border: 1px solid rgba(249, 115, 22, 0.45);
    box-shadow: 0 0 0 1px rgba(0,0,0,0.35), 0 22px 60px rgba(0,0,0,0.45),
                0 0 70px rgba(249,115,22,0.20), inset 0 1px 0 rgba(255,255,255,0.06);
    display: flex; align-items: center; padding: 0 28px; gap: 20px;
  }
  .mcp img { width: 56px; height: 56px; border-radius: 13px;
    box-shadow: 0 6px 18px rgba(0,0,0,0.45), 0 0 0 1px rgba(255,255,255,0.10); }
  .mcp-t1 { font-size: 26px; font-weight: 800; letter-spacing: -0.01em; color: #fff7ed; }
  .mcp-t2 { font-size: 17px; font-weight: 500; color: rgba(255, 237, 213, 0.78); margin-top: 6px; }

  .product-row {
    position: absolute; left: 50%; transform: translateX(-50%); top: 372px;
    display: flex; gap: 24px; justify-content: center;
  }
  .product {
    width: 340px; height: 118px; border-radius: 18px;
    background: linear-gradient(180deg, rgba(255,255,255,0.05), rgba(255,255,255,0.02));
    border: 1px solid rgba(255,255,255,0.10);
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.05), 0 14px 34px rgba(0,0,0,0.30);
    display: flex; align-items: center; padding: 0 22px; gap: 18px;
  }
  .product img { width: 44px; height: 44px; border-radius: 11px;
    box-shadow: 0 5px 14px rgba(0,0,0,0.42), 0 0 0 1px rgba(255,255,255,0.09); }
  .product-t1 { font-size: 23px; font-weight: 800; color: #eef2f8; letter-spacing: -0.01em; }
  .product-t2 { font-size: 15px; font-weight: 500; color: rgba(255,255,255,0.62); margin-top: 5px;
    font-variant-numeric: tabular-nums; }

  .db-row { width: 1284px; }
  .row1 { top: 566px; }
  .row2 { top: 710px; }
  .row3 { top: 854px; width: 1284px; }
  .db {
    width: 150px; height: 132px; border-radius: 18px;
    background: linear-gradient(180deg, rgba(255,255,255,0.045), rgba(255,255,255,0.015));
    border: 1px solid rgba(255,255,255,0.085);
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.05), 0 10px 24px rgba(0,0,0,0.18);
    display: flex; flex-direction: column; align-items: center; justify-content: center;
  }
  .db .tile-logo { width: 100%; height: 70px; display: flex; align-items: center; justify-content: center; }
  .db .tile-logo svg { width: 52px !important; height: 52px !important; }
  .db .tile-logo img { width: 52px; height: 52px; object-fit: contain; }
  .db .tile-name { margin-top: 8px; }

  .note {
    position: absolute; left: 50%; transform: translateX(-50%); top: 1014px; width: 1240px;
    text-align: center; font-size: 18px; color: rgba(255,255,255,0.45); line-height: 1.4;
  }
</style>
</head>
<body>
  <div class="stage">
    <div class="topglow"></div>
    <div class="divider" style="left:1092px; top:64px; bottom:64px;"></div>
    <div class="divider-dot" style="left:1084px; top:712px;"></div>

    <div class="left">
      <div class="brand">
        <img src="data:image/png;base64,__APP__" alt="SqlKit">
        <div>
          <div class="brand-name">SqlKit</div>
          <div class="brand-sub">DATABASE AGENT</div>
        </div>
      </div>
      <div class="eyebrow">Data Studio MCP</div>
      <h1 class="headline">
        <span>YOUR CODING AGENT,</span>
        <span>CONNECTED TO</span>
        <span class="accent">YOUR DATABASES.</span>
      </h1>
      <p class="sub">__SUB__</p>
      <div class="chips">
        <div class="chip">SQL · <b>:9121</b></div>
        <div class="chip">NoSQL · <b>:9120</b></div>
        <div class="chip">Read-safe by default</div>
      </div>
    </div>

    <div class="right">
      <div class="diagram">
        <svg class="lines" width="1310" height="1080">
          <defs>
            <marker id="arr" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
              <path d="M0 0 L10 5 L0 10 z" fill="#fb923c"/>
            </marker>
          </defs>
          <line x1="655" y1="130" x2="655" y2="194" stroke="#fb923c" stroke-width="2.5" marker-end="url(#arr)"/>
          <line x1="652" y1="130" x2="652" y2="194" stroke="rgba(249,115,22,0.20)" stroke-width="9" stroke-linecap="round"/>
          <text x="672" y="167" font-family="inherit" font-size="19" fill="rgba(255,255,255,0.5)">MCP protocol</text>

          <line x1="655" y1="300" x2="655" y2="338" stroke="#fb923c" stroke-width="2.5"/>
          <line x1="473" y1="338" x2="655" y2="338" stroke="#fb923c" stroke-width="2.5"/>
          <line x1="837" y1="338" x2="655" y2="338" stroke="#fb923c" stroke-width="2.5"/>
          <line x1="473" y1="338" x2="473" y2="370" stroke="#fb923c" stroke-width="2.5" marker-end="url(#arr)"/>
          <line x1="837" y1="338" x2="837" y2="370" stroke="#fb923c" stroke-width="2.5" marker-end="url(#arr)"/>

          <line x1="473" y1="490" x2="473" y2="528" stroke="#fb923c" stroke-width="2.5"/>
          <line x1="837" y1="490" x2="837" y2="528" stroke="#fb923c" stroke-width="2.5"/>
          <line x1="473" y1="528" x2="837" y2="528" stroke="#fb923c" stroke-width="2.5"/>
          <line x1="655" y1="528" x2="655" y2="564" stroke="#fb923c" stroke-width="2.5" marker-end="url(#arr)"/>
        </svg>
        <div class="agent-row">__AGENTS__</div>
        <div class="mcp">
          <img src="data:image/png;base64,__APP__" alt="SqlKit">
          <div>
            <div class="mcp-t1">Data Studio MCP Server</div>
            <div class="mcp-t2">local · reads on by default</div>
          </div>
        </div>
        <div class="product-row">
          <div class="product">
            <img src="data:image/png;base64,__APP__" alt="SqlKit">
            <div>
              <div class="product-t1">SqlKit</div>
              <div class="product-t2">SQL bridge · localhost:9121</div>
            </div>
          </div>
          <div class="product">
            <img src="data:image/png;base64,__DOCKIT__" alt="DocKit">
            <div>
              <div class="product-t1">DocKit</div>
              <div class="product-t2">NoSQL bridge · localhost:9120</div>
            </div>
          </div>
        </div>
        <div class="db-row row1">__ROW1__</div>
        <div class="db-row row2">__ROW2__</div>
        <div class="db-row row3">__ROW3__</div>
        <div class="note">Credentials never leave your machine — the MCP server talks to SqlKit and DocKit locally.</div>
      </div>
    </div>
  </div>
</body>
</html>
"""
    html = (html.replace("__APP__", APP_B64)
                .replace("__DOCKIT__", DOCKIT_B64)
                .replace("__SUB__", sub)
                .replace("__AGENTS__", agent_tiles)
                .replace("__ROW1__", row1)
                .replace("__ROW2__", row2)
                .replace("__ROW3__", row3))
    with open(os.path.join(OUT, "data-studio-mcp.html"), "w") as f:
        f.write(html)


SHOT_ASSETS = [
    dict(
        filename="hero-ui.html",
        title="SqlKit — SQL Database Client",
        eyebrow="SQL Database Client",
        l1="EVERY DATABASE.",
        l2="ONE NATIVE APP.",
        sub=("70+ database engines, AI built in, one download. Native Rust + Tauri — "
             "no Electron, no JVM. Free and open source."),
        chips=["macOS · Windows · Linux", "Apache 2.0"],
        caption="Connection manager — 17 databases, one workspace.",
        source="sqlkit-client-ui.png",
        alt="SqlKit connection manager",
    ),
    dict(
        filename="editor-schema.html",
        title="SqlKit — Product-Grade Editor",
        eyebrow="Product-Grade Editor",
        l1="WRITE SQL IN",
        l2="A REAL EDITOR.",
        sub=("Monaco, the engine behind VS Code. SQL autocomplete, multi-tab editing, "
             "and a schema browser right beside your query. Results in milliseconds."),
        chips=["Monaco", "Multi-tab"],
        caption="Editor + schema browser — autocomplete and results side by side.",
        source="sqlkit-sql-editor-ui.png",
        alt="SqlKit SQL editor and schema browser",
    ),
    dict(
        filename="data-grid.html",
        title="SqlKit — Smart Data Grid",
        eyebrow="Smart Data Grid",
        l1="BROWSE, EDIT,",
        l2="EXPORT.",
        sub=("Virtual-scrolled grid with column search, SQL filters, inline editing, "
             "and CSV, JSON, or Markdown export. Thousands of rows, zero lag."),
        chips=["Sort · Filter · Edit", "CSV · JSON · Markdown"],
        caption="8,500 rows at 163ms — sort, filter, edit inline.",
        source="sqlkit-data-view-table-ui.png",
        alt="SqlKit data grid",
    ),
    dict(
        filename="er-diagram.html",
        title="SqlKit — ER Diagrams",
        eyebrow="ER Diagrams",
        l1="SEE HOW YOUR",
        l2="DATA CONNECTS.",
        sub=("Interactive ER diagrams with automatic layout. Foreign keys, indexes, "
             "and table relationships mapped visually from your live schema."),
        chips=["Auto layout", "Live schema"],
        caption="10 tables, 6 relations — generated from the live schema.",
        source="sqlkit-er-diagram-ui.png",
        alt="SqlKit ER diagram",
    ),
]

_SHOT_TEMPLATE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<link rel="icon" href="data:,">
<link rel="stylesheet" href="promo-base.css">
<title>__TITLE__</title>
<style>
  html, body { width: 2560px; height: 1440px; }
  .stage { width: 2560px; height: 1440px; }

  .left {
    position: absolute; left: 0; top: 0; width: 1110px; height: 100%;
    padding: 0 40px 0 128px;
    display: flex; flex-direction: column; justify-content: center;
  }
  .brand { margin-bottom: 62px; }
  .brand img { width: 64px; height: 64px; }
  .eyebrow { margin-bottom: 30px; }
  .headline { font-size: 84px; }
  .headline .l1 { display: block; }
  .headline .l2 { display: block; margin-top: 4px; }
  .headline .l2 .accent {
    background: linear-gradient(180deg, #ffe0bd 0%, #ffb469 30%, #fb923c 62%, #f97316 90%);
    -webkit-background-clip: text; background-clip: text; color: transparent;
  }
  .sub { margin-top: 44px; max-width: 930px; font-size: 29px; }
  .chips { margin-top: 56px; }

  .right {
    position: absolute; left: 1140px; top: 0; width: 1310px; height: 100%;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
  }
  .shot {
    border-radius: 18px; overflow: hidden; width: 1290px;
    border: 1px solid rgba(255, 255, 255, 0.10);
    box-shadow: 0 0 0 1px rgba(0,0,0,0.55), 0 34px 90px rgba(0,0,0,0.55),
                0 0 110px rgba(249, 115, 22, 0.16), inset 0 1px 0 rgba(255,255,255,0.04);
  }
  .shot img { width: 1290px; height: auto; display: block; }
  .caption { margin-top: 26px; font-size: 18px; color: rgba(255,255,255,0.5); }
</style>
</head>
<body>
  <div class="stage">
    <div class="topglow"></div>
    <div class="divider" style="left:1092px; top:64px; bottom:64px;"></div>
    <div class="divider-dot" style="left:1084px; top:712px;"></div>

    <div class="left">
      <div class="brand">
        <img src="data:image/png;base64,__APP__" alt="SqlKit">
        <div>
          <div class="brand-name">SqlKit</div>
          <div class="brand-sub">DATABASE AGENT</div>
        </div>
      </div>
      <div class="eyebrow">__EYEBROW__</div>
      <h1 class="headline">
        <span class="l1">__L1__</span>
        <span class="l2"><span class="accent">__L2__</span></span>
      </h1>
      <p class="sub">__SUB__</p>
      <div class="chips">
        __CHIPS__
      </div>
    </div>

    <div class="right">
      <div class="shot">
        <img src="data:image/png;base64,__SHOT__" alt="__ALT__">
      </div>
      <div class="caption">__CAPTION__</div>
    </div>
  </div>
</body>
</html>
"""


def build_shots():
    for a in SHOT_ASSETS:
        chips = "".join('<div class="chip">%s</div>' % c for c in a["chips"])
        html = (_SHOT_TEMPLATE
                .replace("__TITLE__", a["title"])
                .replace("__EYEBROW__", a["eyebrow"])
                .replace("__L1__", a["l1"])
                .replace("__L2__", a["l2"])
                .replace("__SUB__", a["sub"])
                .replace("__CHIPS__", chips)
                .replace("__CAPTION__", a["caption"])
                .replace("__ALT__", a["alt"])
                .replace("__APP__", APP_B64)
                .replace("__SHOT__", b64(os.path.join(SITE_PUBLIC, a["source"]))))
        with open(os.path.join(OUT, a["filename"]), "w") as f:
            f.write(html)


SQL_CODE = """<span class="tok-kw">SELECT</span>
  c.name,
  <span class="tok-fn">SUM</span>(o.amount) <span class="tok-kw">AS</span> revenue
<span class="tok-kw">FROM</span> customers c
<span class="tok-kw">JOIN</span> orders o <span class="tok-kw">ON</span> o.customer_id = c.id
<span class="tok-kw">WHERE</span> o.created_at <span class="tok-kw">&gt;=</span> <span class="tok-fn">date_trunc</span>(<span class="tok-str">'quarter'</span>, <span class="tok-fn">now</span>())
<span class="tok-kw">GROUP BY</span> c.name
<span class="tok-kw">ORDER BY</span> revenue <span class="tok-kw">DESC</span>
<span class="tok-kw">LIMIT</span> <span class="tok-num">5</span>;"""


def build_gif():
    html = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<link rel="icon" href="data:,">
<link rel="stylesheet" href="promo-base.css">
<title>SqlKit — Agentic Data Studio demo</title>
<style>
  html, body { width: 1280px; height: 800px; }
  .stage { width: 1280px; height: 800px; }

  .app {
    position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%);
    width: 1120px; height: 700px; border-radius: 18px; overflow: hidden;
    border: 1px solid rgba(255,255,255,0.10);
    background: linear-gradient(180deg, #111826, #0c1119);
    box-shadow: 0 0 0 1px rgba(0,0,0,0.5), 0 40px 110px rgba(0,0,0,0.6),
                0 0 110px rgba(249,115,22,0.12);
    display: flex; flex-direction: column;
  }
  .chrome { height: 46px; flex: 0 0 46px; display: flex; align-items: center; gap: 8px;
    padding: 0 18px; border-bottom: 1px solid rgba(255,255,255,0.07); }
  .dot { width: 12px; height: 12px; border-radius: 50%; }
  .chrome-title { margin-left: 12px; font-size: 14px; font-weight: 600; color: rgba(255,255,255,0.55); }
  .chrome .spacer { flex: 1; }
  .chrome-badge { font-size: 12px; font-weight: 700; color: #fb923c; letter-spacing: 0.14em;
    border: 1px solid rgba(249,115,22,0.4); padding: 4px 12px; border-radius: 999px;
    background: rgba(249,115,22,0.08); margin-left: 10px; }
  .chrome img { width: 22px; height: 22px; border-radius: 6px; }

  .body { flex: 1; display: flex; flex-direction: column; padding: 16px 26px 18px; min-height: 0; }

  .prompt-label { font-size: 12px; font-weight: 700; color: #fb923c; letter-spacing: 0.18em; margin-bottom: 9px; }
  .prompt-field {
    position: relative; height: 52px;
    border: 1px solid rgba(255,255,255,0.12); border-radius: 12px;
    background: rgba(255,255,255,0.03);
  }
  .prompt-field .g {
    position: absolute; left: 18px; top: 50%; transform: translateY(-50%);
    font-size: 20px; color: #eef2f8; white-space: nowrap; opacity: 0;
  }
  .caret { display: inline-block; width: 2px; height: 23px; background: #fb923c; margin-left: 4px;
    vertical-align: -4px; animation: blink 1s steps(1) infinite; }
  @keyframes blink { 0%, 55% { opacity: 1; } 56%, 100% { opacity: 0; } }
  .prompt-field .ph { color: rgba(255,255,255,0.28); }

  .editor { flex: 1; margin-top: 14px; min-height: 0; border-radius: 14px;
    border: 1px solid rgba(255,255,255,0.07); background: #0a0f17;
    display: flex; flex-direction: column; }
  .editor-head { display: flex; align-items: center; gap: 10px; padding: 9px 16px;
    border-bottom: 1px solid rgba(255,255,255,0.06); }
  .editor-tab { font-size: 13px; font-weight: 600; color: rgba(255,255,255,0.75);
    background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.08);
    padding: 4px 14px; border-radius: 8px; }
  .editor-tab .dbdot { display:inline-block; width:8px; height:8px; border-radius:50%;
    background:#fb923c; margin-right:7px; }
  .editor-code { position: relative; flex: 1; overflow: hidden; }
  .editor-code .g {
    position: absolute; left: 18px; top: 14px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 15px; line-height: 1.55; color: #dbe4f0; white-space: pre; opacity: 0;
    transition: opacity 0.24s ease;
  }
  .tok-kw { color: #fb923c; font-weight: 600; }
  .tok-fn { color: #7aa7ff; }
  .tok-str { color: #a5e08a; }
  .tok-num { color: #f0c674; }
  .editor-empty { position: absolute; left: 18px; top: 16px; color: rgba(255,255,255,0.14);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 15px;
    line-height: 1.55; white-space: pre; }

  .runbar { display: flex; align-items: center; gap: 14px; padding: 11px 16px;
    border-top: 1px solid rgba(255,255,255,0.06); }
  .run-btn { font-size: 14px; font-weight: 700; color: #fff; background: linear-gradient(180deg,#fb923c,#f97316);
    padding: 8px 18px; border-radius: 9px; box-shadow: 0 6px 18px rgba(249,115,22,0.35); }
  .runstat { font-size: 14px; color: rgba(255,255,255,0.45); }

  .results { margin-top: 12px; border-radius: 14px; overflow: hidden;
    border: 1px solid rgba(255,255,255,0.08); background: rgba(255,255,255,0.02); }
  .results-head { display: flex; align-items: center; padding: 10px 16px; }
  .results-title { font-size: 14px; font-weight: 700; color: #eef2f8; }
  .results-gen { font-size: 13px; color: #fb923c; margin-left: auto; font-weight: 600; opacity: 0;
    transition: opacity 0.24s ease; }
  table { width: 100%; border-collapse: collapse; }
  th { text-align: left; font-size: 11px; font-weight: 600; color: rgba(255,255,255,0.4);
    padding: 6px 16px; border-top: 1px solid rgba(255,255,255,0.06); letter-spacing: 0.08em;
    text-transform: uppercase; }
  tbody tr { opacity: 0; transition: opacity 0.24s ease; }
  td { font-size: 15px; color: #dbe4f0; padding: 6px 16px; border-top: 1px solid rgba(255,255,255,0.04);
    font-variant-numeric: tabular-nums; }
  td.rev { color: #fb923c; text-align: right; font-weight: 600; }
  td.idx { color: rgba(255,255,255,0.35); }
  .runstat { opacity: 0; transition: opacity 0.24s ease; }

  /* stage gating */
  body[data-stage="1"] .on1 { opacity: 1; }
  body[data-stage="2"] .on2 { opacity: 1; }
  body[data-stage="3"] .on3 { opacity: 1; }
  body[data-stage="4"] .on4 { opacity: 1; }
  body[data-stage="4"] .hide4 { opacity: 0; }
</style>
</head>
<body data-stage="1">
  <div class="stage">
    <div class="topglow"></div>
    <div class="app">
      <div class="chrome">
        <span class="dot" style="background:#ff5f57;"></span>
        <span class="dot" style="background:#febc2e;"></span>
        <span class="dot" style="background:#28c840;"></span>
        <span class="chrome-title">geekfun_db — postgres@localhost:5432</span>
        <span class="spacer"></span>
        <img src="data:image/png;base64,__APP__" alt="">
        <span class="chrome-badge">DATA STUDIO</span>
      </div>
      <div class="body">

        <div class="prompt">
          <div class="prompt-label">ASK SQLKIT</div>
          <div class="prompt-field">
            <span class="g on1 ph">Ask in plain language…<span class="caret"></span></span>
            <span class="g on2">Show top 5 customers by<span class="caret"></span></span>
            <span class="g on3">Show top 5 customers by revenue last quarter<span class="caret"></span></span>
            <span class="g on4" style="color:rgba(255,255,255,0.6)">Show top 5 customers by revenue last quarter</span>
          </div>
        </div>

        <div class="editor">
          <div class="editor-head"><span class="editor-tab"><span class="dbdot"></span>query.sql</span></div>
          <div class="editor-code">
            <div class="editor-empty">-- your SQL appears here
-- describe it in plain language</div>
            <div class="g on3 on4">__SQL__</div>
          </div>
          <div class="runbar">
            <span class="run-btn on1 on2 on3 hide4">Run</span>
            <span class="runstat on4" style="color:#a5e08a; font-weight:600;">&#10003; Ran in 2.4s</span>
            <span class="runstat on4">— 5 rows returned</span>
          </div>
        </div>

        <div class="results">
          <div class="results-head">
            <span class="results-title">Results</span>
            <span class="results-gen on4">Generated by SqlKit Agent — 2.4s</span>
          </div>
          <table>
            <thead><tr><th style="width:52px;">#</th><th>Customer</th><th style="text-align:right;">Revenue</th></tr></thead>
            <tbody>
              <tr class="on4"><td class="idx">1</td><td>Northwind Traders</td><td class="rev">$128,430</td></tr>
              <tr class="on4"><td class="idx">2</td><td>Acme Corp</td><td class="rev">$94,210</td></tr>
              <tr class="on4"><td class="idx">3</td><td>Globex</td><td class="rev">$71,875</td></tr>
              <tr class="on4"><td class="idx">4</td><td>Initech</td><td class="rev">$58,030</td></tr>
              <tr class="on4"><td class="idx">5</td><td>Umbrella Corp</td><td class="rev">$44,590</td></tr>
            </tbody>
          </table>
        </div>

      </div>
    </div>
  </div>
</body>
</html>
"""
    html = html.replace("__APP__", APP_B64).replace("__SQL__", SQL_CODE)
    with open(os.path.join(OUT, "agentic-ai-gif.html"), "w") as f:
        f.write(html)


ZH_FONT = ('-apple-system, BlinkMacSystemFont, "Segoe UI", "Inter", Roboto,\n'
           '      "Helvetica Neue", Arial, "PingFang SC", "Microsoft YaHei", '
           '"Noto Sans SC", sans-serif;')

DATABASES_PROMO = os.path.join(ROOT, "docs/product-hunt/databases-promo.html")

ZH_CSS_OVERVIEW = """<style>
  /* zh (Simplified Chinese) typographic overrides */
  .brand-sup { letter-spacing: .1em; }
  .eyebrow { letter-spacing: .15em; }
  .dbs { letter-spacing: -.02em; line-height: 1.15; }
  .tagline { letter-spacing: .05em; text-transform: none; }
  .sub { max-width: 808px; line-height: 1.62; }
</style>"""

ZH_CSS_AI = """<style>
  /* zh (Simplified Chinese) typographic overrides */
  .brand-sub { letter-spacing: .1em; }
  .eyebrow { letter-spacing: .15em; }
  .headline { line-height: 1.2; letter-spacing: -.02em; }
  .sub { line-height: 1.55; }
  .caption { line-height: 20px; }
</style>"""

ZH_CSS_MCP = """<style>
  /* zh (Simplified Chinese) typographic overrides */
  .brand-sub { letter-spacing: .1em; }
  .headline { line-height: 1.2; letter-spacing: -.02em; }
  .sub { line-height: 1.55; }
</style>"""

ZH_OVERVIEW = [
    ('<div class="brand-sup">DATABASE AGENT</div>',
     '<div class="brand-sup">数据库工作台</div>'),
    ('<div class="eyebrow">Support</div>',
     '<div class="eyebrow">支 持</div>'),
    ('<div class="dbs">DATABASES</div>',
     '<div class="dbs">数据库</div>'),
    ('<div class="tagline">All SQL in one client</div>',
     '<div class="tagline">一个客户端,搞定所有 SQL</div>'),
    ('<div class="sub">\n        One native app for every database. Rust + Tauri —<br>\n'
     '        no Electron, no JVM. AI agent built in.\n      </div>',
     '<div class="sub">\n        从 PostgreSQL 到 BigQuery,从本地到云端。<br>\n'
     '        原生 Rust + Tauri,无 Electron、无 JVM,数据与凭据都留在本机。\n      </div>'),
    ('<div class="chip">macOS&nbsp; · &nbsp;Windows&nbsp; · &nbsp;Linux</div>\n'
     '        <div class="chip">Free &amp; open source · <b>Apache 2.0</b></div>',
     '<div class="chip">开源开放 · <b>Apache 2.0</b></div>\n'
     '        <div class="chip">macOS / Windows / Linux</div>'),
]

ZH_AI = [
    ('<div class="brand-sub">DATABASE AGENT</div>',
     '<div class="brand-sub">数据库工作台</div>'),
    ('<div class="eyebrow">Agentic Data Studio</div>',
     '<div class="eyebrow">内置 AI AGENT</div>'),
    ('<h1 class="headline">\n        <span class="l1">DESCRIBE IT.</span>\n'
     '        <span class="l2"><span class="accent">SQLKIT WRITES IT.</span></span>\n      </h1>',
     '<h1 class="headline">\n        <span class="l1">用 AI Agent,</span>\n'
     '        <span class="l2"><span class="accent">和你的数据库对话。</span></span>\n      </h1>'),
    ('<p class="sub">Natural language to SQL. It reads your schema, optimizes slow queries, '
     'explains execution plans, and fixes errors. Bring your own key: OpenAI, Anthropic, '
     'DeepSeek, or Ollama.</p>',
     '<p class="sub">用自然语言说出需求,AI 读你的表结构:生成查询、优化慢 SQL、解释执行计划、'
     '一键修复报错。自带 Key:OpenAI / Anthropic / DeepSeek,或本地 Ollama 模型。'
     '凭据永远不出本机。</p>'),
    ('<div class="chip">Native · <b>Rust + Tauri</b></div>\n'
     '        <div class="chip">BYOK · <b>Credentials stay local</b></div>',
     '<div class="chip">BYOK · <b>凭据不出本机</b></div>\n'
     '        <div class="chip">支持本地模型</div>'),
    ('<div class="caption">Agentic Data Studio — describe it, it writes and runs the SQL</div>',
     '<div class="caption">Agentic Data Studio —— 和数据库对话,它写出并运行 SQL。</div>'),
]

ZH_MCP = [
    ('<div class="brand-sub">DATABASE AGENT</div>',
     '<div class="brand-sub">数据库工作台</div>'),
    ('<h1 class="headline">\n        <span>YOUR CODING AGENT,</span>\n'
     '        <span>CONNECTED TO</span>\n        <span class="accent">YOUR DATABASES.</span>\n      </h1>',
     '<h1 class="headline">\n        <span>你的编码 Agent,</span>\n'
     '        <span class="accent">直连你的数据库。</span>\n      </h1>'),
    ('<p class="sub">One local MCP server connects your coding agent to both SqlKit '
     '(SQL · localhost:9121) and DocKit (NoSQL · localhost:9120). Claude Code, Cursor, '
     'Codex, or any MCP client can list tables, inspect schemas, and run queries. '
     'Reads on by default; writes and DDL only after you approve.</p>',
     '<p class="sub">本地 MCP 服务(localhost:9121)把 SQL 和 NoSQL 数据库接入 Claude Code、'
     'Cursor、Codex 等任意 MCP 客户端。默认只读;写操作与 DDL 需你显式授权。</p>'),
    ('<div class="chip">SQL · <b>:9121</b></div>\n'
     '        <div class="chip">NoSQL · <b>:9120</b></div>\n'
     '        <div class="chip">Read-safe by default</div>',
     '<div class="chip">本地运行</div>\n'
     '        <div class="chip">默认只读</div>'),
    ('<text x="672" y="167" font-family="inherit" font-size="19" '
     'fill="rgba(255,255,255,0.5)">MCP protocol</text>',
     '<text x="672" y="167" font-family="inherit" font-size="19" '
     'fill="rgba(255,255,255,0.5)">MCP 协议</text>'),
    ('<line x1="655" y1="300" x2="655" y2="338" stroke="#fb923c" stroke-width="2.5"/>',
     '<line x1="655" y1="300" x2="655" y2="338" stroke="#fb923c" stroke-width="2.5"/>\n'
     '          <text x="672" y="324" font-family="inherit" font-size="19" '
     'fill="rgba(255,255,255,0.5)">默认只读</text>'),
    ('<div class="mcp-t1">Data Studio MCP Server</div>',
     '<div class="mcp-t1">Data Studio MCP 服务</div>'),
    ('<div class="mcp-t2">local · reads on by default</div>',
     '<div class="mcp-t2">本地运行 · 默认只读</div>'),
    ('<div class="product-t2">SQL bridge · localhost:9121</div>',
     '<div class="product-t2">SQL 桥 · localhost:9121</div>'),
    ('<div class="product-t2">NoSQL bridge · localhost:9120</div>',
     '<div class="product-t2">NoSQL 桥 · localhost:9120</div>'),
    ('<div class="note">Credentials never leave your machine — the MCP server talks to '
     'SqlKit and DocKit locally.</div>',
     '<div class="note">凭据不离开本机 —— MCP 服务只在本地与 SqlKit / DocKit 通信。</div>'),
]


def build_zh(src_path, out_name, replacements, extra_css, title, font_block=None):
    s = open(src_path, encoding="utf-8").read()
    for old, new in replacements:
        if old not in s:
            raise SystemExit(f"[zh] anchor missing in {os.path.basename(src_path)}: {old[:70]!r}")
        s = s.replace(old, new)
    if font_block:
        s = s.replace(font_block[0], font_block[1], 1)
    s = re.sub(r"<title>.*?</title>", title, s, count=1, flags=re.S)
    s = s.replace("</head>", extra_css + "\n</head>", 1)
    with open(os.path.join(OUT, out_name), "w", encoding="utf-8") as f:
        f.write(s)


EN_FONT_OVERVIEW = ('font-family:-apple-system, BlinkMacSystemFont, "Segoe UI", "Inter", Roboto,\n'
                    '      "Helvetica Neue", Arial, sans-serif;')


def build_zh_assets():
    build_zh(DATABASES_PROMO, "zh-overview.html", ZH_OVERVIEW, ZH_CSS_OVERVIEW,
             "<title>SqlKit — 支持 70+ 数据库,一个客户端搞定所有 SQL</title>",
             font_block=(EN_FONT_OVERVIEW, "font-family:" + ZH_FONT))
    build_zh(os.path.join(OUT, "agentic-ai.html"), "zh-ai.html", ZH_AI, ZH_CSS_AI,
             "<title>SqlKit — 用 AI Agent,和你的数据库对话</title>")
    build_zh(os.path.join(OUT, "data-studio-mcp.html"), "zh-mcp.html", ZH_MCP, ZH_CSS_MCP,
             "<title>SqlKit — Data Studio MCP,让编码 Agent 直连数据库</title>")


build_agentic()
build_mcp()
build_shots()
build_gif()
build_zh_assets()
print("wrote HTMLs to", OUT)
