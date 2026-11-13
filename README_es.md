<div align="center">

<img src="app-icon.png" width="120" alt="SqlKit Logo"/>

# SqlKit

**Cliente GUI de bases de datos SQL agéntico y multiplataforma — tu agente de bases de datos para más de 70 bases de datos.**

**Privacidad primero. Tus datos, tus llaves. Código abierto.**

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

[Descargar](https://www.geekfun.club/download) · [Documentación](docs/DISTRIBUTION.md) · [Sitio web](https://www.geekfun.club) · [Versiones](https://github.com/geek-fun/sqlkit/releases)

[English](README.md) · [简体中文](README_zh.md) · [Português (BR)](README_pt-BR.md) · [日本語](README_ja.md) · [한국어](README_ko.md) · Español

</div>

---

SqlKit es un **cliente de bases de datos agéntico** — no solo ejecuta SQL, sino que entiende tus bases de datos y actúa en tu nombre. Describe lo que necesitas en lenguaje natural y el agente escribe consultas, inspecciona esquemas, optimiza SQL y devuelve resultados. Construido con Tauri (Rust), no con Electron, reemplaza a clientes pesados como DBeaver, Navicat y DataGrip con una única app de escritorio nativa.

<p align="center">
  <img src="docs/images/sqlkit-client-ui.png" width="800" alt="Interfaz del cliente SqlKit"/>
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

## Instalación

<a href="https://www.geekfun.club/download">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?logo=download&logoColor=white">
    <img src="https://img.shields.io/badge/Download-macOS_|_Windows_|_Linux-orange?logo=download&logoColor=white" alt="Descargar">
  </picture>
</a>
&nbsp;
<a href="https://github.com/geek-fun/sqlkit/releases">
  <img src="https://img.shields.io/badge/Releases-GitHub-lightgrey?logo=github" alt="Versiones">
</a>
&nbsp;
<a href="https://www.geekfun.club/products/sqlkit/">
  <img src="https://img.shields.io/badge/Website-geekfun.club-blue?logo=google-chrome&logoColor=white" alt="Sitio web">
</a>

## Características

### Estudio de Datos Agéntico

Describe lo que necesitas en lenguaje natural — el agente lee tu esquema, escribe consultas, optimiza el SQL lento, explica los planes de ejecución visualmente y corrige errores al instante. Compatible con OpenAI, Anthropic, DeepSeek y Ollama. Trae tu propia clave.

- **Generación de consultas con IA** — de lenguaje natural a SQL con contexto consciente del esquema
- **Optimización de SQL** — reescribe consultas lentas y visualiza planes de ejecución
- **Corrección de errores** — el agente diagnostica y corrige errores de SQL automáticamente
- **Seguridad** — las operaciones destructivas requieren confirmación; las credenciales nunca se exponen al LLM

### Ligero y con rendimiento nativo

Construido con Rust + Tauri v2 — sin Electron, sin JRE, sin Chromium incluido. Se distribuye como un pequeño binario nativo, con rendimiento nativo en macOS, Windows y Linux. Lo que otras apps necesitan un runtime de Java o una licencia de JetBrains para hacer, SqlKit lo hace con una sola descarga.

### Todas tus bases de datos, una sola app

SqlKit es compatible con **más de 70 bases de datos** mediante cuatro estrategias de adaptador:

| Estrategia | Bases de datos |
|----------|-----------|
| **Nativo** (Rust) | PostgreSQL, MySQL, SQL Server, SQLite |
| **Compatible con protocolo PG** | CockroachDB, Redshift, YugabyteDB, TimescaleDB, QuestDB, Vastbase, YashanDB, KingbaseES, GaussDB, HighGo, UXDB, OpenGauss, GBase8c, Greenplum, EnterpriseDB, CrateDB, Materialize, AlloyDB, CloudSQLPG, FujitsuPG |
| **Compatible con protocolo MySQL** | MariaDB, TiDB, OceanBase, TDSQL, PolarDB, Doris, SelectDB, StarRocks, Databend, GoldenDB, ManticoreSearch, SingleStore, CloudSQLMySQL |
| **Puente JDBC** | Oracle, DuckDB, Firebird, DB2, H2, Snowflake, TDengine, Derby, Hive, Databricks, Hana, Teradata, Vertica, Exasol, BigQuery, Informix, Kylin, Cassandra, Iris, Access, Dameng, XuguDB, GBase8a |
| **Puente HTTP** | ClickHouse, Trino, Presto, RQLite, Turso |

### Editor de calidad profesional

Impulsado por Monaco (el motor de VS Code), ofrece resaltado de sintaxis SQL completo, autocompletado y soporte para múltiples pestañas. Explora esquemas visualmente, inspecciona el DDL y busca objetos en todas las bases de datos.

- **Editor Monaco** — edición de SQL de nivel VS Code con resaltado de sintaxis y autocompletado
- **Múltiples pestañas** — trabaja en varias consultas a la vez con gestión de pestañas
- **Explorador de esquemas** — vista de árbol de bases de datos, esquemas, tablas, columnas e índices
- **Historial de consultas** — se guarda automáticamente, se puede buscar y repetir
- **Cuadrícula de resultados** — paginada, ordenable, con edición en línea y exportación a CSV/JSON/Markdown
- **Visor de DDL** — consulta las sentencias CREATE de cualquier objeto
- **Búsqueda de objetos** — encuentra rápidamente tablas, vistas y procedimientos en los esquemas

### Transferencia de grandes volúmenes de datos

Mueve datos entre cualquier motor compatible — de PostgreSQL a ClickHouse, de Oracle a SQL Server o de MySQL a BigQuery. Sin archivos intermedios.

- **Transferencia entre motores** — migra datos entre diferentes tipos de bases de datos con mapeo automático de tipos
- **Importación / Exportación** — formatos CSV, JSON y JSONL
- **Operaciones masivas** — procesa millones de registros con procesamiento por lotes

### Seguridad y conectividad

- **Túnel SSH** — conéctate a través de SSH con autenticación por clave o contraseña
- **Almacenamiento cifrado** — credenciales protegidas por el llavero del sistema operativo (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **SSL/TLS** — conexiones cifradas a las bases de datos compatibles
- **Reconexión automática** — gestión de conexiones resiliente

### Multiplataforma

- **macOS** (Apple Silicon e Intel) — instalador nativo `.dmg`
- **Windows** — instalador `.exe`
- **Linux** — paquetes `.AppImage` y `.deb`

## Desarrollo

### Requisitos previos

- [Node.js](https://nodejs.org/) >= 18
- [Toolchain de Rust](https://www.rust-lang.org/tools/install)

**Linux (Ubuntu/Debian):**

```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev
```

**macOS:**

```bash
xcode-select --install
```

**Windows:** Instala [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### Inicio rápido

```bash
npm install
npm run tauri dev
```

### Scripts

| Comando | Descripción |
|---------|-------------|
| `npm run dev` | Inicia el servidor de desarrollo de Vite |
| `npm run build` | Compila el frontend para producción |
| `npm run lint:check` | Ejecuta ESLint |
| `npm run lint:fix` | Corrige automáticamente los problemas de lint |
| `npm test` | Ejecuta las pruebas del frontend |
| `npm run tauri dev` | Inicia el desarrollo con Tauri |
| `npm run tauri build` | Compila la aplicación Tauri |

### Instrucciones de compilación

Consulta [BUILD.md](BUILD.md) para ver los requisitos de compilación específicos de cada plataforma y la solución de problemas.

## Stack tecnológico

| Capa | Tecnología |
|-------|------------|
| Framework | [Tauri v2](https://tauri.app/) (Rust) |
| Frontend | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| UI | [shadcn-vue](https://www.shadcn-vue.com/) + [UnoCSS](https://unocss.dev/) |
| Editor | [Monaco Editor](https://microsoft.github.io/monaco-editor/) |
| Base de datos | [sqlx](https://github.com/launchbadge/sqlx) + drivers específicos de cada motor |

## Preguntas frecuentes

<details>
<summary><strong>¿Es SqlKit gratuito?</strong></summary>

Sí — la **edición Community** es gratuita y de código abierto bajo la licencia Apache 2.0. Las conexiones, la edición de SQL, la exploración de esquemas, la edición de datos, el historial de consultas y la exportación de resultados son gratuitos, sin límites artificiales.

Las capacidades avanzadas — agente de IA (con tu propio LLM), servidor MCP, diagramas ER, importación/exportación masiva y transferencia de datos entre motores — forman parte de la suscripción de pago **GEEKFUN Data Studio Ultimate** (9,9 USD/mes o 99 USD/año, con 7 días de prueba gratuita). Todas las versiones publicadas mientras tu suscripción esté activa quedan tuyas para siempre, incluso si cancelas más adelante.

[Ver precios →](https://www.geekfun.club/pricing)
</details>

<details>
<summary><strong>¿SqlKit envía datos a servidores externos?</strong></summary>
No. SqlKit no recopila telemetría. La función de actualización automática consulta GitHub Releases en busca de nuevas versiones — puedes desactivarla en la configuración. Tus credenciales y consultas permanecen en tu equipo.
</details>

<details>
<summary><strong>¿Puedo usar SqlKit sin conexión a internet?</strong></summary>
Sí. La app de escritorio funciona completamente sin conexión. Las funciones de IA requieren acceso de red al endpoint del modelo (o un modelo local mediante Ollama).
</details>

<details>
<summary><strong>¿En qué se diferencia SqlKit de DBeaver / TablePlus / DataGrip?</strong></summary>
SqlKit es una app nativa de Tauri (Rust) — sin Java JRE, sin la sobrecarga de Electron. Incluye IA de forma nativa (no como plugin), es compatible con más de 70 bases de datos y funciona en macOS, Windows y Linux desde un único binario. Privacidad primero, con almacenamiento local cifrado de credenciales.
</details>

<details>
<summary><strong>¿Qué bases de datos son compatibles?</strong></summary>
PostgreSQL, MySQL, Oracle, SQL Server, SQLite, DuckDB, ClickHouse, Firebird, MariaDB, CockroachDB, TiDB, OceanBase, Snowflake, DB2, H2, Trino, Greenplum, CrateDB, SingleStore y más de 60 adicionales. Consulta la sección <a href="#multi-database-support">Soporte de múltiples bases de datos</a> para ver la lista completa.
</details>

<details>
<summary><strong>¿Cómo puedo reportar un error o solicitar una nueva función?</strong></summary>
Abre un issue en <a href="https://github.com/geek-fun/sqlkit/issues">GitHub Issues</a>.
</details>

## Comunidad

<p align="center">
  <img src="docs/images/wechat_official.png" width="300" alt="Cuenta oficial de WeChat">
</p>

<p align="center">
  <a href="https://x.com/geekfun_club"><img src="https://img.shields.io/badge/X-Follow-000000?logo=x&logoColor=white" alt="X / Twitter" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://www.youtube.com/@geekfun-club"><img src="https://img.shields.io/badge/YouTube-Subscribe-FF0000?logo=youtube&logoColor=white" alt="YouTube" /></a>
  &nbsp;&nbsp;&nbsp;
  <a href="https://github.com/geek-fun"><img src="https://img.shields.io/badge/GitHub-Follow-181717?logo=github&logoColor=white" alt="GitHub" /></a>
</p>

## Patrocinio

<p align="center">
  <img src="docs/images/wechat_ponsor.jpg" width="140" alt="Código QR de patrocinio de WeChat">
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <a href="https://github.com/sponsors/geek-fun"><img src="https://img.shields.io/badge/GitHub_Sponsors-%E2%9D%A4_Support-EA4AAA?logo=githubsponsors&logoColor=white" alt="GitHub Sponsors" /></a>
</p>

## Licencia

[Apache 2.0](LICENSE) © GEEKFUN
