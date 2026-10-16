-- =====================================================================
-- SQLKit MySQL 压测 3/3 —— 复杂查询演练场
-- 前置: 01_schema.sql + 02_seed.sql 已执行
-- 用途: 在 SQLKit 的查询编辑器里逐段执行,验证执行/结果集/耗时/报错
--      路径覆盖: 递归 CTE、窗口函数、横向连接、聚合集、UPSERT、
--               JSON_TABLE、全文检索、分区裁剪、事务+保存点、EXPLAIN
-- 每一段都用注释分隔,支持单独执行
-- 注意: SQLKit 的 MySQL 适配器把编辑器内容作为「单条语句」下发,
--       请一次选中一段执行（不要整文件执行）
-- =====================================================================

-- ── A. 多 CTE 商业报表: 周复购 + 客单价分位 ────────────────────────
WITH weekly AS (
    SELECT o.user_id,
           DATE_SUB(o.order_date, INTERVAL WEEKDAY(o.order_date) DAY) AS wk,
           COUNT(*)                AS orders,
           SUM(o.total_amount)     AS spend
    FROM `sqlkit-demo`.orders o
    WHERE o.status IN ('paid', 'completed', 'shipped')
    GROUP BY o.user_id, DATE_SUB(o.order_date, INTERVAL WEEKDAY(o.order_date) DAY)
),
repurchase AS (
    SELECT user_id,
           COUNT(DISTINCT wk) AS active_weeks,
           SUM(orders)        AS total_orders,
           SUM(spend)         AS total_spend
    FROM weekly GROUP BY user_id
)
SELECT r.user_id, u.username, r.active_weeks, r.total_orders,
       ROUND(r.total_spend, 2)                        AS total_spend,
       PERCENT_RANK() OVER (ORDER BY r.total_spend)   AS spend_pct,
       CASE WHEN r.total_spend > 4000 THEN 'whale'
            WHEN r.total_spend > 1000 THEN 'big'
            ELSE 'small' END                          AS segment
FROM repurchase r
JOIN `sqlkit-demo`.users u ON u.id = r.user_id
ORDER BY r.total_spend DESC
LIMIT 25;

-- ── B. WITH RECURSIVE 分类树（自引用） ─────────────────────────────
SET SESSION cte_max_recursion_depth = 1000;

WITH RECURSIVE tree AS (
    SELECT c.id, c.parent_id, c.name, 0 AS lvl,
           CAST(c.name AS CHAR(255)) AS path
    FROM `sqlkit-demo`.categories c WHERE c.parent_id IS NULL
  UNION ALL
    SELECT c.id, c.parent_id, c.name, t.lvl + 1,
           CONCAT(t.path, ' → ', c.name)
    FROM `sqlkit-demo`.categories c JOIN tree t ON c.parent_id = t.id
)
SELECT t.id, t.path,
       COALESCE(COUNT(p.id), 0)  AS product_cnt,
       COALESCE(SUM(p.stock), 0) AS stock_sum
FROM tree t
LEFT JOIN `sqlkit-demo`.products p ON p.category_id = t.id
GROUP BY t.id, t.path
ORDER BY t.path;

-- ── C. LATERAL: 每商户 Top-3 销量商品 + 平滑排名 ───────────────────
SELECT m.name AS merchant, top.product_id, top.name, top.sold,
       RANK() OVER (PARTITION BY m.id ORDER BY top.sold DESC) AS rnk
FROM `sqlkit-demo`.merchants m
JOIN LATERAL (
    SELECT p.id AS product_id, p.name, SUM(oi.quantity) AS sold
    FROM `sqlkit-demo`.products p
    JOIN `sqlkit-demo`.order_items oi ON oi.product_id = p.id
    WHERE p.merchant_id = m.id
    GROUP BY p.id, p.name
    ORDER BY SUM(oi.quantity) DESC
    LIMIT 3
) top ON TRUE
ORDER BY m.name, rnk;

-- ── D. 窗口函数全家桶: 分区移动/滑动平均/间隙滞后 ──────────────────
SELECT o.order_date, o.id, o.total_amount,
       SUM(o.total_amount) OVER (PARTITION BY o.order_date)            AS day_total,
       ROUND(AVG(o.total_amount) OVER (PARTITION BY o.user_id
             ORDER BY o.order_date
             ROWS BETWEEN 2 PRECEDING AND CURRENT ROW), 2)             AS moving_avg3,
       LAG(o.total_amount)  OVER (PARTITION BY o.user_id ORDER BY o.order_date) AS prev_amount,
       LEAD(o.total_amount) OVER (PARTITION BY o.user_id ORDER BY o.order_date) AS next_amount,
       FIRST_VALUE(o.status) OVER (PARTITION BY o.user_id ORDER BY o.order_date) AS first_status,
       COUNT(*) OVER (PARTITION BY o.user_id)                          AS user_orders
