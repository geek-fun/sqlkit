-- =====================================================================
-- SQLKit PostgreSQL 压测 3/3 —— 复杂查询演练场
-- 前置: 01_schema.sql + 02_seed.sql 已执行
-- 用途: 在 SQLKit 的查询编辑器里逐段执行,验证执行/结果集/耗时/报错
--      路径覆盖: 递归 CTE、窗口函数、横竖向连接、聚合集、UPSERT、
--               MERGE、事务+保存点、DO 块、EXPLAIN、JSONB、数组、分区裁剪
-- 每一段都用注释分隔,支持单独执行
-- =====================================================================

-- ── A. 多 CTE 商业报表: 周复购 + 客单价分位 ────────────────────────
WITH weekly AS (
    SELECT o.user_id,
           date_trunc('week', o.order_date) AS wk,
           count(*)               AS orders,
           sum(o.total_amount)    AS spend
    FROM store.orders o
    WHERE o.status IN ('paid', 'completed', 'shipped')
    GROUP BY 1, 2
),
repurchase AS (
    SELECT user_id,
           count(DISTINCT wk) AS active_weeks,
           sum(orders)        AS total_orders,
           sum(spend)         AS total_spend
    FROM weekly GROUP BY user_id
)
SELECT r.user_id,
       u.username,
       r.active_weeks,
       r.total_orders,
       round(r.total_spend, 2)                                   AS total_spend,
       percent_rank() OVER (ORDER BY r.total_spend)               AS spend_pct,
       CASE WHEN r.total_spend > 4000 THEN 'whale'
            WHEN r.total_spend > 1000 THEN 'big'
            ELSE 'small' END                                      AS segment
FROM repurchase r
JOIN store.users u ON u.id = r.user_id
ORDER BY r.total_spend DESC
LIMIT 25;

-- ── B. WITH RECURSIVE 分类树（自引用） ─────────────────────────────
WITH RECURSIVE tree AS (
    SELECT c.id, c.parent_id, c.name, 0 AS level,
           c.name::text AS path
    FROM store.categories c WHERE c.parent_id IS NULL
  UNION ALL
    SELECT c.id, c.parent_id, c.name, t.level + 1,
           t.path || ' → ' || c.name
    FROM store.categories c JOIN tree t ON c.parent_id = t.id
)
SELECT t.id, t.path,
       coalesce(count(p.id), 0) AS product_cnt,
       coalesce(sum(p.stock), 0) AS stock_sum
FROM tree t
LEFT JOIN store.products p ON p.category_id = t.id
GROUP BY t.id, t.path
ORDER BY t.path;

-- ── C. LATERAL: 每商户 Top-3 销量商品 + 平滑排名 ───────────────────
SELECT m.name AS merchant,
       top.product_id,
       top.name,
       top.sold,
       rank() OVER (PARTITION BY m.id ORDER BY top.sold DESC) AS rnk
FROM store.merchants m
JOIN LATERAL (
    SELECT p.id AS product_id, p.name,
           sum(oi.quantity) AS sold
    FROM store.products p
    JOIN store.order_items oi ON oi.product_id = p.id
    WHERE p.merchant_id = m.id
    GROUP BY p.id, p.name
    ORDER BY sum(oi.quantity) DESC
    LIMIT 3
) top ON true
ORDER BY m.name, rnk;

-- ── D. 窗口函数全家桶: 分区移动/滑动平均/间隙滞后 ──────────────────
SELECT o.order_date,
       o.id,
       o.total_amount,
       sum(o.total_amount) OVER (PARTITION BY order_date)                 AS day_total,
       round(avg(o.total_amount) OVER (PARTITION BY user_id
           ORDER BY o.order_date
           ROWS BETWEEN 2 PRECEDING AND CURRENT ROW), 2)                  AS moving_avg3,
       lag(o.total_amount) OVER (PARTITION BY user_id ORDER BY o.order_date)  AS prev_amount,
       lead(o.total_amount) OVER (PARTITION BY user_id ORDER BY o.order_date) AS next_amount,
       first_value(o.status) OVER (PARTITION BY user_id ORDER BY o.order_date),
       count(*) OVER (PARTITION BY o.user_id)                             AS user_orders
FROM store.orders o
ORDER BY o.user_id, o.order_date
LIMIT 60;

-- ── E. GROUPING SETS / CUBE / ROLLUP + FILTER 子句 ─────────────────
SELECT coalesce(o.status::text, 'ALL')                    AS status,
       coalesce(date_trunc('month', o.order_date)::text, 'ALL') AS ym,
       count(*)                                  FILTER (WHERE o.total_amount > 500) AS big_orders,
       sum(o.total_amount)                       FILTER (WHERE o.status = 'completed') AS completed_sum,
       count(DISTINCT o.user_id)                 AS buyers
