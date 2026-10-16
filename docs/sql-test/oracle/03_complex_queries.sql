-- =====================================================================
-- SQLKit Oracle 压测 3/3 —— 复杂查询演练场
-- 前置: 01_schema.sql + 02_seed.sql 已执行
-- 用途: 在 SQLKit 的查询编辑器里逐段执行,验证执行/结果集/耗时/报错
--      路径覆盖: 递归 CTE、CONNECT BY、窗口函数、CROSS APPLY、聚合集、
--               PIVOT/UNPIVOT、MERGE、事务+保存点、动态 SQL、EXPLAIN PLAN、
--               JSON 函数、集合操作、分区路由
-- 每一段都用 banner 注释分隔,支持单独执行
-- 注意: 全文件只有 I 段是"故意报错"（CHECK 约束冲突 + SAVEPOINT 救回），
--       其余各段都必须干净执行；详见 README
-- =====================================================================

-- ── A. 多 CTE 商业报表: 周复购 + 客单价分位 ────────────────────────
WITH weekly AS (
    SELECT o.user_id,
           TRUNC(o.order_date, 'IW') AS wk,
           COUNT(*)                AS orders,
           SUM(o.total_amount)     AS spend
    FROM orders o
    WHERE o.status IN ('paid', 'completed', 'shipped')
    GROUP BY o.user_id, TRUNC(o.order_date, 'IW')
),
repurchase AS (
    SELECT user_id,
           COUNT(DISTINCT wk) AS active_weeks,
           SUM(orders)        AS total_orders,
           SUM(spend)         AS total_spend
    FROM weekly GROUP BY user_id
)
SELECT r.user_id,
       u.username,
       r.active_weeks,
       r.total_orders,
       ROUND(r.total_spend, 2)                                AS total_spend,
       PERCENT_RANK() OVER (ORDER BY r.total_spend)            AS spend_pct,
       CASE WHEN r.total_spend > 4000 THEN 'whale'
            WHEN r.total_spend > 1000 THEN 'big'
            ELSE 'small' END                                  AS segment
FROM repurchase r
JOIN users u ON u.id = r.user_id
ORDER BY r.total_spend DESC
FETCH FIRST 25 ROWS ONLY;

-- ── B. 分类树: CONNECT BY 层次查询 + 递归 WITH（两种写法对照） ─────
-- B1: Oracle 原生层次查询（SYS_CONNECT_BY_PATH / CONNECT_BY_ISLEAF）
SELECT t.id,
       t.path,
       NVL(COUNT(p.id), 0)     AS product_cnt,
       NVL(SUM(p.stock), 0)    AS stock_sum
FROM (
    SELECT c.id,
           SYS_CONNECT_BY_PATH(c.name, ' → ') AS path
    FROM categories c
    START WITH c.parent_id IS NULL
    CONNECT BY PRIOR c.id = c.parent_id
) t
LEFT JOIN products p ON p.category_id = t.id
GROUP BY t.id, t.path
ORDER BY t.path;

-- B2: 12c+ 递归子查询因子（等价写法，PG WITH RECURSIVE 的对应）
WITH tree (id, parent_id, name, lvl, path) AS (
    SELECT c.id, c.parent_id, c.name, 0, c.name
    FROM categories c WHERE c.parent_id IS NULL
    UNION ALL
    SELECT c.id, c.parent_id, c.name, t.lvl + 1, t.path || ' → ' || c.name
    FROM categories c JOIN tree t ON c.parent_id = t.id
)
SELECT t.id, t.lvl, t.path, NVL(COUNT(p.id), 0) AS product_cnt
FROM tree t
LEFT JOIN products p ON p.category_id = t.id
GROUP BY t.id, t.lvl, t.path
ORDER BY t.path;

-- ── C. CROSS APPLY: 每商户 Top-3 销量商品 + 平滑排名 ───────────────
SELECT m.name AS merchant,
       top.product_id,
       top.name,
       top.sold,
       RANK() OVER (PARTITION BY m.id ORDER BY top.sold DESC) AS rnk
