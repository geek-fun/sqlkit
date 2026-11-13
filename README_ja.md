<div align="center">

<img src="app-icon.png" width="120" alt="SqlKit Logo"/>

# SqlKit

**Agentic なクロスプラットフォーム SQL データベース GUI クライアント — 70 以上のデータベースに対応する、あなたのデータベースエージェント。**

**プライバシー第一。あなたのデータ、あなたのキー。オープンソース。**

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

[ダウンロード](https://www.geekfun.club/download) · [ドキュメント](docs/DISTRIBUTION.md) · [ウェブサイト](https://www.geekfun.club) · [リリース](https://github.com/geek-fun/sqlkit/releases)

[English](README.md) · [简体中文](README_zh.md) · [Português (BR)](README_pt-BR.md) · 日本語 · [한국어](README_ko.md) · [Español](README_es.md)

</div>

---

SqlKit は **Agentic なデータベースクライアント** です — 単に SQL を実行するだけではなく、あなたのデータベースを理解し、あなたに代わって操作します。必要なことを自然言語で伝えれば、エージェントがクエリを書き、スキーマを調査し、SQL を最適化して結果を返します。Electron ではなく Tauri（Rust）で構築されており、DBeaver、Navicat、DataGrip といった重厚なクライアントを 1 つのネイティブデスクトップアプリに置き換えます。

<p align="center">
  <img src="docs/images/sqlkit-client-ui.png" width="800" alt="SqlKit クライアント UI"/>
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

## インストール

<a href="https://www.geekfun.club/download">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?logo=download&logoColor=white">
    <img src="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?logo=download&logoColor=white" alt="ダウンロード">
  </picture>
</a>
&nbsp;
<a href="https://github.com/geek-fun/sqlkit/releases">
  <img src="https://img.shields.io/badge/Releases-GitHub-lightgrey?logo=github" alt="リリース">
</a>
&nbsp;
<a href="https://www.geekfun.club/products/sqlkit/">
  <img src="https://img.shields.io/badge/Website-geekfun.club-blue?logo=google-chrome&logoColor=white" alt="ウェブサイト">
</a>

## 機能

### Agentic データスタジオ

必要なことを自然言語で記述するだけです — エージェントがスキーマを読み取り、クエリを書き、遅い SQL を最適化し、実行プランをわかりやすく可視化し、エラーをその場で修正します。OpenAI、Anthropic、DeepSeek、Ollama に対応。API キーはご自身のものをご利用ください。

- **AI クエリ生成** — スキーマを把握したコンテキストで自然言語から SQL を生成
- **SQL 最適化** — 遅いクエリを書き換え、実行プランを可視化
- **エラー修正** — エージェントが SQL エラーを自動で診断・修正
- **安全性** — 破壊的な操作には確認が必要。認証情報が LLM に渡されることはありません

### 軽量でネイティブなパフォーマンス

Rust + Tauri v2 で構築 — Electron も JRE もバンドルされた Chromium もありません。小さなネイティブバイナリとして提供され、macOS、Windows、Linux でネイティブのパフォーマンスを発揮します。他のアプリなら Java ランタイムや JetBrains ライセンスを必要とする処理も、SqlKit なら 1 回のダウンロードで実現します。

### すべてのデータベースを 1 つのアプリで

SqlKit は 4 つのアダプタ戦略によって **70 以上のデータベース** に対応しています:

| 戦略 | データベース |
|----------|-----------|
| **ネイティブ** (Rust) | PostgreSQL, MySQL, SQL Server, SQLite |
| **PG プロトコル互換** | CockroachDB, Redshift, YugabyteDB, TimescaleDB, QuestDB, Vastbase, YashanDB, KingbaseES, GaussDB, HighGo, UXDB, OpenGauss, GBase8c, Greenplum, EnterpriseDB, CrateDB, Materialize, AlloyDB, CloudSQLPG, FujitsuPG |
| **MySQL プロトコル互換** | MariaDB, TiDB, OceanBase, TDSQL, PolarDB, Doris, SelectDB, StarRocks, Databend, GoldenDB, ManticoreSearch, SingleStore, CloudSQLMySQL |
| **JDBC ブリッジ** | Oracle, DuckDB, Firebird, DB2, H2, Snowflake, TDengine, Derby, Hive, Databricks, Hana, Teradata, Vertica, Exasol, BigQuery, Informix, Kylin, Cassandra, Iris, Access, Dameng, XuguDB, GBase8a |
| **HTTP ブリッジ** | ClickHouse, Trino, Presto, RQLite, Turso |

### プロダクトグレードのエディター

Monaco（VS Code エンジン）を搭載し、完全な SQL シンタックスハイライト、自動補完、マルチタブをサポートします。スキーマを視覚的に閲覧し、DDL を確認し、データベースを横断してオブジェクトを検索できます。

- **Monaco Editor** — シンタックスハイライトと自動補完を備えた VS Code グレードの SQL 編集
- **マルチタブ** — タブ管理で複数のクエリを同時に処理
- **スキーマブラウザー** — データベース、スキーマ、テーブル、カラム、インデックスのツリービュー
- **クエリ履歴** — 自動保存、検索可能、再実行可能
- **結果グリッド** — ページネーション、ソート、インライン編集、CSV/JSON/Markdown エクスポートに対応
- **DDL ビューアー** — 任意のオブジェクトの CREATE 文を表示
- **オブジェクト検索** — スキーマを横断してテーブル、ビュー、プロシージャをすばやく検索

### 大量データ転送

対応する任意のエンジン間でデータを移動できます — PostgreSQL から ClickHouse へ、Oracle から SQL Server へ、MySQL から BigQuery へ。中間ファイルは不要です。

- **クロスエンジン転送** — 異なる種類のデータベース間でデータを移行、型のマッピングは自動
- **インポート / エクスポート** — CSV、JSON、JSONL 形式に対応
- **バルク操作** — バッチ処理で数百万件のレコードを処理

### セキュリティと接続

- **SSH トンネル** — 鍵またはパスワード認証による SSH 接続
- **暗号化ストレージ** — 認証情報は OS のキーチェーンで保護（macOS Keychain、Windows Credential Manager、Linux Secret Service）
- **SSL/TLS** — 対応データベースへの暗号化接続
- **自動再接続** — 堅牢な接続管理

### クロスプラットフォーム

- **macOS**（Apple Silicon と Intel）— ネイティブの `.dmg` インストーラー
- **Windows** — `.exe` インストーラー
- **Linux** — `.AppImage` と `.deb` パッケージ

## 開発

### 前提条件

- [Node.js](https://nodejs.org/) >= 18
- [Rust ツールチェーン](https://www.rust-lang.org/tools/install)

**Linux (Ubuntu/Debian):**

```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev
```

**macOS:**

```bash
xcode-select --install
```

**Windows:** [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) をインストールしてください

### クイックスタート

```bash
npm install
npm run tauri dev
```

### スクリプト

| コマンド | 説明 |
|---------|-------------|
| `npm run dev` | Vite 開発サーバーを起動 |
| `npm run build` | フロントエンドの本番用ビルド |
| `npm run lint:check` | ESLint を実行 |
| `npm run lint:fix` | Lint の問題を自動修正 |
| `npm test` | フロントエンドのテストを実行 |
| `npm run tauri dev` | Tauri 開発モードを起動 |
| `npm run tauri build` | Tauri アプリケーションをビルド |

### ビルド手順

プラットフォームごとのビルド要件とトラブルシューティングの詳細は [BUILD.md](BUILD.md) を参照してください。

## 技術スタック

| レイヤー | 技術 |
|-------|------------|
| フレームワーク | [Tauri v2](https://tauri.app/) (Rust) |
| フロントエンド | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| UI | [shadcn-vue](https://www.shadcn-vue.com/) + [UnoCSS](https://unocss.dev/) |
| エディター | [Monaco Editor](https://microsoft.github.io/monaco-editor/) |
| データベース | [sqlx](https://github.com/launchbadge/sqlx) + エンジン固有のドライバー |

## よくある質問

<details>
<summary><strong>SqlKit は無料ですか？</strong></summary>

はい — **Community エディション**は Apache 2.0 ライセンスの下で無料かつオープンソースです。接続、SQL 編集、スキーマ閲覧、データ編集、クエリ履歴、結果のエクスポートはすべて無料で、人為的な制限はありません。

高度な機能 — AI エージェント（お使いの LLM を利用）、MCP サーバー、ER 図、一括インポート/エクスポート、クロスエンジンデータ転送 — は、有料の **GEEKFUN Data Studio Ultimate** サブスクリプション（月額 $9.9 または年額 $99、7 日間の無料トライアル付き）の一部です。サブスクリプション期間中にリリースされたバージョンは、後で解約しても永久にあなたのものです。

[料金を見る →](https://www.geekfun.club/pricing)
</details>

<details>
<summary><strong>SqlKit はデータを外部に送信しますか？</strong></summary>
いいえ。SqlKit はテレメトリーを収集しません。自動更新機能は GitHub Releases で新しいバージョンを確認します — 設定から無効化できます。認証情報とクエリはあなたのマシンから出ることはありません。
</details>

<details>
<summary><strong>インターネット接続なしで SqlKit を使えますか？</strong></summary>
はい。デスクトップアプリは完全にオフラインで動作します。AI 機能にはモデルエンドポイントへのネットワークアクセス（または Ollama によるローカルモデル）が必要です。
</details>

<details>
<summary><strong>SqlKit は DBeaver / TablePlus / DataGrip と何が違いますか？</strong></summary>
SqlKit はネイティブの Tauri アプリ（Rust）です — Java JRE も Electron のオーバーヘッドもありません。AI をプラグインではなくネイティブに搭載し、70 以上のデータベースに対応し、1 つのバイナリで macOS、Windows、Linux 上で動作します。プライバシー第一、認証情報はローカルで暗号化して保存されます。
</details>

<details>
<summary><strong>対応しているデータベースは何ですか？</strong></summary>
PostgreSQL、MySQL、Oracle、SQL Server、SQLite、DuckDB、ClickHouse、Firebird、MariaDB、CockroachDB、TiDB、OceanBase、Snowflake、DB2、H2、Trino、Greenplum、CrateDB、SingleStore、その他 60 以上。完全なリストは<a href="#multi-database-support">マルチデータベース対応</a>のセクションを参照してください。
</details>

<details>
<summary><strong>バグの報告や機能のリクエストはどのように行えばよいですか？</strong></summary>
<a href="https://github.com/geek-fun/sqlkit/issues">GitHub Issues</a> で issue をオープンしてください。
</details>

## コミュニティ

<p align="center">
  <img src="docs/images/wechat_official.png" width="300" alt="WeChat 公式アカウント">
</p>

<p align="center">
  <a href="https://x.com/geekfun_club"><img src="https://img.shields.io/badge/X-Follow-000000?logo=x&logoColor=white" alt="X / Twitter" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://www.youtube.com/@geekfun-club"><img src="https://img.shields.io/badge/YouTube-Subscribe-FF0000?logo=youtube&logoColor=white" alt="YouTube" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://github.com/geek-fun"><img src="https://img.shields.io/badge/GitHub-Follow-181717?logo=github&logoColor=white" alt="GitHub" /></a>
</p>

## スポンサー

<p align="center">
  <img src="docs/images/wechat_ponsor.jpg" width="140" alt="WeChat スポンサー QR コード">
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <a href="https://github.com/sponsors/geek-fun"><img src="https://img.shields.io/badge/GitHub_Sponsors-%E2%9D%A4_Support-EA4AAA?logo=githubsponsors&logoColor=white" alt="GitHub Sponsors" /></a>
</p>

## ライセンス

[Apache 2.0](LICENSE) © GEEKFUN
