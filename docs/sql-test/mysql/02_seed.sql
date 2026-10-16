-- =====================================================================
-- SQLKit MySQL 压测 2/3 —— 真实业务数据灌入
-- 前置: 01_schema.sql 已执行
-- 规模: 400 用户 / 10 商户 / 60 商品 / 500 订单 / ~1250 行明细
-- 确定性生成（递归 CTE 数字序列 + 取模打散），可反复执行
-- 注意: MySQL 8.0+ 才支持递归 CTE；下面临时放宽递归深度到 10000
-- =====================================================================

SET SESSION cte_max_recursion_depth = 10000;
SET SESSION unique_checks = 0;

-- ── 0. 清场（分区表也能 TRUNCATE；外键顺序由 SET FOREIGN_KEY_CHECKS 兜底） ──
SET FOREIGN_KEY_CHECKS = 0;
TRUNCATE TABLE `sqlkit-demo`.audit_log;
TRUNCATE TABLE `sqlkit-demo`.order_events;
TRUNCATE TABLE `sqlkit-demo`.reservations;
TRUNCATE TABLE `sqlkit-demo`.coupons;
TRUNCATE TABLE `sqlkit-demo`.reviews;
TRUNCATE TABLE `sqlkit-demo`.payments;
TRUNCATE TABLE `sqlkit-demo`.order_items;
TRUNCATE TABLE `sqlkit-demo`.orders;
TRUNCATE TABLE `sqlkit-demo`.products;
TRUNCATE TABLE `sqlkit-demo`.categories;
TRUNCATE TABLE `sqlkit-demo`.merchants;
TRUNCATE TABLE `sqlkit-demo`.users;
SET FOREIGN_KEY_CHECKS = 1;

-- ── 1. 分类: 二级分类树（8 个一级 + 2 个二级 → 递归 CTE 压测素材） ──
INSERT INTO `sqlkit-demo`.categories (id, parent_id, name) VALUES
  (1, NULL, '图书音像'), (2, NULL, '数码家电'), (3, NULL, '服饰鞋包'),
  (4, NULL, '美妆个护'), (5, NULL, '食品生鲜'), (6, NULL, '运动户外'),
  (7, 2,    '手机数码'), (8, 2,    '电脑办公'), (9, 3, '男装'), (10, 3, '女装');
UPDATE `sqlkit-demo`.categories SET is_leaf = 1 WHERE parent_id IS NOT NULL;

-- ── 2. 商户: 10 家（含 0 佣金边界值） ─────────────────────────────
INSERT INTO `sqlkit-demo`.merchants (id, name, commission, contact_email, verified) VALUES
  (1,  '晨光书社',    0.0300, 'svc@chenguang.example', 1),
  (2,  '极客电器城',  0.0400, 'ops@geekcity.example',  1),
  (3,  '潮流严选',    0.0350, 'hi@trend.example',      1),
  (4,  '丽人美妆铺',  0.0200, 'care@liren.example',    1),
  (5,  '鲜丰果盘',    0.0100, 'bg@xianfeng.example',   0),
  (6,  '户外nice计划', 0.0300, 'camp@nice.example',    0),
  (7,  '纸品工坊',    0.0000, 'zero@paper.example',    1),
  (8,  '云帆数码',    0.0380, 'yun@cloudsail.example', 1),
  (9,  '布丁百货',    0.0300, 'shop@puding.example',   1),
  (10, '拾光影音',    0.0300, 'av@shiguang.example',   1);

-- ── 3. 商品: 60 件（跨商户/类目，含下架与 0 库存边界值） ───────────
INSERT INTO `sqlkit-demo`.products
    (id, merchant_id, category_id, sku, name, item_condition,
     price, stock, weight_g, specs, keywords, is_active)
WITH RECURSIVE seq(n) AS (
    SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < 60
)
SELECT
    n,
    1 + n % 10,
    CASE WHEN n % 5 = 1 THEN 7 WHEN n % 5 = 2 THEN 1
         WHEN n % 5 = 3 THEN 2 ELSE 3 + n % 4 END,
    CONCAT('SKU-', LPAD(n, 4, '0')),
    CONCAT(
        ELT(1 + n % 6, '银河', '星尘', '云雾', '青柠', '暖阳', '拾光'),
        ELT(1 + n % 6, '笔记本', '机械键盘', '雪梨汁', '精华露', '冲锋衣', '悬疑小说集'),
        ' ', LPAD(n, 2, '0')),
    ELT(1 + n % 5, 'new', 'new', 'new', 'refurbished', 'used'),
    CASE WHEN n % 20 = 0 THEN 99.90
         ELSE 100 + MOD(TRUNCATE(n * 13.37, 0), 4800) END,
    CASE WHEN n % 23 = 0 THEN 0 ELSE 20 + n % 480 END,
    120 + n * 7.5,
    JSON_OBJECT('origin', ELT(1 + n % 4, '杭州', '深圳', '义乌', '成都'),
                'warranty_month', 6 + n % 19),
    JSON_ARRAY(CONCAT('key', n % 7), CONCAT('hot', n % 2)),
    IF(n % 31 <> 0, 1, 0)
