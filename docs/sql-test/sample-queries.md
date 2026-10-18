# SQLKit 示例查询阶梯（易 → 难）

配套 `docs/sql-test/` 四个压测套件使用。所有查询都已在**四个引擎的真实实例**上跑通
（PostgreSQL 17 / MySQL 8.4 / SQL Server 2022 / Oracle 23ai，各 500 订单 / 400 用户 /
60 商品），可直接贴进查询编辑器逐段执行，用来验证结果网格、行数、耗时、排序、分页、
执行计划等交互。

## 0. 先做一步准备（让下面的查询无需改前缀）

下面的查询一律**不写 schema 前缀**，按你想测的引擎做一次准备即可：

| 引擎 | 连接数据库 | 准备工作 |
|---|---|---|
| PostgreSQL | `testdb` | 先执行 `SET search_path TO store, analytics, billing;` |
| MySQL | `sqlkit-demo` | 无（database 即 schema） |
| SQL Server | `testdb` | 查询里的表名要加 `sqlkit.` 前缀（见下方说明） |
| Oracle | `sqlkit` 用户 | 无 |

SQL Server 没有会话级 schema 搜索路径，所以它的表名必须写成 `sqlkit.orders` 这种形式；
不想每次手改的话，把下面每段里的 `orders` / `products` / `users` / `merchants` /
`order_items` 前面加上 `sqlkit.` 即可。

**行数限制子句**三种写法，按引擎替换：

```sql
LIMIT 5                              -- PostgreSQL / MySQL
FETCH FIRST 5 ROWS ONLY              -- Oracle
OFFSET 0 ROWS FETCH NEXT 5 ROWS ONLY -- SQL Server（必须带 ORDER BY）
```

---

## 1. 入门：单表过滤与排序

### 1.1 最贵的 5 件商品

```sql
SELECT sku, name, price
FROM products
ORDER BY price DESC
LIMIT 5;
```

验证点：结果网格列宽、数字右对齐、排序箭头、`LIMIT` 是否生效。

### 1.2 已完成订单的汇总

```sql
SELECT COUNT(*) AS order_cnt,
       ROUND(SUM(total_amount), 2) AS revenue
FROM orders
WHERE status = 'completed';
```

验证点：单行单行结果、聚合列的 NULL 处理、`ROUND` 精度。

### 1.3 订单状态分布

```sql
SELECT status, COUNT(*) AS cnt,
       ROUND(SUM(total_amount), 2) AS revenue,
       ROUND(AVG(total_amount), 2) AS avg_amount
FROM orders
GROUP BY status
ORDER BY revenue DESC;
```

验证点：GROUP BY 结果集、多聚合列、排序。

---

## 2. 进阶：HAVING 与多表连接

### 2.1 下单 ≥ 2 次的用户（HAVING）

```sql
SELECT user_id, COUNT(*) AS order_cnt, ROUND(SUM(total_amount), 2) AS spend
FROM orders
GROUP BY user_id
HAVING COUNT(*) >= 2
ORDER BY spend DESC
LIMIT 5;
```

> 测试数据里每个用户最多 2 单，所以阈值写 2；改成 `>= 3` 会返回空结果集——
> 这也正好可以验证**空结果网格是否仍然显示列名**。

### 2.2 商户营收排行（三表 JOIN + 聚合）

```sql
SELECT m.name AS merchant,
       COUNT(DISTINCT p.id) AS products,
       SUM(oi.quantity) AS units,
       ROUND(SUM(oi.line_total), 2) AS revenue
FROM order_items oi
JOIN products p ON p.id = oi.product_id
JOIN merchants m ON m.id = p.merchant_id
GROUP BY m.name
ORDER BY revenue DESC;
```

验证点：JOIN 走的是哈希/嵌套循环不影响结果、`COUNT(DISTINCT)`、聚合列别名。

### 2.3 从未被下单的商品（反连接 NOT EXISTS）

```sql
SELECT p.id, p.name, p.stock
FROM products p
WHERE NOT EXISTS (
    SELECT 1 FROM order_items oi WHERE oi.product_id = p.id
)
ORDER BY p.id;
```

验证点：子查询、小结果集（测试数据里只有 2 件）。

### 2.4 消费额高于平均的用户（标量子查询）

