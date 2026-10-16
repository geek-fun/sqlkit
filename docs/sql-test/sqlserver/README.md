# SQLKit SQL Server 压测套件

同一套电商业务域（用户 / 商户 / 分类树 / 商品 / 订单 / 明细 / 支付 / 评价 / 优惠券 /
库存预留 / 审计日志），从 [postgres/](../postgres/) 套件逐段翻译成 **SQL Server 2022
惯用写法**，用来把 SQLKit 的元数据面板和查询编辑器在复杂结构上跑一遍 ——
不是单元测试，而是**手工验证用的压测脚本 + 期望行为清单**。

> 同域套件还有 [PostgreSQL](../postgres/README.md)、[MySQL](../mysql/README.md)、
> [Oracle](../oracle/README.md)；跨引擎总览见[套件索引](../README.md)。

**目标实例:** 与 PostgreSQL 套件共用 docker ssh-tunnel 测试栈 ——
`sqlserver-test:1433/testdb`（`sa` / `TestPass!2024`），经堡垒机 `localhost:2223`
（堡垒机账号 `tester` / `testpass`）转发访问。

按顺序执行三个文件：

| 文件 | 作用 |
|------|------|
| `01_schema.sql` | 清场 + 建对象全景：3 个 schema（`sqlkit` / `sqlkit_analytics` / `sqlkit_billing`）、别名类型 + 表类型、1 个序列与 1 组分区函数/分区方案、16 张表（分区订单、复合主键/外键、持久化与非持久化计算列、`SPARSE`/`ROWGUIDCOL`、系统版本时态表）、8 个视图（含 `WITH CHECK OPTION`、索引视图）、5 个触发器（AFTER / INSTEAD OF / 重叠兜底）、5 个函数（标量 + 3 个内联 TVF + 多语句 TVF，含跨 schema 同名）、3 个存储过程（含 OUTPUT 参数）。**幂等**，可反复执行。 |
| `02_seed.sql` | 用集合操作确定性灌数（递归 CTE 数字表 + `CROSS APPLY` + `CHOOSE` + `ROW_NUMBER`，无 `WHILE` 循环）：400 用户、10 商户、10 分类、60 商品、500 订单（2026-01 ~ 2026-03 三个分区）、1,250 行明细、417 支付、168 评价、24 张券、2 条预留、417 张发票、时态历史行与触发审计行。显式事务 + `TRY/CATCH/THROW`，结尾输出**单一结果集计数卡片**。**幂等**。 |
| `03_complex_queries.sql` | A..T 共 20 段的查询演练场（递归 CTE、窗口函数、`CROSS APPLY`、`GROUPING SETS`/`CUBE`、`PIVOT`/`UNPIVOT`、`MERGE`、事务与保存点、`TRY/CATCH`、执行计划、JSON、`OUTPUT`、`OFFSET/FETCH`、分区路由）。**不含 `GO`**，每段都是自包含的单一批次。 |
| `README.md` | 本文：目标实例与凭据、段目录、面板检查清单、与 PostgreSQL 的方言差异、已知限制。 |

## `03_complex_queries.sql` 段目录

