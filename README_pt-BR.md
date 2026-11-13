<div align="center">

<img src="app-icon.png" width="120" alt="SqlKit Logo"/>

# SqlKit

**Cliente GUI de banco de dados SQL agêntico e multiplataforma — seu agente de banco de dados para mais de 70 bancos de dados.**

**Privacidade em primeiro lugar. Seus dados, suas chaves. Código aberto.**

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

[Baixar](https://www.geekfun.club/download) · [Documentação](docs/DISTRIBUTION.md) · [Site](https://www.geekfun.club) · [Versões](https://github.com/geek-fun/sqlkit/releases)

[English](README.md) · [简体中文](README_zh.md) · Português (BR) · [日本語](README_ja.md) · [한국어](README_ko.md) · [Español](README_es.md)

</div>

---

SqlKit é um **cliente de banco de dados agêntico** — ele não apenas executa SQL, ele entende seus bancos de dados e age em seu nome. Descreva o que você precisa em linguagem natural e o agente escreve consultas, inspeciona esquemas, otimiza SQL e retorna resultados. Construído com Tauri (Rust), e não com Electron, ele substitui clientes pesados como DBeaver, Navicat e DataGrip por um único aplicativo desktop nativo.

<p align="center">
  <img src="docs/images/sqlkit-client-ui.png" width="800" alt="Interface do cliente SqlKit"/>
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

## Instalação

<a href="https://www.geekfun.club/download">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?logo=download&logoColor=white">
    <img src="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?logo=download&logoColor=white" alt="Baixar">
  </picture>
</a>
&nbsp;
<a href="https://github.com/geek-fun/sqlkit/releases">
  <img src="https://img.shields.io/badge/Releases-GitHub-lightgrey?logo=github" alt="Versões">
</a>
&nbsp;
<a href="https://www.geekfun.club/products/sqlkit/">
  <img src="https://img.shields.io/badge/Website-geekfun.club-blue?logo=google-chrome&logoColor=white" alt="Site">
</a>

## Funcionalidades

### Estúdio de Dados Agêntico

Descreva o que você precisa em linguagem natural — o agente lê seu esquema, escreve consultas, otimiza SQL lentas, explica planos de execução visualmente e corrige erros na hora. Compatível com OpenAI, Anthropic, DeepSeek e Ollama. Traga sua própria chave.

- **Geração de consultas com IA** — de linguagem natural para SQL com contexto ciente do esquema
- **Otimização de SQL** — reescreve consultas lentas e visualiza planos de execução
- **Correção de erros** — o agente diagnostica e corrige erros de SQL automaticamente
- **Segurança** — operações destrutivas exigem confirmação; credenciais nunca expostas ao LLM

### Leve e com desempenho nativo

Construído com Rust + Tauri v2 — sem Electron, sem JRE, sem Chromium embutido. Distribuído como um pequeno binário nativo, com desempenho nativo no macOS, Windows e Linux. O que outros aplicativos precisam de um runtime Java ou de uma licença JetBrains para fazer, o SqlKit faz com um único download.

### Todos os seus bancos de dados, um só aplicativo

O SqlKit suporta **mais de 70 bancos de dados** por meio de quatro estratégias de adaptador:

| Estratégia | Bancos de dados |
|----------|-----------|
| **Nativo** (Rust) | PostgreSQL, MySQL, SQL Server, SQLite |
| **Compatível com protocolo PG** | CockroachDB, Redshift, YugabyteDB, TimescaleDB, QuestDB, Vastbase, YashanDB, KingbaseES, GaussDB, HighGo, UXDB, OpenGauss, GBase8c, Greenplum, EnterpriseDB, CrateDB, Materialize, AlloyDB, CloudSQLPG, FujitsuPG |
| **Compatível com protocolo MySQL** | MariaDB, TiDB, OceanBase, TDSQL, PolarDB, Doris, SelectDB, StarRocks, Databend, GoldenDB, ManticoreSearch, SingleStore, CloudSQLMySQL |
| **Ponte JDBC** | Oracle, DuckDB, Firebird, DB2, H2, Snowflake, TDengine, Derby, Hive, Databricks, Hana, Teradata, Vertica, Exasol, BigQuery, Informix, Kylin, Cassandra, Iris, Access, Dameng, XuguDB, GBase8a |
| **Ponte HTTP** | ClickHouse, Trino, Presto, RQLite, Turso |

### Editor de nível profissional

Baseado no Monaco (motor do VS Code), oferece destaque de sintaxe SQL completo, autocompletar e suporte a múltiplas abas. Navegue pelos esquemas visualmente, inspecione o DDL e pesquise objetos em todos os bancos de dados.

- **Editor Monaco** — edição de SQL de nível VS Code com destaque de sintaxe e autocompletar
- **Múltiplas abas** — trabalhe em várias consultas ao mesmo tempo com gerenciamento de abas
- **Navegador de esquemas** — visualização em árvore de bancos de dados, esquemas, tabelas, colunas e índices
- **Histórico de consultas** — salvo automaticamente, pesquisável e reproduzível
- **Grade de resultados** — paginada, ordenável, com edição em linha e exportação em CSV/JSON/Markdown
- **Visualizador de DDL** — veja as instruções CREATE de qualquer objeto
- **Busca de objetos** — encontre rapidamente tabelas, views e procedures nos esquemas

### Transferência de grandes volumes de dados

Mova dados entre quaisquer engines suportados — PostgreSQL para ClickHouse, Oracle para SQL Server, ou MySQL para BigQuery. Nenhum arquivo intermediário é necessário.

- **Transferência entre engines** — migre dados entre diferentes tipos de banco de dados com mapeamento automático de tipos
- **Importação / Exportação** — formatos CSV, JSON e JSONL
- **Operações em lote** — processe milhões de registros com processamento em lote

### Segurança e conectividade

- **Túnel SSH** — conecte-se via SSH com autenticação por chave ou senha
- **Armazenamento criptografado** — credenciais protegidas pelo chaveiro do sistema operacional (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **SSL/TLS** — conexões criptografadas para bancos de dados suportados
- **Reconexão automática** — gerenciamento de conexões resiliente

### Multiplataforma

- **macOS** (Apple Silicon e Intel) — instalador nativo `.dmg`
- **Windows** — instalador `.exe`
- **Linux** — pacotes `.AppImage` e `.deb`

## Desenvolvimento

### Pré-requisitos

- [Node.js](https://nodejs.org/) >= 18
- [Toolchain do Rust](https://www.rust-lang.org/tools/install)

**Linux (Ubuntu/Debian):**

```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev
```

**macOS:**

```bash
xcode-select --install
```

**Windows:** Instale o [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### Início rápido

```bash
npm install
npm run tauri dev
```

### Scripts

| Comando | Descrição |
|---------|-------------|
| `npm run dev` | Inicia o servidor de desenvolvimento Vite |
| `npm run build` | Compila o frontend para produção |
| `npm run lint:check` | Executa o ESLint |
| `npm run lint:fix` | Corrige automaticamente os problemas de lint |
| `npm test` | Executa os testes do frontend |
| `npm run tauri dev` | Inicia o desenvolvimento com Tauri |
| `npm run tauri build` | Compila o aplicativo Tauri |

### Instruções de build

Consulte o [BUILD.md](BUILD.md) para ver os requisitos de build específicos de cada plataforma e a solução de problemas.

## Stack tecnológica

| Camada | Tecnologia |
|-------|------------|
| Framework | [Tauri v2](https://tauri.app/) (Rust) |
| Frontend | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| UI | [shadcn-vue](https://www.shadcn-vue.com/) + [UnoCSS](https://unocss.dev/) |
| Editor | [Monaco Editor](https://microsoft.github.io/monaco-editor/) |
| Banco de dados | [sqlx](https://github.com/launchbadge/sqlx) + drivers específicos de cada engine |

## Perguntas frequentes

<details>
<summary><strong>O SqlKit é gratuito?</strong></summary>

Sim — a **edição Community** é gratuita e de código aberto sob a licença Apache 2.0. Conexões, edição de SQL, navegação de esquemas, edição de dados, histórico de consultas e exportação de resultados são todos gratuitos, sem limites artificiais.

Recursos avançados — agente de IA (traga seu próprio LLM), servidor MCP, diagramas ER, importação/exportação em massa e transferência de dados entre engines — fazem parte da assinatura paga **GEEKFUN Data Studio Ultimate** (US$ 9,90/mês ou US$ 99/ano, com 7 dias de teste gratuito). Todas as versões lançadas enquanto sua assinatura estiver ativa permanecem suas para sempre, mesmo que você cancele depois.

[Ver preços →](https://www.geekfun.club/pricing)
</details>

<details>
<summary><strong>O SqlKit envia dados para servidores externos?</strong></summary>
Não. O SqlKit não coleta telemetria. O recurso de atualização automática verifica novas versões no GitHub Releases — você pode desativá-lo nas configurações. Suas credenciais e consultas ficam na sua máquina.
</details>

<details>
<summary><strong>Posso usar o SqlKit sem conexão com a internet?</strong></summary>
Sim. O aplicativo desktop funciona totalmente offline. Os recursos de IA exigem acesso à rede ao endpoint do modelo (ou um modelo local via Ollama).
</details>

<details>
<summary><strong>Qual é a diferença entre o SqlKit e o DBeaver / TablePlus / DataGrip?</strong></summary>
O SqlKit é um aplicativo Tauri nativo (Rust) — sem Java JRE, sem a sobrecarga do Electron. Inclui IA nativamente (não como plugin), suporta mais de 70 bancos de dados e roda no macOS, Windows e Linux a partir de um único binário. Privacidade em primeiro lugar, com armazenamento local e criptografado das credenciais.
</details>

<details>
<summary><strong>Quais bancos de dados são suportados?</strong></summary>
PostgreSQL, MySQL, Oracle, SQL Server, SQLite, DuckDB, ClickHouse, Firebird, MariaDB, CockroachDB, TiDB, OceanBase, Snowflake, DB2, H2, Trino, Greenplum, CrateDB, SingleStore e mais de 60 outros. Consulte a seção <a href="#multi-database-support">Suporte a múltiplos bancos de dados</a> para ver a lista completa.
</details>

<details>
<summary><strong>Como reporto um bug ou solicito um recurso?</strong></summary>
Abra uma issue no <a href="https://github.com/geek-fun/sqlkit/issues">GitHub Issues</a>.
</details>

## Comunidade

<p align="center">
  <img src="docs/images/wechat_official.png" width="300" alt="Conta oficial do WeChat">
</p>

<p align="center">
  <a href="https://x.com/geekfun_club"><img src="https://img.shields.io/badge/X-Follow-000000?logo=x&logoColor=white" alt="X / Twitter" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://www.youtube.com/@geekfun-club"><img src="https://img.shields.io/badge/YouTube-Subscribe-FF0000?logo=youtube&logoColor=white" alt="YouTube" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://github.com/geek-fun"><img src="https://img.shields.io/badge/GitHub-Follow-181717?logo=github&logoColor=white" alt="GitHub" /></a>
</p>

## Patrocínio

<p align="center">
  <img src="docs/images/wechat_ponsor.jpg" width="140" alt="QR Code de patrocínio do WeChat">
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <a href="https://github.com/sponsors/geek-fun"><img src="https://img.shields.io/badge/GitHub_Sponsors-%E2%9D%A4_Support-EA4AAA?logo=githubsponsors&logoColor=white" alt="GitHub Sponsors" /></a>
</p>

## Licença

[Apache 2.0](LICENSE) © GEEKFUN