FROM `sqlkit-demo`.orders o
ORDER BY o.user_id, o.order_date
LIMIT 60;

-- ── E. ROLLUP + 条件聚合（MySQL 无 FILTER 子句，用 CASE 替代） ─────
-- MySQL 限制: 如果把 GROUP BY 的表达式再包一层 COALESCE(...) 放进 SELECT，
-- ONLY_FULL_GROUP_BY 会判定它「非函数依赖」并报 1055。解法是先在 CTE 里聚合，
-- 外层再做 COALESCE / 排序。
WITH agg AS (
    SELECT DATE_FORMAT(o.order_date, '%Y-%m')                     AS ym,
           o.status                                               AS status,
           COUNT(CASE WHEN o.total_amount > 500 THEN 1 END)        AS big_orders,
           SUM(CASE WHEN o.status = 'completed' THEN o.total_amount END) AS completed_sum,
           COUNT(DISTINCT o.user_id)                               AS buyers
    FROM `sqlkit-demo`.orders o
    GROUP BY DATE_FORMAT(o.order_date, '%Y-%m'), o.status WITH ROLLUP
)
SELECT COALESCE(a.status, 'ALL') AS status,
       COALESCE(a.ym, 'ALL')     AS ym,
       a.big_orders, a.completed_sum, a.buyers
FROM agg a
ORDER BY a.status IS NULL, a.ym IS NULL, a.ym, a.status;

-- ── E2. GROUPING() 函数: 区分「真 NULL」与「汇总行」 ───────────────
SELECT o.status,
       GROUPING(o.status) AS is_rollup,
       COUNT(*)           AS cnt
FROM `sqlkit-demo`.orders o
GROUP BY o.status WITH ROLLUP
HAVING GROUPING(o.status) = 1 OR COUNT(*) > 60;

-- ── F. JSON / 二进制 / 空间 类型演练 ──────────────────────────────
SELECT u.username,
       u.attrs ->> '$.city'                        AS city,
       JSON_EXTRACT(u.attrs, '$.vip')              AS is_vip,
       u.attrs -> '$.profile.nick'                 AS nick_quoted,
       JSON_LENGTH(u.tags)                         AS tag_cnt,
       JSON_CONTAINS(u.attrs, 'true', '$.vip')     AS vip_flag,
       CONCAT_WS('.', (UNHEX(SUBSTR(HEX(u.signup_ip), 25, 2)) + 0),
                      UNHEX(SUBSTR(HEX(u.signup_ip), 27, 2)) + 0) AS signup_ip,
       ST_X(u.geo) AS lng, ST_Y(u.geo) AS lat,
       u.balance                                   AS wallet
FROM `sqlkit-demo`.users u
WHERE JSON_EXTRACT(u.attrs, '$.vip') = TRUE
   OR JSON_OVERLAPS(u.tags, CAST('[1,2]' AS JSON))
ORDER BY u.balance DESC
LIMIT 20;

-- ── F2. JSON_TABLE: 把 JSON 数组展开成行（关系化） ────────────────
SELECT u.username, jt.tag
FROM `sqlkit-demo`.users u,
     JSON_TABLE(u.tags, '$[*]' COLUMNS (tag INT PATH '$')) AS jt
WHERE u.id <= 12
ORDER BY u.id, jt.tag;

-- ── G. UPSERT: INSERT ... ON DUPLICATE KEY UPDATE ─────────────────
-- MySQL 没有 EXCLUDE 约束;并发独占用唯一键 + INSERT IGNORE 表达
INSERT INTO `sqlkit-demo`.reservations
    (resource_id, valid_from, valid_to, note_cents, note_currency)
VALUES (1, '2026-07-01 00:00:00', '2026-07-05 00:00:00', 50000, 'CNY');

INSERT INTO `sqlkit-demo`.coupons (code, discount, min_amount, valid_until)
VALUES ('SUMMER2026', 30.00, 200, '2026-08-31 23:59:59')
ON DUPLICATE KEY UPDATE
    discount = GREATEST(discount, VALUES(discount)),
    min_amount = LEAST(min_amount, VALUES(min_amount));

SELECT id, code, discount, min_amount FROM `sqlkit-demo`.coupons
WHERE code = 'SUMMER2026';