FROM seq;

-- ── 4. 用户: 400 名（JSON 属性 / 多值标签 / 空间点 / 二进制 IP） ────
INSERT INTO `sqlkit-demo`.users
    (username, email, phone, password_hash, locale, balance,
     registered_at, last_login_at, attrs, tags, signup_ip, geo, is_active)
WITH RECURSIVE seq(n) AS (
    SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < 400
)
SELECT
    CONCAT('user_', n),
    CONCAT('user', n, '@mail.example.com'),
    CASE WHEN n % 7 = 0 THEN NULL
         ELSE CONCAT('138', LPAD(MOD(n * 7919, 10000), 8, '0')) END,
    MD5(CONCAT('pw', n)),
    CASE WHEN n % 11 = 0 THEN 'en_US' ELSE 'zh_CN' END,
    CASE WHEN n % 10 = 0 THEN 250.50 ELSE n % 300 END,
    TIMESTAMPADD(HOUR, n * 17, TIMESTAMP '2025-11-01 00:00:00'),
    TIMESTAMPADD(HOUR, n * 3,  TIMESTAMP '2026-01-05 00:00:00'),
    JSON_OBJECT(
        'age', 18 + n % 42,
        'city', ELT(1 + n % 6, '杭州', '成都', '北京', '上海', '深圳', '广州'),
        'vip', n % 17 = 0,
        'profile', JSON_OBJECT('nick', CONCAT('nick_', n % 97))),
    -- tags 用数字数组: 多值索引只支持数值/日期元素
    JSON_ARRAY(n % 9, IF(n % 3 = 0, 11, 22)),
    -- IP 以 16 字节二进制保存（IPv4 映射）: 10.0.x.y
    UNHEX(CONCAT('00000000000000000000FFFF0A00',
                 LPAD(HEX(n % 250), 2, '0'), LPAD(HEX(1 + n % 250), 2, '0'))),
    -- 空间点: 经纬度（杭州为中心打散）
    ST_SRID(POINT(120.0 + (n % 100) / 100, 30.0 + (n % 70) / 100), 4326),
    1
FROM seq;

-- ── 5. 订单: 500 单横跨 2026-01-01 ~ 2026-03-30 ───────────────────
INSERT INTO `sqlkit-demo`.orders
    (id, order_no, user_id, status, order_date, shipping_fee,
     shipping_addr, remarks, placed_at, shipped_at, completed_at)
WITH RECURSIVE seq(n) AS (
    SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < 500
)
SELECT
    n,
    CONCAT('SO-2026', LPAD(MONTH(d.dt), 2, '0'), LPAD(n, 5, '0')),
    1 + n % 400,
    ELT(1 + n % 6, 'paid', 'completed', 'completed', 'shipped',
        'cancelled', 'refunded'),
    DATE(d.dt),
    CASE WHEN n % 13 = 0 THEN 0.00 ELSE 8.00 END,
    JSON_OBJECT(
        'province', ELT(1 + n % 5, '浙江', '四川', '北京', '上海', '广东'),
        'city',     ELT(1 + n % 5, '杭州', '成都', '北京', '上海', '深圳'),
        'detail',   CONCAT('仿真路 88 号 ', n, ' 栋'),
        'receiver', CONCAT('收件人', n % 100)),
    CASE WHEN n % 9 = 0 THEN '新用户求加急发货' ELSE NULL END,
    TIMESTAMPADD(HOUR, n % 24, d.dt),
    CASE WHEN n % 6 NOT IN (1, 5)
         THEN TIMESTAMPADD(DAY, 2, d.dt) END,
    CASE WHEN n % 6 = 2
         THEN TIMESTAMPADD(DAY, 5, d.dt) END
FROM seq
CROSS JOIN LATERAL (
    SELECT TIMESTAMPADD(DAY, MOD(n * 11, 85),
                        TIMESTAMP '2026-01-01 09:00:00') AS dt
) d;

-- ── 6. 明细: 每单 1-4 行（数量 1-3、独立取品打散） ─────────────────
INSERT INTO `sqlkit-demo`.order_items (order_id, product_id, quantity, unit_price)
WITH RECURSIVE seq(n) AS (
    SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < 4
)
SELECT o.id, pp.id, 1 + n % 3, pp.price
FROM `sqlkit-demo`.orders o
JOIN seq ON n <= 1 + o.id % 4
JOIN LATERAL (
    SELECT p.id, p.price
    FROM `sqlkit-demo`.products p
    WHERE p.is_active = 1
    ORDER BY MOD(p.id * 31 + o.id, 97)
    LIMIT 1
) pp ON TRUE;

-- ── 7. 合计回填订单主表（UPDATE ... JOIN 聚合子查询） ─────────────
UPDATE `sqlkit-demo`.orders o
JOIN (
    SELECT order_id, SUM(quantity * unit_price) AS amount
    FROM `sqlkit-demo`.order_items
    GROUP BY order_id
) agg ON agg.order_id = o.id
SET o.total_amount = agg.amount;