```sql
SELECT u.username, ROUND(SUM(o.total_amount), 2) AS spend
FROM users u
JOIN orders o ON o.user_id = u.id
GROUP BY u.username
HAVING SUM(o.total_amount) > (SELECT AVG(total_amount) * 4 FROM orders)
ORDER BY spend DESC
LIMIT 5;
```

验证点：标量子查询、HAVING 比较子查询结果。

---

## 3. 高级：CTE 与窗口函数

### 3.1 类目内商品销量排名（CTE + RANK）

```sql
WITH sales AS (
    SELECT p.category_id, p.id, p.name, SUM(oi.quantity) AS sold
    FROM products p
    JOIN order_items oi ON oi.product_id = p.id
    GROUP BY p.category_id, p.id, p.name
)
SELECT category_id, name, sold,
       RANK() OVER (PARTITION BY category_id ORDER BY sold DESC) AS rnk
FROM sales
ORDER BY category_id, rnk
LIMIT 5;
```

### 3.2 每日营收与累计流水（窗口 SUM）

```sql
WITH daily AS (
    SELECT order_date, SUM(total_amount) AS revenue
    FROM orders
    GROUP BY order_date
)
SELECT order_date,
       ROUND(revenue, 2) AS revenue,
       ROUND(SUM(revenue) OVER (ORDER BY order_date), 2) AS running_total
FROM daily
ORDER BY order_date;
```

验证点：85 行的结果集（适合看虚拟滚动）、窗口累计列。

### 3.3 日环比增减（LAG）

```sql
WITH daily AS (
    SELECT order_date, SUM(total_amount) AS revenue
    FROM orders
    GROUP BY order_date
)
SELECT order_date,
       ROUND(revenue, 2) AS revenue,
       ROUND(revenue - LAG(revenue) OVER (ORDER BY order_date), 2) AS delta
FROM daily
ORDER BY order_date;
```

验证点：`delta` 首行为 NULL —— 可验证网格对 NULL 的渲染。

### 3.4 各状态营收占比（窗口函数套聚合）

```sql
SELECT status,
       ROUND(SUM(total_amount), 2) AS revenue,
       ROUND(100.0 * SUM(total_amount) / SUM(SUM(total_amount)) OVER (), 2) AS pct_of_total
FROM orders
GROUP BY status
ORDER BY revenue DESC;
```

验证点：窗口函数作用在聚合结果之上（`SUM(SUM(...)) OVER ()`），是最容易被解析器写错的一类。

### 3.5 每类目 Top-2 商品（DENSE_RANK + 外层过滤）

```sql
WITH sales AS (
    SELECT p.category_id, p.name, SUM(oi.line_total) AS revenue,
           DENSE_RANK() OVER (PARTITION BY p.category_id
                              ORDER BY SUM(oi.line_total) DESC) AS rnk
    FROM products p
    JOIN order_items oi ON oi.product_id = p.id
    GROUP BY p.category_id, p.name
)
SELECT category_id, name, ROUND(revenue, 2) AS revenue, rnk
FROM sales
WHERE rnk <= 2
ORDER BY category_id, rnk;
```

### 3.6 客户分层（多 CTE + CASE）

```sql
WITH per_user AS (
    SELECT user_id, COUNT(*) AS orders, SUM(total_amount) AS spend
    FROM orders
    GROUP BY user_id
),
scored AS (
    SELECT user_id, orders, spend,
           ROUND(spend / orders, 2) AS aov,
           CASE WHEN spend >= 4000 THEN 'whale'
                WHEN spend >= 1500 THEN 'loyal'
                ELSE 'casual' END AS segment
    FROM per_user
)
SELECT segment, COUNT(*) AS users,
       ROUND(SUM(spend), 2) AS total_spend,
       ROUND(AVG(aov), 2) AS avg_order_value
FROM scored
GROUP BY segment
ORDER BY total_spend DESC;
```

验证点：两段 CTE 串联、`CASE` 分桶、结果只有 3 行但计算量大。

---

## 4. 方言差异：同一个问题，四种写法

这一节最适合用来对比 SQLKit 的**方言分支是否接对**。

### 4.1 取最贵的 5 件商品