-- ── H. 库存对账合并（PG 用 MERGE，MySQL 用 UPDATE JOIN + INSERT） ─
-- 用 CTE 取代临时表: SQLKit 逐段执行时每段可能是独立连接，临时表无法跨段存活。
-- 注意: MySQL 8.4 已移除 VALUES()，且 INSERT ... SELECT 不支持行别名，
--       因此「存在则加、不存在则插」拆成 UPDATE JOIN + INSERT 两步。
WITH merged AS (
    SELECT product_id, SUM(delta) AS delta FROM (
        SELECT 1 AS product_id, -3 AS delta
        UNION ALL SELECT 2, 5
        UNION ALL SELECT 1, 1
    ) d GROUP BY product_id
)
UPDATE `sqlkit-demo`.products p
JOIN merged m ON m.product_id = p.id
SET p.stock = GREATEST(0, p.stock + m.delta),
    p.updated_at = NOW(3);

INSERT INTO `sqlkit-demo`.products
    (id, merchant_id, sku, name, price, stock, specs, keywords)
SELECT 99, 1, 'SKU-NEW-99', '自动对账商品-99', 9.90, 2,
       JSON_OBJECT('origin', '对账'), JSON_ARRAY(0)
WHERE NOT EXISTS (SELECT 1 FROM `sqlkit-demo`.products WHERE id = 99);

SELECT id, sku, name, stock FROM `sqlkit-demo`.products
WHERE id IN (1, 2, 99) ORDER BY id;

-- ── I. 事务 + 保存点回滚（嵌套事务语义） ──────────────────────────
-- SQLKit 的 MySQL 通道把编辑器内容作为「一条请求」下发，多语句中间一旦报错
-- 就会中断后续语句。因此把整个事务封进存储过程：SQLEXCEPTION 处理器吞掉
-- 预期错误，ROLLBACK TO SAVEPOINT 再把事务救回，最终只剩一个干净结果集。
DROP PROCEDURE IF EXISTS tx_savepoint_demo;

DELIMITER $$
CREATE PROCEDURE tx_savepoint_demo()
BEGIN
    -- 捕获预期错误(3819)，避免中断过程中的后续语句
    DECLARE CONTINUE HANDLER FOR SQLEXCEPTION BEGIN END;

    START TRANSACTION;
        DELETE FROM `sqlkit-demo`.users WHERE username = 'tx_probe';
        INSERT INTO `sqlkit-demo`.users (username, email, password_hash, attrs, tags, geo)
        VALUES ('tx_probe', 'txprobe@example.com', MD5('x'), JSON_OBJECT(), JSON_ARRAY(),
                ST_SRID(POINT(120.1, 30.1), 4326));

        SAVEPOINT before_bad_insert;
        -- ▼ 预期错误: valid_to 早于 valid_from，触发 chk_reservations_range
        INSERT INTO `sqlkit-demo`.reservations
            (resource_id, valid_from, valid_to, note_cents, note_currency)
        VALUES (1, '2026-06-10 00:00:00', '2026-06-01 00:00:00', 1000, 'CNY');
        -- 把事务从 aborted 状态救回，前面的 INSERT 依然保留
        ROLLBACK TO SAVEPOINT before_bad_insert;

        UPDATE `sqlkit-demo`.users SET balance = balance + 100 WHERE username = 'tx_probe';
    COMMIT;
END$$

DELIMITER ;

CALL tx_savepoint_demo();

SELECT username, balance FROM `sqlkit-demo`.users WHERE username = 'tx_probe';

-- ── J. 存储过程调用（IN / OUT 参数） ──────────────────────────────
SET @claimed = -1;
CALL `sqlkit-demo`.claim_coupon('100010', 42, @claimed);
SELECT @claimed AS claim_ok, code, claimed_by
FROM `sqlkit-demo`.coupons WHERE code = '100010';

CALL `sqlkit-demo`.void_order('SO-20260100017');
SELECT order_no, status FROM `sqlkit-demo`.orders WHERE order_no = 'SO-20260100017';

CALL `sqlkit-demo`.merchant_report(30);

-- ── K. 函数调用（标量函数 + 确定性标记） ──────────────────────────
SELECT `sqlkit-demo`.get_product_rank(1)   AS product_1_revenue,
       `sqlkit-demo`.cny_format(123456)    AS formatted,
       `sqlkit-demo`.get_product_rank(2)   AS product_2_revenue;

