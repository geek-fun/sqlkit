# SQLKit 多数据库压测套件

同一套电商业务域（用户 / 商户 / 分类树 / 商品 / 订单 / 明细 / 支付 / 评价 / 优惠券 /
库存预留 / 审计日志）覆盖 **PostgreSQL、MySQL、SQL Server、Oracle** 四个数据库，
用来把 SQLKit 的元数据面板和查询编辑器在复杂结构上跑一遍 —— 不是单元测试，
而是**手工验证用的压测脚本 + 期望行为清单**。

四个引擎的目录结构完全对称，便于横向对比同一功能在各家的落地方式：

```
docs/sql-test/
├── postgres/   01_schema.sql  02_seed.sql  03_complex_queries.sql  README.md
├── mysql/      01_schema.sql  02_seed.sql  03_complex_queries.sql  README.md
├── sqlserver/  01_schema.sql  02_seed.sql  03_complex_queries.sql  README.md
└── oracle/     01_schema.sql  02_seed.sql  03_complex_queries.sql  README.md
```

三个文件的固定分工：

| 文件 | 职责 |
|------|------|
| `01_schema.sql` | 清场 + 建对象全景：多 schema/库、全类型覆盖、主外键、各类索引、分区、视图、物化视图/索引视图、函数、过程、触发器、序列/生成列。**幂等**，可反复执行。 |
| `02_seed.sql` | 确定性灌数（400 用户 / 10 商户 / 60 商品 / 500 订单 / ~1200 明细 + 支付/评价/券），结尾输出计数卡片。**幂等**。 |
| `03_complex_queries.sql` | 按字母分段的查询演练场（递归 CTE、窗口函数、聚合集、JSON、UPSERT、事务与保存点、执行计划、分区裁剪等），**逐段执行**验证编辑器与结果网格。 |
| `README.md` | 该引擎的目标实例与凭据、面板检查清单、与 PostgreSQL 的方言差异、已知限制。 |

## 覆盖的 SQLKit 面板

四个套件都刻意准备了对应用例，用于验证：

- **Schemas** —— PG 的三 schema、MySQL 的三 database、SQL Server 的多 schema、Oracle 的单用户 schema
- **Tables / Columns / Table Info** —— 复合主键、生成列/虚拟列、分区表、注释元数据
- **Indexes** —— 唯一/复合/部分（或等效）/全文/空间/函数/位图等各家专属索引类型
- **Foreign Keys** —— 命名与默认命名约束、复合外键、`CASCADE / SET NULL / RESTRICT`
- **Triggers / Views / Procedures / Functions** —— 行级与语句级触发器、可更新视图、物化/索引视图、重载函数
- **DDL 预览 / Drop / Rename** —— 表、视图、物化视图、分区表
- **Query editor** —— 逐段执行 `03_*`，观察结果网格、行数、耗时与报错路径

## 四个引擎的测试实例

四个库都跑在同一套 docker ssh-tunnel 测试栈里（compose 文件在
`event-search/docker-compose-sqlkit-ssh-tunnel.yml`），**只在内部网络**，
经堡垒机 `localhost:2223` 转发访问（堡垒机 `tester` / `testpass`）。

| 引擎 | 堡垒机内侧地址 | 凭据 | 套件使用的对象 |
|---|---|---|---|
| PostgreSQL 17 | `pg-test:5432` | `tester` / `testpass` / `testdb` | 库 `testdb`，schema `store` / `analytics` / `billing` |
| MySQL 8.4 | `mysql-test:3306` | `root` / `testpass`（`tester` 同密码） | 库 `sqlkit-demo` + `sqlkit-analytics` + `sqlkit-billing` |
| SQL Server 2022 | `sqlserver-test:1433` | `sa` / `TestPass!2024` | 库 `testdb` |
| Oracle 23ai Free | `oracle-test:1521` | `sqlkit` / `testpass` / 服务名 `FREEPDB1` | 用户 schema `SQLKIT` |

> MySQL 套件单独建库是因为栈内 `tester` 只被授权 `testdb.*`；若坚持用 `tester`，
> 先 `GRANT ALL PRIVILEGES ON \`sqlkit-demo\`.* TO 'tester'@'%';`。
> 详见 [mysql/README.md](mysql/README.md)。

## 为什么不能只写一份通用 SQL

四个引擎在「同一件事」上的语法差异很大，套件刻意保留这些差异而不是回避，
这样才真正压到 SQLKit 的方言分支。最典型的几处：

| 能力 | PostgreSQL | MySQL | SQL Server | Oracle |
|---|---|---|---|---|
| 生成序列 | `generate_series()` | 递归 CTE | 递归 CTE / 数字表 | `CONNECT BY LEVEL` |
| UPSERT | `ON CONFLICT ... DO UPDATE` | `ON DUPLICATE KEY UPDATE` | `MERGE` | `MERGE`（**不支持** `ON DUPLICATE KEY UPDATE`，实测 ORA-03048） |
| 分区表 + 外键 | 支持 | **不支持**（分区表禁外键） | 支持（分区函数 + 方案） | 支持（含 INTERVAL 分区） |
| 可延迟约束 | `DEFERRABLE` | 不支持 | 不支持 | 支持 |
| 条件聚合 | `FILTER (WHERE ...)` | 无，用 `CASE WHEN` | 有 | 有 |
| 物化视图 | `MATERIALIZED VIEW` | 无 | 索引视图（`SCHEMABINDING`） | `MATERIALIZED VIEW` |
| 部分索引 | `WHERE` 部分索引 | 无 | 筛选索引（filtered index） | 无 |
| 数组/网络/范围类型 | 原生 `text[]` `inet` `range` | JSON / VARBINARY | JSON / 无 | JSON / 无 |

各引擎 README 里有完整的逐项对照表。

## 使用建议

1. 先在 SQLKit 里按对应 README 的凭据建立连接（经 SSH 隧道）。
2. 顺序执行 `01_schema.sql` → `02_seed.sql`，确认结尾计数卡片符合 README 预期。
3. 逐段执行 `03_complex_queries.sql`，对照 README 的面板检查清单过一遍 UI。
4. 想复现干净状态时重跑 `01` + `02` 即可（全部幂等）。

## 自动化回归

现在也可以通过 SQLKit 的真实 Rust 适配器和 Java JDBC 桥自动执行这套用例。
运行入口、覆盖范围和限制见 [AUTOMATED_TESTING.md](AUTOMATED_TESTING.md)。
