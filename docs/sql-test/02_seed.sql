-- =====================================================================
-- SQLKit PostgreSQL 压测 2/3 —— 真实业务数据灌入
-- 前置: 01_schema.sql 已执行
-- 规模: 400 用户 / 10 商户 / 60 商品 / 500 订单(1-3 月三分区) / ~1200 行明细
-- 确定性生成（generate_series + 取模打散），可反复执行
-- =====================================================================

BEGIN;

-- 注意: 视图与物化视图不在 TRUNCATE 列表——不能 TRUNCATE
TRUNCATE store.audit_log, store.reservations, store.coupons,
         store.reviews, store.payments, store.order_items,
         store.orders, store.products, store.categories,
         store.merchants, store.users
    RESTART IDENTITY CASCADE;

-- 分类: 二级分类树（8 个一级 + 2 个二级 → 递归 CTE 压测素材）
INSERT INTO store.categories (id, parent_id, name) VALUES
  (1, NULL, '图书音像'), (2, NULL, '数码家电'), (3, NULL, '服饰鞋包'),
  (4, NULL, '美妆个护'), (5, NULL, '食品生鲜'), (6, NULL, '运动户外'),
  (7, 2,   '手机数码'), (8, 2,   '电脑办公');
UPDATE store.categories SET is_leaf = true WHERE parent_id IS NOT NULL;

-- 商户: 10 家（一家佣金 0 边界值）——必须先于 products（FK 依赖）
INSERT INTO store.merchants (id, name, commission, contact_email, verified)
OVERRIDING SYSTEM VALUE VALUES
  (1, '晨光书社',    0.0300, 'svc@chenguang.example', true),
  (2, '极客电器城',  0.0400, 'ops@geekcity.example',  true),
  (3, '潮流严选',    0.0350, 'hi@trend.example',      true),
  (4, '丽人美妆铺',  0.0200, 'care@liren.example',    true),
  (5, '鲜丰果盘',    0.0100, 'bg@xianfeng.example',   false),
  (6, '户外nice计划', 0.0300, 'camp@nice.example',    false),
  (7, '纸品工坊',    0.0000, 'zero@paper.example',    true),
  (8, '云帆数码',    0.0380, 'yun@cloudsail.example', true),
  (9, '布丁百货',    0.0300, 'shop@puding.example',   true),
  (10,'拾光影音',    0.0300, 'av@shiguang.example',   true);

-- 商品: 60 件（跨商户/类目，含下架与 0 库存边界值）
INSERT INTO store.products (id, merchant_id, category_id, sku, name, condition,
                            price, stock, weight_g, dimensions, specs,
                            keywords, is_active)
OVERRIDING SYSTEM VALUE
SELECT
    g,
    1 + g % 10,
    CASE WHEN g % 5 = 1 THEN 7 WHEN g % 5 = 2 THEN 1
         WHEN g % 5 = 3 THEN 2 ELSE 3 + g % 4 END,
    'SKU-' || lpad(g::text, 4, '0'),
    (ARRAY['银河','星尘','云雾','青柠','暖阳','拾光'])[1 + g % 6]
      || (ARRAY['笔记本','机械键盘','雪梨汁','精华露','冲锋屋','悬疑小说集'])[1 + g % 6]
      || ' ' || lpad(g::text, 2, '0'),
    (ARRAY['new','new','new','refurbished','used'])[1 + g % 5]::product_condition,
    CASE WHEN g % 20 = 0 THEN 99.90
         ELSE 100 + trunc((g * 13.37))::numeric % 4800 END,
    CASE WHEN g % 23 = 0 THEN 0 ELSE 20 + g % 480 END,
    120 + (g * 7.5)::numeric,
    (300, 'CNY')::money_amount,
    jsonb_build_object(
        'origin', (ARRAY['杭州','深圳','义乌','成都'])[1 + g % 4],
        'warranty_month', 6 + g % 19
    ),
    ARRAY['key' || (g % 7), 'hot' || (g % 2)::text],
    g % 31 <> 0