FROM merchants m
CROSS APPLY (
    SELECT p.id AS product_id, p.name,
           SUM(oi.quantity) AS sold
    FROM products p
    JOIN order_items oi ON oi.product_id = p.id
    WHERE p.merchant_id = m.id
    GROUP BY p.id, p.name
    ORDER BY SUM(oi.quantity) DESC, p.id
    FETCH FIRST 3 ROWS ONLY
) top
ORDER BY m.name, rnk;

-- ── D. 窗口函数全家桶: 分区移动/滑动平均/前后值 ────────────────────
SELECT o.order_date,
       o.id,
       o.total_amount,
       SUM(o.total_amount) OVER (PARTITION BY o.order_date)                AS day_total,
       ROUND(AVG(o.total_amount) OVER (PARTITION BY o.user_id
             ORDER BY o.order_date
             ROWS BETWEEN 2 PRECEDING AND CURRENT ROW), 2)                 AS moving_avg3,
       LAG(o.total_amount)  OVER (PARTITION BY o.user_id ORDER BY o.order_date) AS prev_amount,
       LEAD(o.total_amount) OVER (PARTITION BY o.user_id ORDER BY o.order_date) AS next_amount,
       FIRST_VALUE(o.status) OVER (PARTITION BY o.user_id ORDER BY o.order_date) AS first_status,
       COUNT(*) OVER (PARTITION BY o.user_id)                              AS user_orders
FROM orders o
ORDER BY o.user_id, o.order_date
FETCH FIRST 60 ROWS ONLY;

-- ── E. 聚合集 + 行列转置: GROUPING SETS / ROLLUP / CUBE / PIVOT / UNPIVOT ──
SELECT NVL(o.status, 'ALL')                                    AS status,
       NVL(TO_CHAR(TRUNC(o.order_date, 'MM'), 'YYYY-MM'), 'ALL') AS ym,
       COUNT(*) FILTER (WHERE o.total_amount > 500)             AS big_orders,
       SUM(o.total_amount) FILTER (WHERE o.status = 'completed') AS completed_sum,
       COUNT(DISTINCT o.user_id)                               AS buyers
FROM orders o
GROUP BY GROUPING SETS ((o.status), (TRUNC(o.order_date, 'MM')),
                        (o.status, TRUNC(o.order_date, 'MM')), ())
ORDER BY ym NULLS LAST;

SELECT o.status,
       TRUNC(o.order_date, 'MM') AS ym,
       SUM(o.total_amount)       AS total,
       GROUPING_ID(o.status, TRUNC(o.order_date, 'MM')) AS gid
FROM orders o
GROUP BY ROLLUP(o.status, TRUNC(o.order_date, 'MM'))
ORDER BY gid, status, ym;

SELECT o.status,
       o.shipping_fee,
       COUNT(*) AS order_cnt,
       SUM(o.total_amount) AS total
FROM orders o
GROUP BY CUBE(o.status, o.shipping_fee)
ORDER BY o.status, o.shipping_fee;

SELECT *
FROM (
    SELECT o.status,
           TO_CHAR(TRUNC(o.order_date, 'MM'), 'MM') AS mm,
           o.total_amount
    FROM orders o
)
PIVOT (SUM(total_amount) AS amt, COUNT(*) AS cnt FOR mm IN ('01' AS m01, '02' AS m02, '03' AS m03))
ORDER BY status;

SELECT status, mm, amt
FROM (
    SELECT *
    FROM (
        SELECT o.status,
               TO_CHAR(TRUNC(o.order_date, 'MM'), 'MM') AS mm,
               o.total_amount
        FROM orders o
    )
    PIVOT (SUM(total_amount) AS amt FOR mm IN ('01' AS m01, '02' AS m02, '03' AS m03))
)
UNPIVOT (amt FOR mm IN (m01_amt AS '01', m02_amt AS '02', m03_amt AS '03'))
ORDER BY status, mm;

