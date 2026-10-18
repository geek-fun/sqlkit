# SQLKit Oracle 压测套件

面向真实的电商业务数据（users / merchants / categories / products / orders /
order_items / payments / reviews / coupons / reservations / audit_log），
用来把 SQLKit 的每个功能面板在复杂 Oracle 结构上压一遍。

> 同域套件还有 [PostgreSQL](../postgres/README.md)、[MySQL](../mysql/README.md)、
> [SQL Server](../sqlserver/README.md)；跨引擎总览见[套件索引](../README.md)。

**目标实例：** docker ssh-tunnel 测试栈 —— `oracle-test:1521/FREEPDB1`
（`sqlkit` / `testpass`），经堡垒机 `localhost:2223` 可达。
容器内等价直连：

```bash
docker exec -i sqlkit-ssh-oracle bash -lc \
  'sqlplus -S -L sqlkit/testpass@localhost:1521/FREEPDB1'
```

按顺序执行：

| 文件 | 作用 |
|------|------|
| `01_schema.sql` | 一个用户 schema（SQLKIT）里的对象全景：对象类型 + 域 + 枚举字典表，16 张表（RANGE + INTERVAL 自动分区的 `orders`、带 MAXVALUE 兜底分区的 `orders_archive`、复合主键/复合外键、虚拟列、IDENTITY、JSON/BOOLEAN/CLOB/BLOB/RAW），唯一/检查/延迟约束、ENABLE/DISABLE、系统命名约束，B-tree/唯一/函数/复合/位图/降序/压缩索引，普通视图 + 连接视图 + 聚合视图 + `WITH CHECK OPTION` 视图，`REFRESH COMPLETE ON DEMAND` 物化视图 + 唯一索引，序列 + 独立函数（含同名重载）/过程/包/包体，BEFORE/AFTER 行级触发器 + 语句级触发器 + 复合触发器。 |
| `02_seed.sql` | 确定性灌数据：400 用户、10 商户、8 类目（6 一级 + 2 二级）、60 商品、500 订单（2026-01 ~ 03）、1250 行明细、417 笔支付、168 条评价、24 张券、2 条预留，以及触发器自动产生的 1002 行审计日志。纯集合式 SQL（`CONNECT BY LEVEL` + `MOD`/`CASE` + `JSON_OBJECT`/`JSON_ARRAY`），结尾 COMMIT、刷新物化视图，并打印一张按表计数的校验卡片。 |
| `03_complex_queries.sql` | 20 段（A~T）查询演练场：多 CTE 报表、`CONNECT BY` + 递归 `WITH` 分类树、`CROSS APPLY` Top-N、窗口函数全家桶、`GROUPING SETS`/`ROLLUP`/`CUBE`/`GROUPING_ID`/`FILTER`、`PIVOT`/`UNPIVOT`、JSON 函数、`MERGE` upsert、库存对账、事务 + `SAVEPOINT`（全文件唯一一处故意报错）、动态 SQL、`CALL` 过程、集合操作、去重与分页、JSON 聚合、`EXPLAIN PLAN` + `DBMS_XPLAN`、关联子查询 UPDATE/DELETE、延迟约束邮箱互换、函数重载调用、`RANGE` 区间窗口、INTERVAL 分区路由。 |

`03_complex_queries.sql` 的每段都用 banner 注释分隔，可以逐段执行（或选中一段执行），
用来测试查询编辑器、结果网格、大结果集、多语句执行与报错路径。

## 目录

### `01_schema.sql` 的章节

| 章节 | 内容 |
|------|------|
| 0 | 清场（`DROP ... IF EXISTS` + `PURGE RECYCLEBIN`，可反复执行） |
| 1 | 独立序列 `coupon_code_seq`（CACHE 20 / CYCLE / MAXVALUE） |
| 2 | 对象类型 `money_amount` + 域 `cny_price`（SQL*Plus 需 `/` 终止） |
| 3 | 枚举字典表 `order_status` / `product_condition` |
| 4 | `users`（全类型覆盖 + deferrable 唯一 + DISABLE/ENABLE 约束） |
| 5 | `merchants`（`UNIQUE ... USING INDEX`） |
| 6 | `categories`（自引用树，`ON DELETE SET NULL`） |
| 7 | `products`（复合主键 + 双外键 + 虚拟列 `price_level`） |
| 8 | `orders`（RANGE + INTERVAL 分区、复合主键、复合外键源头、`ENABLE ROW MOVEMENT`） |
| 9 | `orders_archive`（RANGE + `MAXVALUE` 兜底分区） |
| 10 | `order_items`（复合外键 `ON DELETE CASCADE` + 虚拟列 `line_total`） |
| 11 | `payments`（复合外键 `ON DELETE SET NULL`、系统命名 FK） |
| 12 | `reservations`（CHECK 区间约束 = PG EXCLUDE 的降级替代） |
| 13 | `reviews` / `coupons`（系统命名 FK、CASE 函数唯一索引） |
| 14 | `audit_log` / `stock_diff` / `billing_invoices`（审计落点、MERGE 暂存、billing 域占位表） |
| 15 | 索引全谱系（B-tree / 唯一 / 函数 / 复合 / 位图 / 降序 / COMPRESS） |
| 16 | 视图（`active_products` / `product_catalog` / `shop_with_check` / `merchant_sales_summary`） |
| 17 | 物化视图 `analytics_daily_sales` + 唯一索引 + `analytics_sales_dashboard` |
| 18 | 函数（含同名重载）/ 独立过程 / 包 + 包体 |
| 19 | 触发器（AFTER 行级多事件、BEFORE UPDATE 行级、AFTER INSERT 语句级、复合触发器） |