FROM generate_series(1, 60) g;

-- 客户: 400 名（随机属性、JSONB 偏好、标签数组）
INSERT INTO store.users (username, email, phone, password_hash, locale,
                         balance, registered_at, last_login_at, attrs, tags,
                         signup_ip, is_active)
SELECT
    'user_' || g,
    'user' || g || '@mail.example.com',
    CASE WHEN g % 7 = 0 THEN NULL ELSE '138' || lpad(((g * 7919) % 10000)::text, 8, '0') END,
    md5('pw' || g),
    CASE WHEN g % 11 = 0 THEN 'en_US' ELSE 'zh_CN' END,
    CASE WHEN g % 10 = 0 THEN 250.50::numeric ELSE (g % 300)::numeric END,
    '2025-11-01'::timestamptz + (g * interval '17 hours'),
    '2026-01-05'::timestamptz + (g * interval '3 hours'),
    jsonb_build_object(
        'age',   18 + g % 42,
        'city',  (ARRAY['杭州','成都','北京','上海','深圳','广州'])[1 + g % 6],
        'vip',   g % 17 = 0,
        'profile', jsonb_build_object('nick', 'nick_' || (g % 97))
    ),
    ARRAY['tag' || (g % 9), CASE WHEN g % 3 = 0 THEN 'reader' ELSE 'buyer' END],
    ('10.0.' || (g % 250) || '.' || (1 + g % 250))::inet,
    true
FROM generate_series(1, 400) g;

-- 订单: 500 单横跨 2026-01-01 ~ 2026-03-30（自动路由到三个分区）
INSERT INTO store.orders (id, order_date, order_no, user_id, status,
                          shipping_fee, shipping_addr, remarks, placed_at,
                          shipped_at, completed_at)
OVERRIDING SYSTEM VALUE
SELECT
    g,
    d.dt::date,
    'SO-2026' || to_char(d.dt, 'MM') || lpad(g::text, 5, '0'),
    1 + (g % 400),
    (ARRAY['paid','completed','completed','shipped','cancelled','refunded'])
        [1 + g % 6]::order_status,
    CASE WHEN g % 13 = 0 THEN 0 ELSE 8.00 END,
    jsonb_build_object(
        'province', (ARRAY['浙江','四川','北京','上海','广东'])[1 + g % 5],
        'city',     (ARRAY['杭州','成都','北京','上海','深圳'])[1 + g % 5],
        'detail',   '仿真路 88 号 ' || g || ' 栋',
        'receiver', '收件人' || (g % 100)
    ),
    CASE WHEN g % 9 = 0 THEN '新用户求加急发货' ELSE NULL END,
    d.dt + (g % 24 || ' hours')::interval,
    CASE WHEN g % 6 NOT IN (1, 5)
         THEN d.dt + interval '2 days' ELSE NULL END,
    CASE WHEN g % 6 = 2 THEN d.dt + interval '5 days' ELSE NULL END
FROM generate_series(1, 500) g,
LATERAL (SELECT '2026-01-01 09:00'::timestamptz
         + ((g * 11) % 85) * interval '1 day' AS dt) d;  -- 85 天循环打散

-- 明细: 每单 1-4 行（数量 1-3、独立取品打散）
INSERT INTO store.order_items (order_id, order_date, product_id, quantity, unit_price)
SELECT o.id, o.order_date, pp.id, 1 + i.i % 3, pp.price
FROM store.orders o
JOIN LATERAL generate_series(1, 1 + o.id % 4) i(i) ON true
JOIN LATERAL (SELECT pr.id, pr.price
              FROM store.products pr
              WHERE pr.is_active
              ORDER BY (pr.id * 31 + o.id) % 97
              LIMIT 1) pp ON true;