-- ── F. JSON 类型演练: JSON_VALUE / JSON_QUERY / JSON_EXISTS / JSON_TABLE ──
SELECT u.username,
       JSON_VALUE(u.attrs, '$.city')                         AS city,
       JSON_VALUE(u.attrs, '$.vip')                          AS is_vip,
       JSON_QUERY(u.attrs, '$.profile')                      AS profile,
       JSON_VALUE(u.tags, '$[0]')                            AS first_tag,
       JSON_EXISTS(u.tags, '$[*]?(@ == "reader")')           AS is_reader,
       u.balance                                             AS wallet
FROM users u
WHERE JSON_VALUE(u.attrs, '$.vip') = 'true'
   OR JSON_EXISTS(u.tags, '$[*]?(@ == "reader")')
ORDER BY u.balance DESC
FETCH FIRST 20 ROWS ONLY;

SELECT jt.origin,
       jt.warranty_month,
       COUNT(*)                AS product_cnt,
       ROUND(AVG(p.price), 2)  AS avg_price
FROM products p,
     JSON_TABLE(p.specs, '$' COLUMNS (
         origin         VARCHAR2(20) PATH '$.origin',
         warranty_month NUMBER       PATH '$.warranty_month')) jt
GROUP BY jt.origin, jt.warranty_month
ORDER BY jt.origin, jt.warranty_month;

-- ── G. UPSERT: MERGE（PG ON CONFLICT 的 Oracle 写法） ──────────────
-- 23ai 探针结论: INSERT ... ON DUPLICATE KEY UPDATE 不被支持
-- （ORA-03048: SQL reserved word 'ON' ...），Oracle 一律用 MERGE。
-- G1: 存在即忽略（对应 ON CONFLICT DO NOTHING）
MERGE INTO reservations r
USING (SELECT 1 AS resource_id, TIMESTAMP '2026-07-01 00:00:00' AS valid_from FROM dual) s
ON (r.resource_id = s.resource_id AND r.valid_from = s.valid_from)
WHEN NOT MATCHED THEN
    INSERT (resource_id, valid_from, valid_to, note)
    VALUES (s.resource_id, s.valid_from, TIMESTAMP '2026-07-05 00:00:00', money_amount(500, 'CNY'));

SELECT id, resource_id, valid_from, valid_to FROM reservations ORDER BY resource_id, valid_from;

-- G2: 存在即取较大折扣（对应 ON CONFLICT (code) DO UPDATE）
MERGE INTO coupons c
USING (SELECT 'SUMMER2026' AS code, 30.00 AS discount, 200 AS min_amount,
              TIMESTAMP '2026-08-31 00:00:00 +08:00' AS valid_until FROM dual) s
ON (c.code = s.code)
WHEN MATCHED THEN
    UPDATE SET c.discount = GREATEST(c.discount, s.discount)
WHEN NOT MATCHED THEN
    INSERT (code, discount, min_amount, valid_until)
    VALUES (s.code, s.discount, s.min_amount, s.valid_until);

SELECT id, code, discount, min_amount, claimed_by FROM coupons WHERE code = 'SUMMER2026';

-- ── H. MERGE 库存对账（按 sku 聚合差异后合并） ────────────────────
DELETE FROM stock_diff;

INSERT INTO stock_diff (sku, delta)
SELECT 'SKU-0001', -3 FROM dual
UNION ALL SELECT 'SKU-0002', 5 FROM dual
UNION ALL SELECT 'SKU-0001', 1 FROM dual
UNION ALL SELECT 'SKU-NEW-99', 2 FROM dual;

MERGE INTO products p
USING (SELECT d.sku,
              SUM(d.delta) AS delta,
              (SELECT MIN(id) FROM merchants) AS merchant_id
       FROM stock_diff d
       GROUP BY d.sku) s
ON (p.sku = s.sku)
WHEN MATCHED THEN
    UPDATE SET p.stock = GREATEST(0, p.stock + s.delta),
               p.updated_at = SYSTIMESTAMP
WHEN NOT MATCHED THEN
    INSERT (merchant_id, category_id, sku, name, condition, price, stock)
    VALUES (s.merchant_id, NULL, s.sku, '自动对账商品-' || s.sku, 'new', 9.90, GREATEST(0, s.delta));