| 段 | 主题 | 主要看点 |
|----|------|----------|
| A | 多 CTE 商业报表 | 周复购 + `PERCENT_RANK()` 客单价分位、`DATETRUNC`（2022 新函数） |
| B | 递归 CTE 分类树 | 自引用树 + `OPTION (MAXRECURSION 100)` |
| C | `CROSS APPLY` Top-N | T-SQL 的 LATERAL 等价物 + `RANK()` |
| D | 窗口函数全家桶 | `ROWS` 滑动平均、`LAG`/`LEAD`/`FIRST_VALUE`、`ROW_NUMBER` |
| E | 聚合集 | `GROUPING SETS` + `CUBE` + `ROLLUP` 三种小计粒度，用 `SUM(CASE WHEN ...)` 替代 PG 的 `FILTER` |
| F | JSON / XML 类型演练 | `ISJSON`/`JSON_VALUE`/`OPENJSON`、XML `.value()`、JSON 标签数组 |
| G | `MERGE` upsert | 替代 `ON CONFLICT DO NOTHING` / `DO UPDATE`，`OUTPUT ... INTO` 收 $action |
| H | `MERGE` 库存对账 | 临时表 + 事务 + `OUTPUT ... INTO` + 清理复原 |
| I | **事务 + 保存点（唯一故意报错段）** | `SAVE TRANSACTION` / `ROLLBACK TRANSACTION <保存点>` 救援被触发器拒绝的写入 |
| J | `TRY...CATCH` + `THROW` | 替代 PG 的 `DO` 匿名块，输出 `ERROR_*()` 错误上下文 |
| K | `EXEC` 存储过程 | 输入 + OUTPUT 参数回传，段尾复位保证可重复执行 |
| L | 集合操作 | `FULL OUTER JOIN` / `INTERSECT` / `EXCEPT`，`IS DISTINCT FROM` 的手写替代 |
| M | `PIVOT` / `UNPIVOT` | 行列互转，外加 `DISTINCT ON` 的 `ROW_NUMBER` 替代写法 |
| N | JSON 聚合 | `STRING_AGG ... WITHIN GROUP`（2022 起支持排序）+ `FOR JSON PATH` |
| O | 执行计划 | `SET STATISTICS XML ON/OFF`，含分区裁剪与索引命中对比 |
| P | `UPDATE ... FROM` / `DELETE ... FROM` | T-SQL 的 `OUTPUT` 子句替代 `RETURNING` |
| Q | 约束立即校验 | PG 可延迟约束在 T-SQL 不存在，`TRY/CATCH` 捕获撞车后改用三步交换法 |
| R | 函数三形态 | 标量 UDF / 内联 TVF / 多语句 TVF + 跨 schema 同名函数 |
| S | 窗口框架 `ROWS` vs `RANGE` | T-SQL 的 `RANGE` 不支持 `INTERVAL` 偏移 → `ROWS` / `OUTER APPLY` 等价实现 |
| T | 分区路由 + 分页 | `$PARTITION.pf_orders_by_date()` 验证落位并回滚，`OFFSET/FETCH` 三种分页 |

## 覆盖的 SQLKit 面板

| 面板 | 套件里刻意准备的用例 |
|------|----------------------|
| **Schemas** | `sqlkit` / `sqlkit_analytics` / `sqlkit_billing` 三个 schema，且都带 `MS_Description` 扩展属性 |
| **Tables / Columns / Table Info** | 16 张表；复合主键（`products`、`orders`、`order_items`）；`IDENTITY`；持久化计算列（`users.email_domain`、`products.line_margin`、`order_items.line_total`）与非持久化计算列（`users.display_name`）；`SPARSE` 列（`phone`、`geo_lat`/`geo_lon`、`weight_g`、`span_x1/x2`）；`ROWGUIDCOL` + `NEWSEQUENTIALID()`；`XML`、`VARBINARY(MAX)`、`DATETIMEOFFSET`、别名类型列；分区父表 `orders`；时态表 `price_book` 与自动生成的历史表 `price_book_history`；`GROUPING`/注释元数据 |
| **DDL 预览 / Drop / Rename** | 对普通表、分区表、时态表、视图、索引视图、过程、函数、触发器、序列、类型都能拿到完整 DDL |
| **Indexes** | 聚集/非聚集、唯一索引、**过滤索引**（`ix_users_active_email`、`ix_products_active_price`、`ix_reviews_active`）、**包含列 INCLUDE**、分区对齐索引（`ON ps_orders_by_date(order_date)`）、计算列索引、降序索引、索引视图上的唯一聚集索引 + 非聚集索引 |
| **Foreign Keys** | 复合外键（`order_items`/`payments`/`invoices` → `orders(id, order_date)`）；显式命名与**默认命名**（`reviews.product_id`、`coupons.claimed_by`、`merchants` 的第二个唯一约束）；`ON DELETE CASCADE`（`order_items`、`invoices`）、`ON DELETE SET NULL`（`payments`、`reviews`）、`ON UPDATE CASCADE`（`products → merchants`）、`ON DELETE NO ACTION` |
| **Triggers** | `AFTER INSERT, UPDATE, DELETE`（`orders` 审计，按 `inserted`/`deleted` 区分三种操作）、`AFTER UPDATE`（`products`）、`AFTER INSERT`（`reviews`）、`AFTER INSERT/UPDATE` 时段互斥兜底（`reservations`）、视图上的 `INSTEAD OF DELETE`（软删除 + 审计） |
| **Views** | 普通视图、`WITH CHECK OPTION` 视图、带聚合的报表视图、**索引视图**（`WITH SCHEMABINDING` + 唯一聚集索引，替代 PG 的物化视图）、跨 schema 视图、`UNION ALL` 视图 |
| **Procedures** | 3 个过程：2 个带 OUTPUT 参数（`sqlkit.usp_claim_coupon`、`sqlkit.usp_void_order`）+ 1 个内部用 `MERGE` 的跨 schema 过程（`sqlkit_billing.usp_refresh_metrics`） |
| **Functions** | 1 个标量 UDF、3 个内联 TVF、1 个多语句 TVF；`sqlkit.get_product_rank` 与 `sqlkit_analytics.get_product_rank` **跨 schema 同名**（schema 作用域校验） |
| **Sequences** | `sqlkit.coupon_code_seq`（`START 100000 / INCREMENT 1 / CYCLE / CACHE 50`），并被 `coupons.code` 的 DEFAULT 约束引用 |
| **Types** | 2 个别名类型（`sqlkit.cny_price`、`sqlkit.order_status_t`，替代 PG 的 DOMAIN/ENUM）+ 1 个表类型（`sqlkit.tt_money_amount`，替代 PG 的复合类型） |
| **Partitions** | 分区函数 `pf_orders_by_date`（`RANGE RIGHT`，边界 2026-02-01/03-01/04-01）+ 分区方案 `ps_orders_by_date`（单文件库 `ALL TO ([PRIMARY])`），`orders` 的表与索引全部按 `order_date` 对齐 |
| **Query editor** | 逐段执行 `03_*` 的 A..T，观察结果网格、行数、耗时、多结果集与报错路径 |

