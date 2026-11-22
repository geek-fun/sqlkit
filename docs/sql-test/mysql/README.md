# SQLKit MySQL 压测套件

对应 PostgreSQL 套件（`docs/sql-test/postgres/`）的 MySQL 版本：同一套电商业务域
（用户 / 商户 / 商品 / 订单 / 支付 / 评价 / 优惠券），用于把 SQLKit 的各个功能面板
在复杂 MySQL 结构上跑一遍。

> 同域套件还有 [PostgreSQL](../postgres/README.md)、
> [SQL Server](../sqlserver/README.md)、[Oracle](../oracle/README.md)；
> 跨引擎总览见[套件索引](../README.md)。

**目标实例：** docker ssh-tunnel 测试栈里的 `mysql-test:3306`。
为不破坏栈里 `tester` 用户既有的权限，本套件使用**新建的库**：

| 项 | 值 |
|---|---|
| 主机 | `mysql-test`（经堡垒机 `localhost:2223` 转发时填这个内部名） |
| 端口 | `3306` |
| 数据库 | `sqlkit-demo`（`01_schema.sql` 自动创建） |
| 用户名 | `root`（建库 / 建例程需要）或栈内 `tester` |
| 密码 | `testpass` |

> 栈内 `tester` 只被授权 `testdb.*`，无法创建新库；因此这里用 `root/testpass`
> （与 `tester` 同密码）。若只想用 `tester`，请先执行
> `GRANT ALL PRIVILEGES ON \`sqlkit-demo\`.* TO 'tester'@'%';`。

按顺序执行三个文件：

| 文件 | 内容 |
|------|------|
| `01_schema.sql` | 建 `sqlkit-demo` / `sqlkit-analytics` / `sqlkit-billing` 三个库，11 张表 + 3 张视图 + 5 个触发器 + 2 个函数 + 3 个存储过程。含枚举字典表、生成列（STORED/VIRTUAL）、JSON 列与多值索引、空间列 + SPATIAL 索引、前缀索引、全文索引（ngram 中文分词）、RANGE 分区表、复合/单列外键、CHECK 与唯一约束。 |
| `02_seed.sql` | 确定性灌数：400 用户、10 商户、60 商品、500 订单（横跨 2026-01 ~ 2026-03）、1250 行明细、417 支付、168 评价、24 优惠券、510 订单事件。用递归 CTE 生成数字序列（非 PG 的 `generate_series`）。结尾有计数卡片。 |
| `03_complex_queries.sql` | 24 段查询演练场（`A`~`U`，含 `E2`/`F2`）：多 CTE 报表、递归 CTE 分类树、`LATERAL` Top-N、窗口函数、`WITH ROLLUP` + `GROUPING()`、JSON/空间/二进制类型、`JSON_TABLE` 关系化、UPSERT、对账合并、事务 + 保存点、存储过程与函数调用、`INTERSECT`/`EXCEPT`、窗口去重、JSON 聚合、全文检索、`EXPLAIN`/`EXPLAIN ANALYZE`、多表 `UPDATE`/`DELETE`、唯一键交换、例程元数据、`RANGE` 帧滑动窗口、分区裁剪、触发器审计。 |

每段注释横幅分隔，可单独选中执行。

## 在 SQLKit 里怎么跑

MySQL 适配器把编辑器内容作为**一条请求**整包下发给服务器，因此：

- **`01_schema.sql` / `02_seed.sql`** 可以整文件执行（连接库填 `sqlkit-demo`）。
- **注意 `DELIMITER`**：MySQL 例程（函数 / 过程）和触发器需要用 `DELIMITER $$` 包裹，
  而 `DELIMITER` 是 **mysql 客户端指令、不是 SQL**，服务端不认识它。SQLKit 会把
  `DELIMITER $$` 当普通语句下发并报语法错误。所以用 SQLKit 跑 `01_schema.sql` 时：
  1. 先整文件执行到例程段之前（即跳过三个 `DELIMITER $$ ... DELIMITER ;` 段落）；
  2. 再把每个例程 / 触发器**逐条**贴进编辑器执行 —— 从 `CREATE ...` 到
     `END` 为止（**不要**带上 `DELIMITER $$` / `DELIMITER ;` 这两行，
    也不需要末尾的 `$$`，SQLKit 会把它作为一条语句整包下发）。
  3. 每个对象都是 `DROP ... IF EXISTS` + `CREATE` 形式，可重复单独执行。
  > 已实测：9 个例程/触发器语句以「单条语句整包下发」方式全部创建成功，
  > 唯一的前提就是去掉客户端 `DELIMITER` 指令。
  - 也可以先用 mysql 客户端（`mysql ... < 01_schema.sql`）把 schema 建好，再回到 SQLKit 验证面板。
- **`03_complex_queries.sql`** 请**一段一段执行**（`A` → `U`）。段内若一次下发多条语句，
  中间任一语句报错会中断该请求的后续语句，结果集也会看不全。
- 会话级设置（`cte_max_recursion_depth`、`unique_checks`）只对当前连接有效，
  所以 `B` 段自己带了一句 `SET SESSION`；用 SQLKit 单独执行 `B` 段即可。

## 建议的 SQLKit 检查项

- **Schemas 面板** —— `sqlkit-demo` / `sqlkit-analytics` / `sqlkit-billing`
  三个库都应列出（MySQL 里 database 就是 schema 层）。
- **Tables / Columns / Table Info** —— 生成列（`email_lower` STORED、`name_len` VIRTUAL）、
  枚举列、JSON 列、空间列、分区表 `order_events` 与普通表 `orders` 的对比、
  表/列 COMMENT 元数据。