SELECT id, sku, name, stock, price, price_level
FROM products
WHERE sku IN ('SKU-0001', 'SKU-0002', 'SKU-NEW-99')
ORDER BY sku;

-- ── I. 事务 + SAVEPOINT（本节含 03 中唯一一处"故意报错"） ──────────
-- Oracle 报错不会像 PG 那样把事务置为 aborted 状态：报错语句本身不留痕迹，
-- ROLLBACK TO SAVEPOINT 只回退该段，之后的 UPDATE 依然生效并提交。
--
-- 注意执行方式：SQLKit 的 Oracle 通道经 JDBC 桥「逐条自动提交」，SAVEPOINT
-- 在上一条语句提交后即失效（会报 ORA-01086）。因此本节改为调用 01_schema.sql
-- 里建好的 demo_savepoint_rescue 过程 —— 一次 CALL 即可跑出完整事务语义，
-- 过程内的预期错误（ORA-02290）被异常处理器吞掉，界面上不出现噪声报错。
-- 参考：等价的原生脚本写法（仅在 SQL*Plus 整段执行时可用）
--   DELETE FROM users WHERE username = 'tx_probe';
--   INSERT INTO users (username, email, password_hash) VALUES (...);
--   SAVEPOINT before_bad_insert;
--   INSERT INTO reservations (...) VALUES (1, '2026-06-05', '2026-06-03', ...);  -- ORA-02290
--   ROLLBACK TO SAVEPOINT before_bad_insert;
--   UPDATE users SET balance = balance + 100 WHERE username = 'tx_probe';
--   COMMIT;
CALL demo_savepoint_rescue();

SELECT username, balance, email FROM users WHERE username = 'tx_probe';

-- ── J. 动态 SQL 函数调用（对应 PG 的 DO 块 + 动态 SQL） ────────────
SELECT stale_paid_orders(30) AS stale_paid_30d,
       stale_paid_orders(60) AS stale_paid_60d,
       coupon_pkg.claimed_count() AS claimed_coupons
FROM dual;

-- ── K. CALL 存储过程（包过程 + 独立过程；无匿名块，分隔器友好） ────
CALL coupon_pkg.claim_coupon('100010', 42);

CALL void_order('SO-20260100017');

SELECT code, claimed_by, claimed_at FROM coupons WHERE code = '100010';

SELECT order_no, order_date, status FROM orders WHERE order_no = 'SO-20260100017';

-- ── L. 集合操作与反连接: FULL OUTER / INTERSECT / MINUS ────────────
-- Oracle 没有 IS DISTINCT FROM，用 NVL 归一化后比较
SELECT NVL(m1.merchant_id, m2.merchant_id) AS merchant_id,
       m1.active_cnt AS via_active_products,
       m2.all_cnt    AS via_products
FROM (SELECT merchant_id, COUNT(*) AS active_cnt
      FROM active_products GROUP BY merchant_id) m1
FULL OUTER JOIN
     (SELECT merchant_id, COUNT(*) AS all_cnt
      FROM products GROUP BY merchant_id) m2
  ON m1.merchant_id = m2.merchant_id
WHERE NVL(m1.active_cnt, -1) <> NVL(m2.all_cnt, -1);

-- 交集: 1 月买过、3 月也买过的用户
SELECT u.username
FROM users u
WHERE u.id IN (SELECT o.user_id FROM orders o
               WHERE o.order_date >= DATE '2026-01-01' AND o.order_date < DATE '2026-02-01')
INTERSECT
SELECT u.username
FROM users u
WHERE u.id IN (SELECT o.user_id FROM orders o
               WHERE o.order_date >= DATE '2026-03-01' AND o.order_date < DATE '2026-04-01');

-- 差集（PG 的 EXCEPT = Oracle 的 MINUS）: 1 月买过但 2 月没买的用户
SELECT u.username
FROM users u
WHERE u.id IN (SELECT o.user_id FROM orders o
               WHERE o.order_date >= DATE '2026-01-01' AND o.order_date < DATE '2026-02-01')