```sql
-- PostgreSQL / MySQL
SELECT sku, name, price FROM products ORDER BY price DESC LIMIT 5;

-- Oracle
SELECT sku, name, price FROM products ORDER BY price DESC FETCH FIRST 5 ROWS ONLY;

-- SQL Server
SELECT TOP (5) sku, name, price FROM sqlkit.products ORDER BY price DESC;
```

### 4.2 按月汇总营收

```sql
-- PostgreSQL
SELECT date_trunc('month', order_date)::date AS ym, ROUND(SUM(total_amount), 2) AS revenue
FROM store.orders GROUP BY 1 ORDER BY 1;

-- MySQL
SELECT DATE_FORMAT(order_date, '%Y-%m') AS ym, ROUND(SUM(total_amount), 2) AS revenue
FROM orders GROUP BY 1 ORDER BY 1;

-- SQL Server
SELECT DATEFROMPARTS(YEAR(order_date), MONTH(order_date), 1) AS ym,
       ROUND(SUM(total_amount), 2) AS revenue
FROM sqlkit.orders
GROUP BY DATEFROMPARTS(YEAR(order_date), MONTH(order_date), 1)
ORDER BY 1;

-- Oracle
SELECT TRUNC(order_date, 'MM') AS ym, ROUND(SUM(total_amount), 2) AS revenue
FROM orders GROUP BY TRUNC(order_date, 'MM') ORDER BY 1;
```

### 4.3 分类树（自引用递归）

```sql
-- PostgreSQL / MySQL（需要 RECURSIVE 关键字）
WITH RECURSIVE tree AS (
    SELECT id, parent_id, name, 0 AS lvl
    FROM categories WHERE parent_id IS NULL
    UNION ALL
    SELECT c.id, c.parent_id, c.name, t.lvl + 1
    FROM categories c JOIN tree t ON c.parent_id = t.id
)
SELECT id, name, lvl FROM tree ORDER BY lvl, id;

-- SQL Server（无 RECURSIVE，且要放开递归上限）
WITH tree AS (
    SELECT id, parent_id, name, 0 AS lvl
    FROM sqlkit.categories WHERE parent_id IS NULL
    UNION ALL
    SELECT c.id, c.parent_id, c.name, t.lvl + 1
    FROM sqlkit.categories c JOIN tree t ON c.parent_id = t.id
)
SELECT id, name, lvl FROM tree ORDER BY lvl, id
OPTION (MAXRECURSION 100);

-- Oracle（无 RECURSIVE 关键字）
WITH tree (id, parent_id, name, lvl) AS (
    SELECT id, parent_id, name, 0 FROM categories WHERE parent_id IS NULL
    UNION ALL
    SELECT c.id, c.parent_id, c.name, t.lvl + 1
    FROM categories c JOIN tree t ON c.parent_id = t.id
)
SELECT id, name, lvl FROM tree ORDER BY lvl, id;
```

### 4.4 从 JSON 地址里取城市并计数

```sql
-- PostgreSQL
SELECT shipping_addr ->> 'city' AS city, COUNT(*) AS orders
FROM store.orders GROUP BY 1 ORDER BY orders DESC;

-- MySQL（JSON 路径要带 $）
SELECT shipping_addr ->> '$.city' AS city, COUNT(*) AS orders
FROM orders GROUP BY 1 ORDER BY orders DESC;

-- SQL Server
SELECT JSON_VALUE(shipping_addr, '$.city') AS city, COUNT(*) AS orders
FROM sqlkit.orders GROUP BY JSON_VALUE(shipping_addr, '$.city') ORDER BY orders DESC;

-- Oracle
SELECT JSON_VALUE(shipping_addr, '$.city') AS city, COUNT(*) AS orders
FROM orders GROUP BY JSON_VALUE(shipping_addr, '$.city') ORDER BY orders DESC;
```

---

## 5. 写操作：幂等 UPSERT（验证影响行数显示）

同一个 `code` 跑两次，第二次应走 UPDATE。测试完记得删除（见最后）。