### `03_complex_queries.sql` 的章节

| 段 | 主题 |
|----|------|
| A | 多 CTE 商业报表：周复购 + `PERCENT_RANK` 客单价分位 |
| B | 分类树：`CONNECT BY`/`SYS_CONNECT_BY_PATH` + 递归 `WITH` 两种写法 |
| C | `CROSS APPLY`：每商户 Top-3 销量商品 + `RANK` |
| D | 窗口函数全家桶：`ROWS` 帧移动平均、`LAG`/`LEAD`、`FIRST_VALUE` |
| E | 聚合集与转置：`GROUPING SETS`/`ROLLUP`/`CUBE`/`GROUPING_ID`/`FILTER` + `PIVOT`/`UNPIVOT` |
| F | JSON：`JSON_VALUE` / `JSON_QUERY` / `JSON_EXISTS` / `JSON_TABLE` |
| G | UPSERT：`MERGE`（对应 PG 的 `ON CONFLICT DO NOTHING` / `DO UPDATE`） |
| H | `MERGE` 库存对账（聚合差异后合并，命中即更新、未命中即插入） |
| I | 事务 + `SAVEPOINT`：**唯一一处故意报错**（CHECK 约束冲突 + 回滚到保存点） |
| J | 动态 SQL 函数调用（对应 PG 的 `DO` 块） |
| K | `CALL` 包过程 + 独立过程（无匿名块，分隔器友好） |
| L | 集合操作：`FULL OUTER JOIN` / `INTERSECT` / `MINUS` |
| M | 去重与分页：`ROW_NUMBER` / `KEEP (DENSE_RANK FIRST/LAST)` / `FETCH FIRST` + `OFFSET` |
| N | JSON 聚合：`JSON_ARRAYAGG` / `JSON_OBJECTAGG`（订单地址树） |
| O | `EXPLAIN PLAN FOR` + `DBMS_XPLAN.DISPLAY`（含分区裁剪计划） |
| P | 关联子查询 `UPDATE` / `DELETE`（VIP 评价标记、退款与保留策略清理） |
| Q | `DEFERRABLE INITIALLY DEFERRED` 唯一约束：同事务内互换邮箱 |
| R | 函数调用三途径（同名重载 NUMBER/VARCHAR2、包函数） |
| S | `RANGE BETWEEN INTERVAL '7' DAY PRECEDING` 滚动窗口（物化视图上） |
| T | 分区路由：未来日期触发 INTERVAL 自动建分区 + 分区元数据 |

## SQLKit 面板对照清单

- **Schemas 面板** —— Oracle 里"用户即 schema"，本套件只占用 `SQLKIT` 一个用户。
  PG 的 `store` / `analytics` / `billing` 三个 schema 收敛为同一 schema 下的表名：
  核心交易域不改名，分析域用 `analytics_` 前缀（`analytics_daily_sales` /
  `analytics_sales_dashboard`），计费域用 `billing_invoices` 空表镜像。
- **Tables / Columns / Table Info** —— 分区父表与分区（`USER_TAB_PARTITIONS`）、
  复合主键、IDENTITY 列、虚拟列（`line_total` / `price_level`）、对象类型列
  （`wallet` / `dimensions` / `note`）、JSON/BOOLEAN/CLOB/BLOB/RAW 列、
  `COMMENT ON` 注释、DISABLED 约束状态。