MINUS
SELECT u.username
FROM users u
WHERE u.id IN (SELECT o.user_id FROM orders o
               WHERE o.order_date >= DATE '2026-02-01' AND o.order_date < DATE '2026-03-01');

-- ── M. 去重与分页: ROW_NUMBER / KEEP DENSE_RANK / FETCH FIRST ──────
SELECT category_id, id, name, price
FROM (
    SELECT p.category_id, p.id, p.name, p.price,
           ROW_NUMBER() OVER (PARTITION BY p.category_id
                              ORDER BY p.created_at DESC, p.id DESC) AS rn
    FROM products p
    WHERE p.category_id IS NOT NULL
) v
WHERE v.rn = 1
ORDER BY category_id;

SELECT p.category_id,
       MAX(p.name)  KEEP (DENSE_RANK LAST  ORDER BY p.price, p.id) AS priciest_name,
       MAX(p.price) KEEP (DENSE_RANK LAST  ORDER BY p.price, p.id) AS priciest_price,
       MIN(p.price) KEEP (DENSE_RANK FIRST ORDER BY p.price, p.id) AS cheapest_price
FROM products p
WHERE p.category_id IS NOT NULL
GROUP BY p.category_id
ORDER BY p.category_id;

SELECT id, order_no, order_date, total_amount
FROM orders
ORDER BY order_date DESC, id DESC
FETCH FIRST 10 ROWS ONLY;

SELECT id, order_no, order_date, total_amount
FROM orders
ORDER BY order_date DESC, id DESC
OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;

-- ── N. JSON 聚合: 订单地址树构建 + JSON_ARRAYAGG / JSON_OBJECTAGG ──
SELECT JSON_VALUE(o.shipping_addr, '$.province') AS province,
       JSON_ARRAYAGG(JSON_OBJECT('order' VALUE o.order_no, 'amount' VALUE o.total_amount)
                     ORDER BY o.total_amount DESC RETURNING CLOB) AS orders_bundle,
       MAX(o.total_amount) AS biggest
FROM orders o
WHERE JSON_VALUE(o.shipping_addr, '$.city') = '杭州'
GROUP BY JSON_VALUE(o.shipping_addr, '$.province');

SELECT JSON_OBJECTAGG(order_no VALUE total_amount) AS amount_map
FROM (
    SELECT order_no, total_amount
    FROM orders
    WHERE status = 'completed'
    ORDER BY total_amount DESC
    FETCH FIRST 5 ROWS ONLY
);

-- ── O. EXPLAIN PLAN + DBMS_XPLAN.DISPLAY（计划结果面） ─────────────
-- 两条语句必须同一会话连着跑：EXPLAIN PLAN 写入 PLAN_TABLE，
-- 紧接着的 DBMS_XPLAN.DISPLAY() 读取它。
EXPLAIN PLAN FOR
SELECT p.name, oi.quantity
FROM products p
JOIN order_items oi ON oi.product_id = p.id
WHERE LOWER(p.sku) LIKE '%sku-01%'
ORDER BY oi.quantity DESC
FETCH FIRST 10 ROWS ONLY;

SELECT * FROM TABLE(DBMS_XPLAN.DISPLAY());

-- 分区裁剪验证
EXPLAIN PLAN FOR
SELECT * FROM orders WHERE order_date = DATE '2026-02-10';

SELECT * FROM TABLE(DBMS_XPLAN.DISPLAY());

-- ── P. 关联子查询 UPDATE / DELETE（Oracle 无 UPDATE ... FROM 之外的写法） ──
-- 说明: 23ai 其实已支持 UPDATE ... FROM，但这里用关联子查询保持通用
UPDATE reviews r
   SET r.content = r.content || ' [reviewed]'
 WHERE EXISTS (SELECT 1
               FROM users u
               WHERE u.id = r.user_id
                 AND JSON_VALUE(u.attrs, '$.vip') = 'true');

SELECT COUNT(*) AS reviewed_rows FROM reviews WHERE content LIKE '%[reviewed]%';