```sql
-- PostgreSQL
INSERT INTO store.coupons (code, discount, min_amount, valid_until)
VALUES ('TEST-UPSERT', 10.00, 100, '2026-12-31')
ON CONFLICT (code) DO UPDATE SET discount = EXCLUDED.discount
RETURNING id, code, discount;

-- MySQL（8.4 已移除 VALUES()，改用行别名）
INSERT INTO coupons (code, discount, min_amount, valid_until)
VALUES ('TEST-UPSERT', 10.00, 100, '2026-12-31') AS new
ON DUPLICATE KEY UPDATE discount = new.discount;

-- SQL Server
MERGE sqlkit.coupons AS t
USING (SELECT 'TEST-UPSERT' AS code,
              CAST(10.00 AS DECIMAL(12,2)) AS discount,
              CAST(100 AS DECIMAL(12,2)) AS min_amount,
              CAST('2026-12-31' AS DATETIMEOFFSET) AS valid_until) AS s
ON t.code = s.code
WHEN MATCHED THEN UPDATE SET discount = s.discount
WHEN NOT MATCHED THEN INSERT (code, discount, min_amount, valid_until)
     VALUES (s.code, s.discount, s.min_amount, s.valid_until);

-- Oracle
MERGE INTO coupons t
USING (SELECT 'TEST-UPSERT' AS code, 10.00 AS discount, 100 AS min_amount,
              TO_TIMESTAMP_TZ('2026-12-31 00:00:00 +08:00',
                              'YYYY-MM-DD HH24:MI:SS TZH:TZM') AS valid_until
       FROM dual) s
ON (t.code = s.code)
WHEN MATCHED THEN UPDATE SET t.discount = s.discount
WHEN NOT MATCHED THEN INSERT (t.code, t.discount, t.min_amount, t.valid_until)
     VALUES (s.code, s.discount, s.min_amount, s.valid_until);

-- 清理（四个引擎通用，MySQL 里去掉 store. / 加反引号）
DELETE FROM store.coupons WHERE code = 'TEST-UPSERT';
```

验证点：第一次「影响 1 行」，第二次仍是 1 行（UPDATE），且 `id` 不变。

---

## 6. 执行计划（验证 Explain 面板 / 结果面）

```sql
-- PostgreSQL
EXPLAIN (ANALYZE, BUFFERS)
SELECT p.name, SUM(oi.quantity) AS sold
FROM store.products p JOIN store.order_items oi ON oi.product_id = p.id
GROUP BY p.name ORDER BY sold DESC LIMIT 5;

-- MySQL
EXPLAIN ANALYZE
SELECT p.name, SUM(oi.quantity) AS sold
FROM products p JOIN order_items oi ON oi.product_id = p.id
GROUP BY p.name ORDER BY sold DESC LIMIT 5;

-- SQL Server（三条语句要作为一批执行）
SET STATISTICS XML ON;
SELECT p.name, SUM(oi.quantity) AS sold
FROM sqlkit.products p JOIN sqlkit.order_items oi ON oi.product_id = p.id
GROUP BY p.name ORDER BY sold DESC
OFFSET 0 ROWS FETCH NEXT 5 ROWS ONLY;
SET STATISTICS XML OFF;

-- Oracle（两条语句先后执行）
EXPLAIN PLAN FOR
SELECT p.name, SUM(oi.quantity) AS sold
FROM products p JOIN order_items oi ON oi.product_id = p.id
GROUP BY p.name ORDER BY sold DESC FETCH FIRST 5 ROWS ONLY;

SELECT * FROM TABLE(DBMS_XPLAN.DISPLAY(NULL, NULL, 'BASIC +COST'));
```

---

## 7. 建议的验证顺序

1. 先用第 1 节确认基本的结果网格、排序、`LIMIT` 都对。
2. 用第 2 节看 JOIN / HAVING / 空结果集的渲染（2.1 把阈值改成 3 就是空集）。
3. 用第 3 节看窗口函数、CTE、较大结果集（3.2 的 85 行适合看滚动与列宽）。
4. 用第 4 节在同一套数据上对比四种方言，确认解析器没有把某家的语法错判成另一家。
5. 用第 5 节验证写路径的影响行数与 `RETURNING` / `OUTPUT` 结果面，测完执行清理语句。
6. 用第 6 节验证执行计划面板（PostgreSQL 的计划是文本行，Oracle 需要第二条语句读 `PLAN_TABLE`）。

> 数值提示：四个套件是**各自独立灌数**的，所以同一条查询在各引擎上的金额/排名会不同
> （订单数、用户数、商品数一致）。这是正常的，不要据此判断查询写错。