## 连接参数

| 项 | 值 |
|----|----|
| 主机（SQLKit 连接对话框里填**内部名**） | `sqlserver-test` |
| 端口 | `1433` |
| 登录 | `sa` / `TestPass!2024` |
| 数据库 | `testdb` |
| SSH 堡垒机 Host / Port | `localhost` / `2223` |
| SSH 堡垒机账号 | `tester` / `testpass`（或三把私钥之一） |
| 服务器版本 | SQL Server 2022 (RTM-CU24) 16.0.4245.2 Developer Edition，兼容级别 160 |
| Docker 容器 | `sqlkit-ssh-sqlserver` |

`testdb` 不存在时 `01_schema.sql` 会自动建库（脚本用 `IF DB_ID(N'testdb') IS NULL
CREATE DATABASE testdb` + `USE testdb` 兜底，所以用 `sqlcmd` 直接连 master 执行也没问题）。
`sa` 是 sysadmin，脚本没有用到任何 sysadmin 之外的能力。

### 命令行执行方式（本套件的验证方式）

```bash
# 1) 建对象（幂等：会先 DROP 再 CREATE）
docker exec -i sqlkit-ssh-sqlserver /opt/mssql-tools18/bin/sqlcmd \
  -C -S localhost -U sa -P 'TestPass!2024' -b < docs/sql-test/sqlserver/01_schema.sql

# 2) 灌数，结尾输出计数卡片
docker exec -i sqlkit-ssh-sqlserver /opt/mssql-tools18/bin/sqlcmd \
  -C -S localhost -U sa -P 'TestPass!2024' -b < docs/sql-test/sqlserver/02_seed.sql

# 3) 逐段跑查询演练场（抽出 A..T 中的一段单独执行；-d testdb 指定库）
docker exec -i sqlkit-ssh-sqlserver /opt/mssql-tools18/bin/sqlcmd \
  -C -S localhost -U sa -P 'TestPass!2024' -d testdb -b -i /dev/stdin < section_A.sql
```

`02_seed.sql` 结束时的计数卡片应长这样（18 行，按名称排序）：

```
what|cnt
analytics_metrics|3
audit_log|1168
billing_metrics|3
categories|10
coupons|24
coupons_claimed|6
invoices|417
merchants|10
order_items|1250
orders|500
payments|417
price_book|3
price_book_history|2
products|60
reservations|2
reviews|168
users|400
```

> `03` 的 P 段会删掉退款订单的评价，K/T 段会临时改状态再复位，所以**想拿到干净状态
> 重跑 `01` → `02` 即可**（三个文件都幂等）。

## 需要人工分批执行的地方（重要）

SQLKit 的 SQL Server 适配器把编辑器里的内容**一次性发给服务器**，而 `GO` 不是 T-SQL
语句（服务器会直接报语法错误），因此：