- **DDL 预览** —— 表、分区表、视图、物化视图、序列、函数/过程/包都能取到 DDL。
- **Drop / Rename** —— 拿 `stock_diff`、`billing_invoices` 这类边角对象练手最合适。
- **Indexes 面板** —— 普通 B-tree、唯一、`UPPER(name)` 函数索引、`LOWER(sku)` 复合唯一函数索引、
  位图索引、`rating DESC` 降序索引、`COMPRESS 1` 压缩索引、`LOCAL` 分区索引、
  `CASE WHEN claimed_by IS NOT NULL THEN id END` 部分唯一替代。
- **Foreign Keys 面板** —— 命名与系统命名（`SYS_C...`）并存、单列与复合外键、
  `ON DELETE CASCADE` / `ON DELETE SET NULL`、自引用外键。
- **Constraints** —— 唯一（含 `DEFERRABLE INITIALLY DEFERRED`）、CHECK、NOT NULL、
  DEFAULT、`DISABLE` 与 `ENABLE`、`UNIQUE ... USING INDEX`。
- **Triggers 面板** —— `BEFORE UPDATE` 行级、`AFTER INSERT OR UPDATE OR DELETE` 行级、
  语句级、复合（COMPOUND）触发器。
- **Views 面板** —— 普通视图、连接视图（`WITH READ ONLY`）、`WITH CHECK OPTION`
  可更新视图、聚合视图、物化视图之上的仪表盘视图。
- **Procedures / Functions 面板** —— 独立过程（`refresh_daily_sales` / `void_order`）、
  包过程（`coupon_pkg.claim_coupon`）、独立函数（`stale_paid_orders` 用动态 SQL）、
  **同名重载**（`get_product_rank(NUMBER)` / `get_product_rank(VARCHAR2)`）、包函数。
- **Sequences 面板** —— `coupon_code_seq`（START 100000 / MAXVALUE / CACHE 20 / CYCLE）
  以及各表 IDENTITY 背后的系统序列。
- **查询编辑器** —— 03 的 A~T 逐段执行：每段都应有结果网格，只有 I 段是预期报错。
- **审计留痕** —— `audit_log` 由触发器写入，SQLKit 发出的 INSERT/UPDATE/DELETE
  都会在表里留下 JSON 快照，可用来确认语句真的打到了隧道另一端的数据库。

## 连接参数

| 项 | 值 |
|----|----|
| Host（ssh-tunnel 栈内主机名） | `oracle-test` |
| Port | `1521` |
| Service name | `FREEPDB1` |
| 用户 / 口令 | `sqlkit` / `testpass` |
| 堡垒机（宿主机端口） | `localhost:2223`（ssh 隧道转发到容器 1521） |
| 容器名（本地验证用） | `sqlkit-ssh-oracle` |
| 版本 | Oracle AI Database 26ai Free 23.26.3.0.0（`gvenzl/oracle-free:23-slim`） |
| 权限 | 普通用户，无 DBA；不查 `v$`/`v_$` 视图，不建用户/表空间 |

SQLKit 里新建 Oracle 连接时按上表填写（schema 留空即用 `sqlkit` 用户自己的 schema）。
命令行直连（走容器本地监听）：

```bash
docker exec -i sqlkit-ssh-oracle bash -lc \
  'sqlplus -S -L sqlkit/testpass@localhost:1521/FREEPDB1'
```

## Oracle 与 PostgreSQL 的方言差异（本套件的替换清单）