-- 合计回填到订单主表
UPDATE store.orders o
SET total_amount = agg.amount
FROM (SELECT order_id, sum(quantity * unit_price) AS amount
      FROM store.order_items GROUP BY order_id) agg
WHERE agg.order_id = o.id;

-- 支付: 已取消订单不产生支付；FK 未命名（默认名）/SET NULL 语义
INSERT INTO store.payments (order_id, order_date, external_no, amount,
                            payment_channel, status, paid_at, payload)
SELECT o.id, o.order_date, 'PAY' || o.order_no, o.total_amount,
       (ARRAY['alipay','wechat','unionpay'])[1 + o.id % 3],
       o.status, o.placed_at + interval '3 minutes', gen_random_bytes(64)
FROM store.orders o
WHERE o.status <> 'cancelled';

-- 评价: 已完成订单用户对最贵一件商品打分（防重复评价）
INSERT INTO store.reviews (payment_id, user_id, product_id, rating, content)
SELECT pm.id, o.user_id, top.product_id,
       1 + (o.id + top.oi_id) % 5,
       (ARRAY['很好用，物流很快','一般般吧','回购无数次','包装精美','凑单买的，还行'])
         [1 + o.id % 5]
FROM store.orders o
JOIN LATERAL (SELECT oi.id AS oi_id, oi.product_id, oi.unit_price
              FROM store.order_items oi
              WHERE oi.order_id = o.id AND oi.order_date = o.order_date
              ORDER BY oi.unit_price DESC LIMIT 1) top ON true
JOIN store.payments pm ON pm.order_id = o.id
WHERE o.status = 'completed'
  AND NOT EXISTS (SELECT 1 FROM store.reviews r
                  WHERE r.user_id = o.user_id AND r.product_id = top.product_id);

-- 券与领取（EXCLUDE 独占约束压力）
INSERT INTO store.coupons (id, discount, min_amount, valid_until, claimed_by, claimed_at)
OVERRIDING SYSTEM VALUE
SELECT g, 5 + g * 2.5, 100 + g * 10,
       '2026-12-31'::timestamptz,
       CASE WHEN g % 4 = 0 THEN 1 + (g * 3) % 400 ELSE NULL END,
       CASE WHEN g % 4 = 0 THEN now() - interval '1 day' * g ELSE NULL END
FROM generate_series(1, 24) g;

-- 库存预留（EXCLUDE 时段互斥演示）
INSERT INTO store.reservations (resource_id, valid, note) VALUES
  (1, '[2026-06-01, 2026-06-10)', (1200, 'CNY')::money_amount),
  (2, '[2026-06-01, 2026-06-30)', (980,  'CNY')::money_amount);

-- 显式 id 灌入不推进 identity 序列：推到 max(id)，避免 03 的自增 INSERT 撞键
SELECT setval(pg_get_serial_sequence('store.merchants', 'id'), (SELECT max(id) FROM store.merchants));
SELECT setval(pg_get_serial_sequence('store.products',  'id'), (SELECT max(id) FROM store.products));
SELECT setval(pg_get_serial_sequence('store.orders',    'id'), (SELECT max(id) FROM store.orders));
SELECT setval(pg_get_serial_sequence('store.coupons',   'id'), (SELECT max(id) FROM store.coupons));

COMMIT;

-- 物化视图刷新（可在 SQLKit 里右键/菜单测试 REFRESH）
REFRESH MATERIALIZED VIEW analytics.daily_sales;

-- 校验卡片
SELECT 'users'   AS what, count(*) FROM store.users
UNION ALL SELECT 'products', count(*) FROM store.products
UNION ALL SELECT 'orders',   count(*) FROM store.orders
UNION ALL SELECT 'items',    count(*) FROM store.order_items
UNION ALL SELECT 'payments', count(*) FROM store.payments
UNION ALL SELECT 'reviews',  count(*) FROM store.reviews
UNION ALL SELECT 'audit',    count(*) FROM store.audit_log
ORDER BY 1;