- **`01_schema.sql` / `02_seed.sql` 含 `GO`**：请用 `sqlcmd` 整体执行，或在 SQLKit 里
  **按 `GO` 逐批执行**。必须单独的批次的语句包括：`CREATE DATABASE` + `USE`、
  `CREATE SCHEMA`、`CREATE TYPE`、`CREATE SEQUENCE`、`CREATE PARTITION FUNCTION/SCHEME`、
  所有 `CREATE VIEW` / `FUNCTION` / `PROCEDURE` / `TRIGGER`（T-SQL 要求它们是批次的
  第一条语句）、`CREATE INDEX`、`ALTER TABLE ... SET (SYSTEM_VERSIONING = ...)`、
  以及第 1 节的 `SET` 选项块。
- **`03_complex_queries.sql` 不含 `GO`**：正确用法是**选中 A..T 中的一段**执行，不要整
  文件一次性运行 —— 多段共享同名局部变量（`@upd`、`@del`、`@stock_log`、`@claimed` …）
  和同名临时表，拼在一个批次里会重复声明而报错。
- **I 段是唯一故意报错段**：`sqlcmd -b` 会在该错误处直接退出（退出码 1，**这是预期
  现象**）。去掉 `-b`（或只在 SQLKit 编辑器里执行该段）才能看到后面保存点救援成功的
  结果网格。任何**其他**段的报错都说明环境或脚本需要排查。

## 与 PostgreSQL 的方言差异

本套件刻意保留（而不是回避）以下差异，这样才真正压到 SQLKit 的 SQL Server 分支：