-- ── L. 集合操作与反连接: 反连接 / INTERSECT / EXCEPT ──────────────
-- MySQL 8.0.31+ 才支持 INTERSECT / EXCEPT
-- L1: 反连接 —— 找出「有下架商品」的商户及其下架占比
SELECT p.merchant_id,
       COUNT(*)                          AS all_cnt,
       SUM(p.is_active = 0)              AS inactive_cnt,
       ROUND(100 * SUM(p.is_active = 0) / COUNT(*), 1) AS inactive_pct
FROM `sqlkit-demo`.products p
GROUP BY p.merchant_id
HAVING SUM(p.is_active = 0) > 0
ORDER BY inactive_pct DESC;

-- L2: 视图 vs 基表的差异集合（可上架视图看不到的下架商品）
SELECT p.id, p.name, p.is_active
FROM `sqlkit-demo`.products p
LEFT JOIN `sqlkit-demo`.v_active_products v ON v.id = p.id
WHERE v.id IS NULL
ORDER BY p.id
LIMIT 15;

-- L3: INTERSECT —— 既贵又缺货的商品
SELECT p.name FROM `sqlkit-demo`.products p
WHERE p.price > 1000
INTERSECT
SELECT p.name FROM `sqlkit-demo`.products p WHERE p.stock < 100;

-- L4: EXCEPT —— 活跃但非 VIP 的用户
SELECT username FROM `sqlkit-demo`.users WHERE is_active = 1
EXCEPT
SELECT username FROM `sqlkit-demo`.users
WHERE JSON_EXTRACT(attrs, '$.vip') = TRUE
ORDER BY username
LIMIT 15;

-- ── M. 每类目最新商品（PG 的 DISTINCT ON → MySQL 用窗口函数） ─────
SELECT category_id, id, name, price, created_at
FROM (
    SELECT p.category_id, p.id, p.name, p.price, p.created_at,
           ROW_NUMBER() OVER (PARTITION BY p.category_id
                              ORDER BY p.created_at DESC, p.id DESC) AS rn
    FROM `sqlkit-demo`.products p
    WHERE p.category_id IS NOT NULL
) t
WHERE rn = 1
ORDER BY category_id;

-- ── N. JSON 聚合: 订单地址树构建 + 聚合 ───────────────────────────
SELECT o.shipping_addr ->> '$.province' AS province,
       -- MySQL 的 JSON_ARRAYAGG 不支持 ORDER BY;用 ORDER BY 子查询先排好再聚合
       JSON_ARRAYAGG(JSON_OBJECT('order', o.order_no, 'amount', o.total_amount))
                                                       AS orders_bundle,
       MAX(o.total_amount) AS biggest,
       COUNT(*)            AS order_cnt
FROM (
    SELECT o.order_no, o.total_amount, o.shipping_addr
    FROM `sqlkit-demo`.orders o
    WHERE o.shipping_addr ->> '$.city' = '杭州'
    ORDER BY o.total_amount DESC
) o
GROUP BY o.shipping_addr ->> '$.province';

-- ── N2. 全文检索（ngram 解析器 → 中文分词） ───────────────────────
SELECT id, name, MATCH(name) AGAINST('笔记本' IN NATURAL LANGUAGE MODE) AS score
FROM `sqlkit-demo`.products
WHERE MATCH(name) AGAINST('笔记本' IN NATURAL LANGUAGE MODE)
ORDER BY score DESC
LIMIT 10;

-- ── O. EXPLAIN: 查询计划（测试 rows/args 结果面） ─────────────────
EXPLAIN ANALYZE
SELECT p.name, oi.quantity
FROM `sqlkit-demo`.products p
JOIN `sqlkit-demo`.order_items oi ON oi.product_id = p.id
WHERE LOWER(p.sku) LIKE '%sku-01%'
ORDER BY oi.quantity DESC
LIMIT 10;

EXPLAIN FORMAT = TREE
SELECT * FROM `sqlkit-demo`.orders WHERE order_date = '2026-02-10';

-- 分区裁剪验证
EXPLAIN
SELECT * FROM `sqlkit-demo`.order_events WHERE event_year = 2027;

-- ── P. UPDATE JOIN + DELETE JOIN（复合连接修改） ──────────────────
-- 注意: MySQL 的多表 UPDATE/DELETE 不允许 LIMIT
USE `sqlkit-demo`;
UPDATE `sqlkit-demo`.reviews r
JOIN `sqlkit-demo`.users u ON u.id = r.user_id
SET r.content = CONCAT(r.content, ' [reviewed]')
WHERE JSON_EXTRACT(u.attrs, '$.vip') = TRUE;

SELECT COUNT(*) AS reviewed_cnt FROM `sqlkit-demo`.reviews WHERE content LIKE '%[reviewed]%';