-- ── 8. 支付: 已取消订单不产生支付 ─────────────────────────────────
INSERT INTO `sqlkit-demo`.payments
    (order_id, external_no, amount, payment_channel, status, paid_at, payload)
SELECT o.id, CONCAT('PAY', o.order_no), o.total_amount,
       ELT(1 + o.id % 3, 'alipay', 'wechat', 'unionpay'),
       o.status,
       TIMESTAMPADD(MINUTE, 3, o.placed_at),
       RANDOM_BYTES(64)
FROM `sqlkit-demo`.orders o
WHERE o.status <> 'cancelled';

-- ── 9. 评价: 已完成订单用户对最贵一件商品打分（防重复评价） ────────
INSERT INTO `sqlkit-demo`.reviews (payment_id, user_id, product_id, rating, content)
SELECT pm.id, o.user_id, top.product_id,
       1 + MOD(o.id + top.oi_id, 5),
       ELT(1 + o.id % 5, '很好用，物流很快', '一般般吧', '回购无数次',
           '包装精美', '凑单买的，还行')
FROM `sqlkit-demo`.orders o
JOIN LATERAL (
    SELECT oi.id AS oi_id, oi.product_id
    FROM `sqlkit-demo`.order_items oi
    WHERE oi.order_id = o.id
    ORDER BY oi.unit_price DESC
    LIMIT 1
) top ON TRUE
JOIN `sqlkit-demo`.payments pm ON pm.order_id = o.id
WHERE o.status = 'completed'
  AND NOT EXISTS (
      SELECT 1 FROM `sqlkit-demo`.reviews r
      WHERE r.user_id = o.user_id AND r.product_id = top.product_id);

-- ── 10. 券与领取 ──────────────────────────────────────────────────
INSERT INTO `sqlkit-demo`.coupons (id, code, discount, min_amount, valid_until,
                                   claimed_by, claimed_at)
WITH RECURSIVE seq(n) AS (
    SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < 24
)
SELECT n, LPAD(100000 + n, 6, '0'), 5 + n * 2.5, 100 + n * 10,
       TIMESTAMP '2026-12-31 23:59:59',
       CASE WHEN n % 4 = 0 THEN 1 + MOD(n * 3, 400) END,
       CASE WHEN n % 4 = 0
            THEN TIMESTAMPADD(DAY, -n, TIMESTAMP '2026-01-01 00:00:00') END
FROM seq;

-- ── 11. 库存预留（时间区间列近似 PG 的 tstzrange） ────────────────
INSERT INTO `sqlkit-demo`.reservations
    (resource_id, valid_from, valid_to, note_cents, note_currency) VALUES
  (1, '2026-06-01 00:00:00', '2026-06-10 00:00:00', 120000, 'CNY'),
  (2, '2026-06-01 00:00:00', '2026-06-30 00:00:00',  98000, 'CNY');

-- ── 12. 订单事件: 跨分区写入（2026 与 2027 两个分区） ──────────────
INSERT INTO `sqlkit-demo`.order_events (order_id, event_year, event_type, event_at, detail)
SELECT o.id, YEAR(o.order_date), 'ORDER_CREATED', o.placed_at,
       JSON_OBJECT('status', o.status, 'amount', o.total_amount)
FROM `sqlkit-demo`.orders o
UNION ALL
SELECT o.id, 2027, 'ARCHIVED', TIMESTAMP '2027-01-05 08:00:00',
       JSON_OBJECT('reason', 'yearly-archive')
FROM `sqlkit-demo`.orders o
WHERE o.id % 50 = 0;

-- ── 13. 显式 id 灌入后刷新统计信息 ───────────────────────────────
ANALYZE TABLE `sqlkit-demo`.users, `sqlkit-demo`.products,
              `sqlkit-demo`.orders, `sqlkit-demo`.order_items;

SET SESSION unique_checks = 1;

-- ── 14. 校验卡片 ─────────────────────────────────────────────────
SELECT 'users'      AS what, COUNT(*) AS cnt FROM `sqlkit-demo`.users
UNION ALL SELECT 'products',  COUNT(*) FROM `sqlkit-demo`.products
UNION ALL SELECT 'orders',    COUNT(*) FROM `sqlkit-demo`.orders
UNION ALL SELECT 'items',     COUNT(*) FROM `sqlkit-demo`.order_items
UNION ALL SELECT 'payments',  COUNT(*) FROM `sqlkit-demo`.payments
UNION ALL SELECT 'reviews',   COUNT(*) FROM `sqlkit-demo`.reviews
UNION ALL SELECT 'coupons',   COUNT(*) FROM `sqlkit-demo`.coupons
UNION ALL SELECT 'events',    COUNT(*) FROM `sqlkit-demo`.order_events
UNION ALL SELECT 'audit',     COUNT(*) FROM `sqlkit-demo`.audit_log
ORDER BY 1;