| 能力 | PostgreSQL 套件 | 本 SQL Server 套件的落地方式 |
|------|-----------------|------------------------------|
| 批量生成行 | `generate_series()` | 递归 CTE 数字表（`WITH seq(n) AS (... UNION ALL ...)`）+ `ROW_NUMBER()`，`OPTION (MAXRECURSION 0)` |
| 枚举 / 域 / 复合类型 | `CREATE TYPE ... AS ENUM`、`CREATE DOMAIN`、`CREATE TYPE AS (...)` | 别名类型（`CREATE TYPE ... FROM ...`）+ `CHECK` 约束 + 表类型（TVP） |
| 序列取值 | `nextval('seq')` | `NEXT VALUE FOR seq`（可用在 `DEFAULT` 约束里） |
| 自增 / 生成列 | `GENERATED ALWAYS AS IDENTITY`、`GENERATED ALWAYS AS (...) STORED` | `IDENTITY(1,1)` + `SET IDENTITY_INSERT` 灌显式 id；计算列（`PERSISTED` / 非持久化） |
| 重置自增 | `TRUNCATE ... RESTART IDENTITY CASCADE` | `TRUNCATE` 没有 `CASCADE` → 按依赖顺序 `DELETE` + `DBCC CHECKIDENT(..., RESEED, n)` |
| 分区 | `PARTITION BY RANGE (order_date)` + `PARTITION OF` | 分区函数（`RANGE RIGHT`）+ 分区方案（`ALL TO ([PRIMARY])`），表与索引用 `ON ps_...(order_date)` 对齐；分区号用 `$PARTITION.pf_...(col)` 查 |
| 时段互斥 | `EXCLUDE USING gist (resource_id WITH =, valid WITH &&)` | T-SQL 无 `EXCLUDE` → `AFTER INSERT/UPDATE` 触发器 + `RAISERROR` 兜底（I 段故意触发它） |
| 物化视图 | `CREATE MATERIALIZED VIEW` + `REFRESH` | 索引视图（`WITH SCHEMABINDING` + 唯一聚集索引），由基表 DML **自动维护**，无需刷新 |
| 部分索引 / 覆盖索引 | `WHERE` 部分索引、`INCLUDE` | 过滤索引（filtered index）+ `INCLUDE` 列 |
| 函数重载 | 同 schema 重载 `get_product_rank(int)` / `(bigint)` | T-SQL **不支持重载** → 同 schema 内换名，跨 schema 用同名（`sqlkit` vs `sqlkit_analytics`） |
| 触发器粒度 | 行级 `FOR EACH ROW` + 语句级 `FOR EACH STATEMENT` | T-SQL 只有语句级（集合）触发器，靠 `inserted` / `deleted` 处理整批行 |
| 审计快照 | `to_jsonb(NEW)` | `(SELECT ... FOR JSON PATH, WITHOUT_ARRAY_WRAPPER)` 子查询写进 `NVARCHAR(MAX)`（配 `ISJSON` CHECK） |
| 条件聚合 | `count(*) FILTER (WHERE ...)` | `SUM(CASE WHEN ... THEN 1 ELSE 0 END)` |
| `DISTINCT ON` | `SELECT DISTINCT ON (category_id) ...` | `ROW_NUMBER() OVER (PARTITION BY ... ORDER BY ...) = 1`（M 段） |
| 窗口框架 `RANGE` | `RANGE BETWEEN INTERVAL '7 days' PRECEDING AND CURRENT ROW` | T-SQL 的 `RANGE` 只支持 `UNBOUNDED PRECEDING` / `CURRENT ROW` → 用 `ROWS BETWEEN 6 PRECEDING` 或 `OUTER APPLY`（S 段） |
| 横向连接 | `JOIN LATERAL (... LIMIT n)` | `CROSS APPLY (SELECT TOP (n) ... ORDER BY ...)`（或 `ORDER BY ... OFFSET 0 ROWS FETCH NEXT n ROWS ONLY`） |
| UPSERT | `INSERT ... ON CONFLICT DO UPDATE ... RETURNING` | `MERGE ... WHEN MATCHED / WHEN NOT MATCHED` + `OUTPUT`（等价 `greatest()` 用 `WHEN MATCHED AND t.x < s.x`） |
| 返回受影响行 | `RETURNING` / `UPDATE ... FROM ... RETURNING` | `OUTPUT inserted.* / deleted.* INTO @表变量` |
| 可延迟约束 | `DEFERRABLE INITIALLY DEFERRED` | **不存在**，唯一约束/索引一律立即校验 → Q 段演示直接互换必撞车，改用三步交换法 |
| 匿名块 / 动态 SQL | `DO $$ ... $$`、`RAISE NOTICE` | `BEGIN TRY ... END TRY BEGIN CATCH ... END CATCH` + `THROW` / `PRINT` |
| 保存点 | `SAVEPOINT x` / `ROLLBACK TO SAVEPOINT x` | `SAVE TRANSACTION x` / `ROLLBACK TRANSACTION x`（需 `SET XACT_ABORT OFF` 才能救回事务） |
| 调用过程 | `CALL proc(...)` | `EXEC proc @p = ..., @out = @var OUTPUT` |
| 执行计划 | `EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)` | `SET STATISTICS XML ON/OFF`（`SET SHOWPLAN_XML` 必须是批次唯一语句，本套件避开了这个坑） |
| NULL 安全比较 | `a IS DISTINCT FROM b` | 手写 `a <> b OR a IS NULL OR b IS NULL`（L 段） |
| 分页 | `LIMIT n OFFSET m` | `ORDER BY ... OFFSET m ROWS FETCH NEXT n ROWS ONLY` / `SELECT TOP (n)` |
| JSON | 原生 `jsonb`（`->>`、`@>`、`jsonb_agg`、GIN 索引） | `NVARCHAR(MAX)` + `ISJSON` CHECK + `JSON_VALUE` / `OPENJSON` / `FOR JSON PATH` / `STRING_AGG` |
| 数组 / 网络 / 几何 / 二进制 | `text[]`、`inet`、`point`、`lseg`、`bytea` | JSON 数组（`NVARCHAR`）、`VARCHAR(45)`、`DECIMAL(9,6)` 两列、`DECIMAL` 两列、`VARBINARY(MAX)` |
| 注释 | `COMMENT ON TABLE/COLUMN` | `sys.sp_addextendedproperty`（`MS_Description`） |
| 删除时的级联顺序 | `DROP SCHEMA ... CASCADE` | 无 `CASCADE` → 先显式 `DROP ... IF EXISTS` 点名，再用动态 SQL 兜底扫描残留对象 |
| 视图安全检查 | `WITH (security_barrier = true)` | 无对应物（可读作 SQL Server 的行级安全 / 视图权限模型差异） |
| 默认大小写 | 区分大小写取决于排序规则 | 实例排序规则 `SQL_Latin1_General_CP1_CI_AS`，`LIKE` 天然不区分大小写，不需要 `lower()` 包一层 |

## 已知限制与坑