FROM store.orders o
GROUP BY GROUPING SETS ((o.status), (date_trunc('month', o.order_date)), (o.status, date_trunc('month', o.order_date)), ())
ORDER BY ym NULLS LAST;

-- ── F. JSONB / 数组 / 范围 / 网络 类型演练 ─────────────────────────
SELECT u.username,
       u.attrs ->> 'city'                          AS city,
       u.attrs @> '{"vip": true}'                  AS is_vip,
       coalesce(array_length(u.tags, 1), 0)        AS tag_cnt,
       u.tags[1]                                   AS first_tag,
       u.signup_ip << '10.0.128.0/17'::cidr        AS big_net,
       u.balance                                   AS wallet
FROM store.users u
WHERE u.attrs @> '{"vip": true}'
   OR u.tags && ARRAY['reader']
ORDER BY u.balance DESC
LIMIT 20;

-- ── G. UPSERT: 幂等敏感的 ON CONFLICT ─────────────────────────────
INSERT INTO store.reservations (resource_id, valid, note) VALUES
  (1, '[2026-07-01, 2026-07-05)', (500, 'CNY')::money_amount)
ON CONFLICT ON CONSTRAINT reservations_no_overlap DO NOTHING;

INSERT INTO store.coupons (code, discount, min_amount, valid_until)
VALUES ('SUMMER2026', 30.00, 200, '2026-08-31')
ON CONFLICT (code) DO UPDATE
   SET discount = greatest(store.coupons.discount, EXCLUDED.discount)
RETURNING id, code, discount, (xmax = 0) AS inserted;

-- ── H. MERGE (PG15+): 库存对账合并 ────────────────────────────────
-- ON COMMIT DROP 的 TEMP 表必须与 MERGE 同事务（autocommit 会在语句末直接删表）
BEGIN;
CREATE TEMP TABLE stock_diff (product_id bigint, delta int) ON COMMIT DROP;
INSERT INTO stock_diff VALUES (1, -3), (2, 5), (1, 1), (99, 2);

MERGE INTO store.products AS p
USING (SELECT product_id, sum(delta) AS delta FROM stock_diff GROUP BY product_id) AS s
ON p.id = s.product_id
WHEN MATCHED THEN
    UPDATE SET stock = p.stock + s.delta,
               updated_at = now()
WHEN NOT MATCHED THEN
    -- identity 列不接受显式 id;新行自增 (对账 id 通过 sku 关联)
    INSERT (merchant_id, sku, name, price, stock)
    VALUES (1, 'SKU-NEW-' || s.product_id,
            '自动对账商品-' || s.product_id, 9.90,
            GREATEST(0, s.delta));
COMMIT;

SELECT id, sku, name, stock FROM store.products WHERE id IN (1, 2, 99) ORDER BY id;

-- ── I. 事务 + 保存点回滚（嵌套事务语义） ──────────────────────────
BEGIN;
    INSERT INTO store.users (username, email, password_hash)
    VALUES ('tx_probe', 'txprobe@example.com', md5('x'));
    SAVEPOINT before_bad_insert;
    -- 这一步会因 EXCLUDE 时段互斥冲突而报错;ROLLBACK TO 会把事务从
    -- aborted 状态救回 —— 正好演练保存点:
    INSERT INTO store.reservations (resource_id, valid, note)
    VALUES (1, '[2026-06-03, 2026-06-05)', (10, 'CNY')::money_amount);
    ROLLBACK TO SAVEPOINT before_bad_insert;   -- 回滚并解除 aborted
    UPDATE store.users SET balance = balance + 100 WHERE username = 'tx_probe';
COMMIT;

SELECT username, balance FROM store.users WHERE username = 'tx_probe';

-- ── J. DO 匿名块 + 动态 SQL ───────────────────────────────────────
DO $$
DECLARE
    stale_count int;
BEGIN
    SELECT count(*) INTO stale_count
    FROM store.orders o
    WHERE o.status = 'paid'
      AND o.placed_at < now() - interval '30 days';
    RAISE NOTICE '超期未发货订单: %', stale_count;
END;
$$;

-- ── K. CALL 存储过程（procedures 面板/CALL 语句） ─────────────────
CALL store.claim_coupon('100010', 42);
CALL store.void_order('SO-20260100017');
SELECT code, claimed_by FROM store.coupons WHERE code = '100010';
SELECT status FROM store.orders WHERE order_no = 'SO-20260100017';