| PostgreSQL | Oracle（本套件的做法） |
|------------|------------------------|
| 多 schema（store/analytics/billing） | 一个用户 schema + 表名前缀（`analytics_*` / `billing_*`） |
| `text` | `VARCHAR2` |
| `boolean` | 23ai 原生 `BOOLEAN`（注意：`DEFAULT` 必须写在 `NOT NULL` **之前**，否则 ORA-03076） |
| `serial` / `GENERATED ... AS IDENTITY` | `GENERATED ALWAYS AS IDENTITY` / `GENERATED BY DEFAULT AS IDENTITY`；显式写值需 `OVERRIDING SYSTEM VALUE`（仅 VALUES 形式），`INSERT ... SELECT` 不允许 |
| `setval(...)` 校准序列 | `ALTER TABLE t MODIFY id GENERATED BY DEFAULT AS IDENTITY (START WITH LIMIT VALUE)` |
| `DEFAULT nextval('seq')::text` | 不允许 `TO_CHAR(seq.NEXTVAL)` 作为列默认值，改为在 `INSERT ... SELECT` 里显式取 `seq.NEXTVAL` |
| `jsonb` | 原生 `JSON` 列 + `JSON_VALUE`/`JSON_QUERY`/`JSON_EXISTS`/`JSON_TABLE`/`JSON_ARRAYAGG`/`JSON_OBJECTAGG` |
| 数组 `text[]` | Oracle 无数组类型，改用 JSON 数组（`JSON_ARRAY`） |
| `inet` / `point` / `xml` | `VARCHAR2` / 两个 `NUMBER` 经纬度 / `CLOB`（未使用 `XMLType`） |
| `bytea` | `BLOB`（头像、支付载荷）与 `RAW(16)`（设备指纹） |
| 复合类型 `money_amount` | 对象类型 `CREATE TYPE money_amount AS OBJECT (...)`，列值为 `money_amount(300,'CNY')`；SQL*Plus 需要 `/` 终止该 DDL |
| `CREATE DOMAIN cny_price` | 23ai 原生 `CREATE DOMAIN cny_price AS NUMBER(12,2) CHECK (...)` |
| `CREATE TYPE ... AS ENUM` | 字典表 + 外键（`order_status` / `product_condition`） |
| `EXCLUDE` 约束（时段互斥） | **不存在**；降级为 `CHECK (valid_to > valid_from)` + `(resource_id, valid_from)` 唯一占位 |
| 部分索引 `WHERE ...` | **不存在**；用 `CASE` 表达式函数索引近似（券"仅已领取行唯一"） |
| GIN / trigram / BRIN 索引 | **不存在**；函数索引 + 普通 B-tree 替代 |
| `PARTITION BY RANGE` + DEFAULT 分区 | `PARTITION BY RANGE ... INTERVAL (NUMTOYMINTERVAL(1,'MONTH'))` 自动分区；INTERVAL 表**禁止** MAXVALUE 分区（ORA-14761），兜底分区放到 `orders_archive` |
| `TRUNCATE ... RESTART IDENTITY CASCADE` | 逐表 `TRUNCATE`（子表→父表；父表被引用时子表必须先清空），IDENTITY 不重置 |
| `generate_series(1, n)` | `(SELECT LEVEL g FROM dual CONNECT BY LEVEL <= n)` |
| `LATERAL (...)` / `LIMIT n` | `CROSS APPLY` / `OUTER APPLY` + `FETCH FIRST n ROWS ONLY` |
| `WITH RECURSIVE` | 递归 `WITH` 子查询因子（12c+）或 `CONNECT BY` + `SYS_CONNECT_BY_PATH` |
| `DISTINCT ON` | `ROW_NUMBER()` 去重或 `KEEP (DENSE_RANK LAST/FIRST)` |
| `ON CONFLICT DO NOTHING/UPDATE` | `MERGE`（23ai **不支持** `INSERT ... ON DUPLICATE KEY UPDATE`，实测 ORA-03048） |
| `UPDATE ... FROM` / `DELETE ... USING` | 关联子查询（23ai 其实也接受 `UPDATE ... FROM`，但套件统一用关联子查询） |
| `IS DISTINCT FROM` | **不存在**（实测 ORA-00908），用 `NVL(a,-1) <> NVL(b,-1)` 归一化 |
| `EXCEPT` | `MINUS` |
| `count(*) FILTER (WHERE ...)` | 23ai 支持，本套件直接使用 |
| 空串 `''` | Oracle 视作 `NULL`，不要依赖空串语义 |
| 未加引号标识符 | 全部折叠为大写（本套件不引用任何标识符，一律不加引号） |
| `DO $$ ... $$` 匿名块 | 独立函数/过程 + `CALL`（避免匿名块内部分号打乱分隔器） |
| 触发器 `TG_OP` / `TG_LEVEL` | `INSERTING` / `UPDATING` / `DELETING` 分支 + 语句级触发器 + 复合触发器；`:NEW`/`:OLD` 不能在 `CASE` 表达式里混用（ORA-00984），用 `IF/ELSIF` |
| 视图 `security_barrier` | **不存在**；用 `WITH READ ONLY` 近似 |
| `EXPLAIN (ANALYZE, BUFFERS)` | `EXPLAIN PLAN FOR ...` + `SELECT * FROM TABLE(DBMS_XPLAN.DISPLAY())` |
| `DROP ... IF EXISTS` | 23ai 支持（`DROP TABLE IF EXISTS t PURGE` 等）；12c~19c 需改写为 `BEGIN EXECUTE IMMEDIATE ...; EXCEPTION WHEN OTHERS THEN NULL; END;` 守卫块（即 `01` 第 0 节注释里给出的写法） |

## PL/SQL 对象怎么跑（SQLKit vs SQL*Plus）

SQLKit 现在会保留匿名块、函数/过程、包、触发器和对象类型定义中的内部分号，
并识别独占一行的 `/` 块终止符。字符串、注释和 `q'[...]'` 字面量中的分号也会保留。