1. **SQL Server 2022 没有原生 `json` 数据类型**（那是 2025 的能力，本实例实测
   `CREATE TABLE (j json)` 报 "Cannot find data type json"）。JSON 列一律用
   `NVARCHAR(MAX)` + `CHECK (ISJSON(col) = 1)`，查询用 `JSON_VALUE` / `OPENJSON` /
   `FOR JSON`。
2. **`ON DELETE RESTRICT` 不是 T-SQL 语法**（实测报语法错误）。等价语义用默认的
   `NO ACTION` 表达，见 `products → merchants`、`order_items → products` 等。
3. **自引用外键不能配 `ON DELETE SET NULL`**：实测报 "may cause cycles or multiple
   cascade paths"。`categories.parent_id` 只能 `NO ACTION`；`SET NULL` 演示放在
   `payments → orders`、`reviews → payments` 上。
4. **时态表的历史表禁止 `DELETE`**（错误 13560），清空必须先
   `ALTER TABLE ... SET (SYSTEM_VERSIONING = OFF)` 再 `TRUNCATE TABLE`。
5. **索引视图 / 过滤索引 / 持久化计算列 / XML 方法对会话 `SET` 选项敏感**。`sqlcmd`
   默认把 `QUOTED_IDENTIFIER` 设为 **OFF**，所以 `01` / `02` 以及 `03` 中所有做 DML
   的段（G/H/I/K/P/Q/T）开头都显式设置：
   `ANSI_NULLS / ANSI_PADDING / ANSI_WARNINGS / ARITHABORT / CONCAT_NULL_YIELDS_NULL /
   QUOTED_IDENTIFIER ON` + `NUMERIC_ROUNDABORT OFF`。漏掉会报 1934，或建不出过滤索引。
6. **目标表带触发器时 `OUTPUT` 必须配 `INTO`**（错误 334），所以 G/H 段先把 `$action`
   收进表变量再 `SELECT`。
7. **`WITH CHECK OPTION` 的视图上不能建 `INSTEAD OF` 触发器**（实测报错），
   `INSTEAD OF DELETE` 软删除触发器因此挂在 `sqlkit.active_products` 上。
8. **`DBCC CHECKIDENT(表, RESEED, n)` 在空表上把 `n` 当"下一个值"用**，在非空表上
   才是"下一个值 = n+1"（即常见的"重置到 1"要用 `RESEED 1`，而重灌数据后要"接着涨"
   才用 `RESEED max`）。本套件对需要确定 id 的表统一用 `SET IDENTITY_INSERT` 灌显式
   id，灌完再 `RESEED max`，所以重跑 500 单的 id 恒为 1..500。
9. **临时表作用域**：T-SQL 没有 `ON COMMIT DROP`，`#temp` 与连接同生命周期，所以 H
   段自己负责 `DROP TABLE IF EXISTS #stock_diff`（开头做清理、结尾做收尾）。
10. **`SET SHOWPLAN_XML ON` 必须是批次的唯一语句**，和"一段一个批次"的约束冲突，
    因此 O 段用 `SET STATISTICS XML ON/OFF` 拿真实执行计划（会多返回一个 XML 结果集，
    SQLKit 会把它当成一个额外的结果标签页）。Developer 版无需 `NOEXPAND` 提示。
11. **`sqlcmd -b` 遇到第一个错误就退出**，所以 I 段用 `-b` 跑必然是退出码 1。这是设计
    上的预期：I 段故意违反"时段互斥"触发器，再用 `ROLLBACK TRANSACTION <保存点>`
    把事务救回来。要看救援结果请去掉 `-b`。
12. `03` 的 P 段（删退款评价）、K 段（领券/作废订单，已复位）、H 段（对账商品，已清理）
    会改动数据；**要恢复出厂数据重跑 `01` → `02`**。
13. 依赖版本：`DATETRUNC`、`STRING_AGG ... WITHIN GROUP (ORDER BY ...)`、`DATE_BUCKET`
    需要 SQL Server 2022（兼容级别 160）。在 2019 及更早版本上 A/E/N 段需改写。
14. PG 套件的 `adminpack` / `pg_trgm` / `btree_gist` / `pgcrypto` 这类扩展在 T-SQL 没有
    对应物：三元组索引 → 无（要上全文索引目录），`gen_random_uuid()` → `NEWID()` /
    `NEWSEQUENTIALID()`，`gen_random_bytes()` → `HASHBYTES`。