-- ── L. 集合操作与反连接: FULL OUTER / INTERSECT / EXCEPT ──────────
SELECT coalesce(m1.merchant_id, m2.merchant_id) AS merchant_id,
       m1.active_cnt AS via_active_products,
       m2.all_cnt    AS via_products
FROM (SELECT merchant_id, count(*) active_cnt
      FROM store.active_products GROUP BY merchant_id) m1
FULL OUTER JOIN
     (SELECT merchant_id, count(*) all_cnt
      FROM store.products GROUP BY merchant_id) m2
  ON m1.merchant_id = m2.merchant_id
WHERE m1.active_cnt IS DISTINCT FROM m2.all_cnt;

SELECT p.name FROM store.products p
WHERE p.price > 1000
INTERSECT
SELECT p.name FROM store.products p WHERE p.stock < 100;

SELECT username FROM store.users
WHERE is_active
EXCEPT
SELECT username FROM store.users WHERE attrs @> '{"vip": true}';

-- ── M. DISTINCT ON: 每类目最新商品 ────────────────────────────────
SELECT DISTINCT ON (category_id) category_id, id, name, price
FROM store.products
WHERE category_id IS NOT NULL
ORDER BY category_id, created_at DESC NULLS LAST;

-- ── N. jsonb_head 深度: 订单地址树构建 + 聚合 ─────────────────────
SELECT o.shipping_addr ->> 'province' AS province,
       jsonb_agg(jsonb_build_object('order', o.order_no, 'amount', o.total_amount)
                 ORDER BY o.total_amount DESC) AS orders_bundle,
       max(o.total_amount) AS biggest
FROM store.orders o
WHERE o.shipping_addr ->> 'city' = '杭州'
GROUP BY 1;

-- ── O. EXPLAIN: 查询计划（测试 rows/args 结果面） ─────────────────
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT p.name, oi.quantity
FROM store.products p
JOIN store.order_items oi ON oi.product_id = p.id
WHERE lower(p.sku) LIKE '%sku-01%'
ORDER BY oi.quantity DESC
LIMIT 10;

EXPLAIN (ANALYZE, FORMAT TEXT)
SELECT * FROM store.orders WHERE order_date = '2026-02-10'; -- 分区裁剪验证

-- ── P. UPDATE FROM + DELETE USING（复合 join 修改） ────────────────
UPDATE store.reviews r
SET content = r.content || ' [reviewed]'
FROM store.users u
WHERE r.user_id = u.id AND u.attrs @> '{"vip": true}'
RETURNING r.id, r.content;

DELETE FROM store.reviews r
USING store.payments pm
WHERE r.payment_id = pm.id
  AND pm.status = 'refunded'
RETURNING r.id;

-- ── Q. deferrable 约束演示（同一事务改邮箱冲突交换） ──────────────
BEGIN;
-- 先制造瞬时冲突: 把 user_1 的邮箱改成 user_2 正在用的那个
UPDATE store.users SET email = 'user2@mail.example.com' WHERE username = 'user_1';
UPDATE store.users SET email = 'user1@mail.example.com' WHERE username = 'user_2';
COMMIT; -- 延迟约束在提交时统一校验,中途互相冲突也放行

-- 还原邮箱(普通事务)
BEGIN;
UPDATE store.users SET email = 'user1@mail.example.com' WHERE username = 'user_1';
UPDATE store.users SET email = 'user2@mail.example.com' WHERE username = 'user_2';
COMMIT;

-- ── R. 大函数调用: 三种途径验证 functions 面板签名 ─────────────────
SELECT * FROM store.get_product_rank(1::int);
SELECT * FROM store.get_product_rank(1::bigint);
SELECT * FROM analytics.get_product_rank(5);

-- ── S. 窗口框架 RANGE 与 ORDER BY NULLS ───────────────────────────
SELECT d.sales_day, d.revenue,
       sum(d.revenue) OVER (ORDER BY d.sales_day
           RANGE BETWEEN INTERVAL '7 days' PRECEDING AND CURRENT ROW) AS rev_7d
FROM analytics.daily_sales d
ORDER BY d.sales_day;

-- ── T. 分区 Default 兜底路由 + 回滚的空操作分区路由测试 ────────────
BEGIN;
INSERT INTO store.orders (order_no, user_id, order_date, status, shipping_addr)
VALUES ('SO-2026FUTURE', 1, '2026-12-25', 'draft',
        '{"city":"default"}'::jsonb)   -- 落到 orders_default 分区
RETURNING tableoid::regclass; -- 验证路由
ROLLBACK;

-- 完成。清理残余演示数据:
DELETE FROM store.users WHERE username IN ('tx_probe', 'user_na');