DELETE r
FROM `sqlkit-demo`.reviews r
JOIN `sqlkit-demo`.payments pm ON pm.id = r.payment_id
WHERE pm.status = 'refunded';

-- ── Q. 唯一约束交换（MySQL 无 DEFERRABLE，必须借道临时值） ────────
-- PG 用可延迟约束让两个邮箱在同一事务内互相冲突;MySQL 的唯一键是即时校验，
-- 两步交换必然失败，只能借道临时值走三步 —— 这段专门演示两者的语义差异。
-- 同样封进存储过程，让「预期错误」不打断后续语句。
USE `sqlkit-demo`;

DROP PROCEDURE IF EXISTS email_swap_two_step;
DROP PROCEDURE IF EXISTS email_swap_three_step;

DELIMITER $$

-- ① 两步交换: 第 2 步因唯一键冲突失败(预期错误 1062)，整体回滚
CREATE PROCEDURE email_swap_two_step()
BEGIN
    DECLARE CONTINUE HANDLER FOR SQLEXCEPTION BEGIN END;
    START TRANSACTION;
        UPDATE `sqlkit-demo`.users SET email = 'user2@mail.example.com'
         WHERE username = 'user_1';
        UPDATE `sqlkit-demo`.users SET email = 'user1@mail.example.com'
         WHERE username = 'user_2';
    ROLLBACK;
END$$

-- ② 三步交换: 借道临时值 —— 成功
CREATE PROCEDURE email_swap_three_step()
BEGIN
    START TRANSACTION;
        UPDATE `sqlkit-demo`.users SET email = 'swap_tmp@example.com'
         WHERE username = 'user_1';
        UPDATE `sqlkit-demo`.users SET email = 'user1@mail.example.com'
         WHERE username = 'user_2';
        UPDATE `sqlkit-demo`.users SET email = 'user2@mail.example.com'
         WHERE username = 'user_1';
    COMMIT;
END$$

DELIMITER ;

-- 重置到互不冲突的确定状态(可反复执行、与起始状态无关)
UPDATE `sqlkit-demo`.users SET email = 'reset_1@tmp.example' WHERE username = 'user_1';
UPDATE `sqlkit-demo`.users SET email = 'reset_2@tmp.example' WHERE username = 'user_2';

CALL email_swap_two_step();
SELECT username, email FROM `sqlkit-demo`.users
WHERE username IN ('user_1', 'user_2') ORDER BY username;

CALL email_swap_three_step();
SELECT username, email FROM `sqlkit-demo`.users
WHERE username IN ('user_1', 'user_2') ORDER BY username;

-- 还原
UPDATE `sqlkit-demo`.users SET email = 'reset_1@tmp.example' WHERE username = 'user_1';
UPDATE `sqlkit-demo`.users SET email = 'reset_2@tmp.example' WHERE username = 'user_2';

-- ── R. 例程元数据: functions / procedures 面板数据源 ──────────────
SELECT routine_schema, routine_name, routine_type, data_type, is_deterministic
FROM information_schema.routines
WHERE routine_schema = 'sqlkit-demo'
ORDER BY routine_type, routine_name;

-- ── S. 窗口框架 RANGE 与滑动窗口（MySQL 8.0 支持 RANGE 帧） ───────
SELECT d.sales_day, d.revenue,
       SUM(d.revenue) OVER (ORDER BY d.sales_day
           RANGE BETWEEN INTERVAL 7 DAY PRECEDING AND CURRENT ROW) AS rev_7d
FROM `sqlkit-analytics`.v_daily_sales d
ORDER BY d.sales_day;

-- ── T. 分区路由验证（order_events 四个分区） ──────────────────────
SELECT partition_name, table_rows
FROM information_schema.partitions
WHERE table_schema = 'sqlkit-demo' AND table_name = 'order_events'
ORDER BY partition_ordinal_position;

START TRANSACTION;
INSERT INTO `sqlkit-demo`.order_events
    (order_id, event_year, event_type, event_at, detail)
VALUES (1, 2028, 'FUTURE_EVENT', NOW(3), JSON_OBJECT('city', 'default'))
ON DUPLICATE KEY UPDATE detail = VALUES(detail);
ROLLBACK;

-- ── U. 触发器效果验证（跨表审计落到 audit_log） ───────────────────
SELECT table_name, op_type, COUNT(*) AS cnt
FROM `sqlkit-demo`.audit_log
GROUP BY table_name, op_type
ORDER BY table_name, op_type;

-- 完成。清理残余演示数据:
DELETE FROM `sqlkit-demo`.users WHERE username = 'tx_probe';