- **整文件执行**：选中 `01_schema.sql` 全文执行，随后执行 `02_seed.sql`。
  自动化回归使用真实 JDBC 适配器验证这两份文件，以及 `03` 的所有分段。
- **逐个对象执行**：选中完整定义（从 `CREATE OR REPLACE ...` 到 `END;`）即可；
  末尾的 `/` 行可保留。编辑器的光标定位仍建议配合显式选择执行 PL/SQL 对象。
- **SQL*Plus 整文件跑**（本套件的验证方式，`/` 是 SQL*Plus 的块终止符；
  这些指令不要写进三个 SQL 文件本身）：

  ```bash
  { printf 'WHENEVER SQLERROR EXIT SQL.SQLCODE\nWHENEVER OSERROR EXIT FAILURE\nSET DEFINE OFF\n'; \
    cat 01_schema.sql; } \
    | docker exec -i sqlkit-ssh-oracle bash -lc \
        'sqlplus -S -L sqlkit/testpass@localhost:1521/FREEPDB1'
  ```

  单条语句语法错误时 `sqlplus` 会以非 0 退出码结束（`EXIT SQL.SQLCODE`），
  可以用来做 fail-fast 校验。
- **事务相关分段（I / Q）已改为调用存储过程**：SQLKit 的 JDBC 桥逐条自动提交，
  `SAVEPOINT` 与可延迟约束都需要"多条语句在同一事务里"，逐条下发会被拆散。
  因此 `01_schema.sql` 第 20 节建了 `demo_savepoint_rescue`、
  `demo_deferrable_swap`、`demo_deferrable_restore` 三个过程，`03` 的 I / Q 段
  用单条 `CALL` 触发：一次请求内完成整个事务，**界面上不再出现 ORA-01086 /
  ORA-02091 之类的噪声报错**。

## 已知限制

- `03_complex_queries.sql` **只有一处故意报错**：I 段的 `INSERT INTO reservations`
  故意让 `valid_to < valid_from`，触发 `ck_reservations_range`（ORA-02290），
  紧接着 `ROLLBACK TO SAVEPOINT` 救回。其它任何错误都说明环境或数据需要检查。
- I 段把预期错误包在 `demo_savepoint_rescue` 过程内部（内层异常处理器吞掉
  ORA-02290），所以从 SQLKit 看是**干净的一次 `CALL` + 一个结果集**；错误只在
  过程内部被演练。Q 段同理，用 `CALL demo_deferrable_swap()` 演示可延迟约束
  允许事务中途瞬时冲突，再用 `CALL demo_deferrable_restore()` 还原。
- O 段的 `EXPLAIN PLAN FOR` 与 `DBMS_XPLAN.DISPLAY()` 要连着跑：前者写
  `PLAN_TABLE`，后者读它（23ai 的 `PLAN_TABLE` 是公共表，实测跨会话也能读到
  最近一次计划，但仍建议在同一会话顺序执行）。
- `reservations` 无法表达 PG 的区间互斥：Oracle 没有 `EXCLUDE` 约束，且
  TIMESTAMP WITH TIME ZONE 列不能进唯一键（ORA-02329），因此预留窗口用普通
  `TIMESTAMP` + `CHECK` + 唯一占位约束近似。
- 对象类型列（`users.wallet`、`products.dimensions`、`reservations.note`）在
  部分客户端里会显示成 STRUCT / `MONEY_AMOUNT(300, 'CNY')` 形态，属预期。
- 各表主键都由 IDENTITY 生成，`02_seed.sql` 反复执行会让自增值持续增长
  （`TRUNCATE` 不重置 IDENTITY），计数卡片不受影响；业务关系一律用
  `username` / `sku` / `order_no` 等业务键 JOIN，不依赖具体自增值。
- T 段插入 2026-12 的未来订单会触发 INTERVAL 自动分区，而建分区是 DDL、会隐式
  提交（Oracle 与 PG 的差别之一），因此该段用 `DELETE` 清理演示行而不是
  `ROLLBACK`，并在库里留下一个自动创建的分区（`SYS_Pxxxx`，计数不受影响）。
- Oracle 用户没有 DBA 权限：套件不查询 `v$`/`v_$` 视图，不建用户/表空间，
  也不使用需要额外授权的 `ON PREBUILT TABLE` 物化视图。
- `CREATE OR REPLACE VIEW ... WITH CHECK OPTION` 只覆盖可更新视图
  （`shop_with_check`）；连接/聚合视图一律 `WITH READ ONLY`。
- 审计表会随触发器持续增长：`02` 重跑会先 `TRUNCATE audit_log`，
  正常运行一次后约 1002 行。