- **Indexes 面板** —— 主键 / 唯一 / 普通 / 前缀（`name(24)`）/ 全文（FULLTEXT + ngram）/
  空间（SPATIAL）/ 多值（`CAST(... AS UNSIGNED ARRAY)`）/ 函数索引（`(CAST(... AS CHAR))`）。
- **Foreign Keys 面板** —— 命名外键、`ON DELETE CASCADE / SET NULL / RESTRICT`、
  `ON UPDATE CASCADE`，以及自引用外键（`categories.parent_id`）。
- **Triggers 面板** —— `trg_orders_ai/au/ad`（INSERT/UPDATE/DELETE 各一）、
  `trg_products_bu`（BEFORE UPDATE 纠偏）、`trg_users_bu`（BEFORE UPDATE 写审计）。
- **Views 面板** —— `v_active_products`、`v_product_catalog`、`v_shop_with_check`
  （`WITH CASCADED CHECK OPTION`），以及跨库视图 `sqlkit-analytics.v_daily_sales`、
  `sqlkit-billing.v_reconciliation`。
- **Procedures / Functions 面板** —— 3 个过程（`claim_coupon` 带 OUT 参数、
  `void_order`、`merchant_report`）+ 2 个函数（`get_product_rank`、`cny_format`）。
- **DDL 预览 / Drop / Rename** —— 表、视图、过程、函数、触发器均可。
- **Query editor** —— 逐段执行 `03`，观察结果网格、行数、耗时、报错路径；
  `O` 段验证 `EXPLAIN ANALYZE` 结果面，`T` 段验证分区裁剪。

## 与 PostgreSQL 套件的方言差异

| 主题 | PostgreSQL | MySQL |
|---|---|---|
| schema 层 | 一个库内多 schema | database 即 schema（3 个库） |
| 分区 + 外键 | 分区表可带外键 | **不允许**：分区表不能有外键，故 `orders` 保持普通表，另建无外键的 `order_events` 演示 RANGE 分区 |
| 部分索引 | `WHERE` 部分索引 | 不支持，改用复合索引 + 生成列 |
| 排斥约束 | `EXCLUDE USING gist` | 不支持；并发占用改用唯一键/CHECK 近似 |
| 可延迟约束 | `DEFERRABLE INITIALLY DEFERRED` | 不支持，唯一键即时校验（`Q` 段专门演示差异） |
| 物化视图 | `MATERIALIZED VIEW` + `REFRESH` | 不支持，用跨库视图 + 聚合视图替代 |
| 数组 / INET / 范围类型 | 原生 `text[]`、`inet`、`tstzrange` | 无；用 JSON 数组、`VARBINARY(16)`、`valid_from/valid_to` 两列 |
| JSON 聚合排序 | `jsonb_agg(... ORDER BY ...)` | `JSON_ARRAYAGG` **不支持** `ORDER BY`（先用子查询排好） |
| 条件聚合 | `FILTER (WHERE ...)` | 无 `FILTER`，用 `CASE WHEN` 包在聚合里 |
| `DISTINCT ON` | 原生支持 | 无，用 `ROW_NUMBER() OVER (PARTITION BY ...)` |
| 聚合集 | `GROUPING SETS / CUBE / ROLLUP(...)` | 只有 `WITH ROLLUP`（`E2` 段用 `GROUPING()` 区分汇总行） |
| 生成数字序列 | `generate_series()` | 递归 CTE `SELECT 1 UNION ALL SELECT n+1 ...`（需放宽 `cte_max_recursion_depth`） |
| MERGE | `MERGE INTO`（PG15+） | 无 `MERGE`，用 `INSERT ... ON DUPLICATE KEY UPDATE` / `UPDATE JOIN` |
| 身份列显式插值 | `OVERRIDING SYSTEM VALUE` | 直接插显式 id 即可（并会推进 AUTO_INCREMENT） |
| 例程语法 | `LANGUAGE plpgsql` + `$$` | `DELIMITER $$` + `BEGIN ... END$$`（见上文注意事项） |
| 多语句下发 | 支持 `;` 分隔脚本 | 整包一条请求；中间报错会中断后续语句 |

## 注意事项

- 全部幂等：`01_schema.sql` 会 `DROP DATABASE` 后重建，`02_seed.sql` 先 `TRUNCATE`。
  连续执行两轮结果一致（已实测 2 轮，24/24 查询段全部通过）。
- `03_complex_queries.sql` 里 **`I` 段和 `Q` 段内含「预期错误」**
  （CHECK 约束 3819 / 唯一键 1062）。为了不让预期错误污染结果网格，
  这两段把事务封进了存储过程并在过程内用 `SQLEXCEPTION` 处理器吞掉错误，
  所以从 SQLKit 看是**干净的一次调用 + 一个结果集**；错误只在过程内部被演练。
- `02_seed.sql` 里 `users.tags` 是**数字数组**：MySQL 多值索引只接受数值 / 日期元素，
  字符串数组无法建多值索引（这也是与 PG `text[]` 的一处真实差异）。
- 空间列 `users.geo` 必须 `NOT NULL SRID 4326` 才能建 SPATIAL 索引（MySQL 限制）。
- 用命令行灌数时务必带 `--default-character-set=utf8mb4`，否则中文会被双重编码
  （SQLKit 驱动默认走 utf8mb4，不受此影响）。
- 计数卡片预期值：users 400 / products 60 / orders 500 / items 1250 / payments 417 /
  reviews 168 / coupons 24 / events 510 / audit 1000。