DELETE FROM reviews r
 WHERE EXISTS (SELECT 1
               FROM payments pm
               WHERE pm.id = r.payment_id
                 AND pm.status = 'refunded');

DELETE FROM reviews r
 WHERE EXISTS (SELECT 1
               FROM products p
               WHERE p.id = r.product_id
                 AND p.is_active = FALSE);

-- 保留策略清理: 删掉 2026-01 完成订单对应的评价（关联子查询穿透 payment → order）
DELETE FROM reviews r
 WHERE EXISTS (SELECT 1
               FROM payments pm
               JOIN orders o ON o.id = pm.order_id AND o.order_date = pm.order_date
               WHERE pm.id = r.payment_id
                 AND o.order_date < DATE '2026-02-01');

SELECT COUNT(*) AS reviews_left FROM reviews;

-- ── Q. deferrable 约束演示（同一事务内互换邮箱） ───────────────────
-- users_email_unique 建为 DEFERRABLE INITIALLY DEFERRED：约束在 COMMIT 时才校验，
-- 因此两条 UPDATE 中途出现瞬时重复是允许的 —— 这正是与 MySQL/SQL Server
-- 「唯一键即时校验」的关键差异（那两个引擎必须借道临时值走三步）。
--
-- 同样封进存储过程：SQLKit 逐条自动提交会把第一条 UPDATE 立刻提交并撞约束。
-- 这里 CALL 两个过程：先互换、再还原，保证本节可反复执行。
CALL demo_deferrable_swap();

SELECT username, email FROM users WHERE username IN ('user_1', 'user_2') ORDER BY username;

CALL demo_deferrable_restore();

SELECT username, email FROM users WHERE username IN ('user_1', 'user_2') ORDER BY username;

-- ── R. 函数调用三途径（含同名重载签名验证） ────────────────────────
SELECT get_product_rank(1) AS units_by_id,
       get_product_revenue(1) AS revenue_by_id
FROM dual;

SELECT get_product_rank('SKU-0001') AS units_by_sku FROM dual;

SELECT analytics_pkg.product_units(1) AS pkg_units,
       analytics_pkg.product_revenue(1) AS pkg_revenue
FROM dual;

-- ── S. 窗口框架 RANGE 区间（物化视图上的 7 日滚动收入） ────────────
SELECT d.sales_day,
       d.revenue,
       SUM(d.revenue) OVER (ORDER BY d.sales_day
            RANGE BETWEEN INTERVAL '7' DAY PRECEDING AND CURRENT ROW) AS rev_7d,
       ROUND(AVG(d.revenue) OVER (ORDER BY d.sales_day
            ROWS BETWEEN 6 PRECEDING AND CURRENT ROW), 2) AS avg_7rows
FROM analytics_daily_sales d
ORDER BY d.sales_day;

-- ── T. 分区路由: 未来日期触发 INTERVAL 自动建分区 + 分区元数据 ─────
-- 注意: 自动创建分区是 DDL，会隐式提交（Oracle 与 PG 的差别之一），
-- 所以这里演示完用 DELETE 清掉演示行，而不是依赖 ROLLBACK。
INSERT INTO orders (order_no, user_id, status, order_date, shipping_addr, remarks)
SELECT 'SO-2026FUTURE', MIN(u.id), 'draft', DATE '2026-12-25',
       JSON_OBJECT('city' VALUE 'default'), '间隔分区自动创建演示'
FROM users u;

SELECT partition_name, high_value, interval
FROM user_tab_partitions
WHERE table_name = 'ORDERS'
ORDER BY partition_position;

SELECT id, order_no, order_date, status FROM orders WHERE order_no = 'SO-2026FUTURE';

DELETE FROM orders WHERE order_no = 'SO-2026FUTURE';

COMMIT;

-- 完成。清理残余演示数据:
DELETE FROM users WHERE username = 'tx_probe';

COMMIT;

-- T 段的分区 DDL 会把物化视图打成 INVALID，这里刷新一次让它回到 VALID 并同步数据:
CALL refresh_daily_sales();
