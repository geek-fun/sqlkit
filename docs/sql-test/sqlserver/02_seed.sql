-- =====================================================================
-- SQLKit SQL Server 压测 2/3 —— 真实业务数据灌入
-- 前置: 01_schema.sql 已执行
-- 规模: 400 用户 / 10 商户 / 60 商品 / 500 订单(1-3 月三分区) / ~1250 行明细
-- 生成方式: 集合操作（递归 CTE 数字表 + CROSS APPLY + CHOOSE + ROW_NUMBER）,
--           全程无 WHILE 循环、无逐行处理, 可反复执行
-- 注意: 本文件含 GO 批分隔符, 请用 sqlcmd 执行
-- =====================================================================
USE testdb;
GO

-- SQL Server 陷阱: 索引视图 / 过滤索引要求下列 SET 选项, 否则 DML 直接失败
SET ANSI_NULLS ON;
SET ANSI_PADDING ON;
SET ANSI_WARNINGS ON;
SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON;
SET QUOTED_IDENTIFIER ON;
SET NUMERIC_ROUNDABORT OFF;
SET NOCOUNT ON;
GO

-- ── 0. 数字表: 递归 CTE 生成 1..1000（替代 PG 的 generate_series） ──
IF OBJECT_ID('tempdb..#nums') IS NOT NULL DROP TABLE #nums;
WITH seq(n) AS (
    SELECT 1
    UNION ALL
    SELECT n + 1 FROM seq WHERE n < 1000
)
SELECT ROW_NUMBER() OVER (ORDER BY n) AS n
INTO #nums
FROM seq
OPTION (MAXRECURSION 0);
CREATE UNIQUE CLUSTERED INDEX ix_nums ON #nums (n);
GO

BEGIN TRY
    BEGIN TRANSACTION;

    -- ── 1. 清场（T-SQL 的 TRUNCATE 没有 CASCADE → 按依赖顺序 DELETE） ──
    DELETE FROM sqlkit.reviews;
    DELETE FROM sqlkit.coupons;
    DELETE FROM sqlkit.reservations;
    DELETE FROM sqlkit.payments;
    DELETE FROM sqlkit.order_items;
    DELETE FROM sqlkit_billing.invoices;
    DELETE FROM sqlkit.orders;          -- AFTER DELETE 触发器会写审计行, 稍后一并清掉
    DELETE FROM sqlkit.products;
    DELETE FROM sqlkit.categories;
    DELETE FROM sqlkit.merchants;
    DELETE FROM sqlkit.users;
    DELETE FROM sqlkit.audit_log;
    DELETE FROM sqlkit_analytics.metrics;
    DELETE FROM sqlkit_billing.metrics;

    -- 时态表清空: 历史表禁止 DELETE（错误 13560）, 只能先关版本控制再 TRUNCATE
    ALTER TABLE sqlkit.price_book SET (SYSTEM_VERSIONING = OFF);
    TRUNCATE TABLE sqlkit.price_book_history;
    DELETE FROM sqlkit.price_book;
    ALTER TABLE sqlkit.price_book SET (SYSTEM_VERSIONING = ON (HISTORY_TABLE = sqlkit.price_book_history));

    -- ── 2. 分类: 二级分类树（8 个一级 + 2 个二级 → 递归 CTE 压测素材） ──
    SET IDENTITY_INSERT sqlkit.categories ON;
    INSERT INTO sqlkit.categories (id, parent_id, name) VALUES
      (1,  NULL, N'图书音像'), (2,  NULL, N'数码家电'), (3,  NULL, N'服饰鞋包'),
      (4,  NULL, N'美妆个护'), (5,  NULL, N'食品生鲜'), (6,  NULL, N'运动户外'),
      (7,  2,    N'手机数码'), (8,  2,    N'电脑办公'),
      (9,  2,    N'影音娱乐'), (10, 4,    N'护肤彩妆');
    SET IDENTITY_INSERT sqlkit.categories OFF;
    UPDATE sqlkit.categories SET is_leaf = 1 WHERE parent_id IS NOT NULL;
    DBCC CHECKIDENT ('sqlkit.categories', RESEED, 10) WITH NO_INFOMSGS;

    -- ── 3. 商户: 10 家（一家佣金 0 边界值）——必须先于 products（FK 依赖） ──
    SET IDENTITY_INSERT sqlkit.merchants ON;
    INSERT INTO sqlkit.merchants (id, name, commission, contact_email, verified) VALUES
      (1,  N'晨光书社',     0.0300, N'svc@chenguang.example', 1),
      (2,  N'极客电器城',   0.0400, N'ops@geekcity.example',  1),
      (3,  N'潮流严选',     0.0350, N'hi@trend.example',      1),
      (4,  N'丽人美妆铺',   0.0200, N'care@liren.example',    1),
      (5,  N'鲜丰果盘',     0.0100, N'bg@xianfeng.example',   0),
      (6,  N'户外 nice 计划', 0.0300, N'camp@nice.example',   0),
      (7,  N'纸品工坊',     0.0000, N'zero@paper.example',    1),
      (8,  N'云帆数码',     0.0380, N'yun@cloudsail.example', 1),
      (9,  N'布丁百货',     0.0300, N'shop@puding.example',   1),
      (10, N'拾光影音',     0.0300, N'av@shiguang.example',   1);
    SET IDENTITY_INSERT sqlkit.merchants OFF;
    DBCC CHECKIDENT ('sqlkit.merchants', RESEED, 10) WITH NO_INFOMSGS;

    -- ── 4. 商品: 60 件（跨商户/类目, 含下架与 0 库存边界值） ──────────
    SET IDENTITY_INSERT sqlkit.products ON;
    INSERT INTO sqlkit.products (id, merchant_id, category_id, sku, name, condition_t,
                                 price, stock, weight_g, dim_cents, dim_ccy,
                                 specs, tags, is_active)
    SELECT n.n,
           1 + n.n % 10,
           CASE WHEN n.n % 5 = 1 THEN 7 WHEN n.n % 5 = 2 THEN 1
                WHEN n.n % 5 = 3 THEN 2 ELSE 3 + n.n % 4 END,
           'SKU-' + RIGHT('0000' + CAST(n.n AS VARCHAR(4)), 4),
           CHOOSE(1 + n.n % 6, N'银河', N'星尘', N'云雾', N'青柠', N'暖阳', N'拾光')
             + CHOOSE(1 + n.n % 6, N'笔记本', N'机械键盘', N'雪梨汁', N'精华露', N'冲锋衣', N'悬疑小说集')
             + N' ' + RIGHT(N'0' + CAST(n.n AS NVARCHAR(2)), 2),
           CHOOSE(1 + n.n % 5, 'new', 'new', 'new', 'refurbished', 'used'),
           CASE WHEN n.n % 20 = 0 THEN 99.90 ELSE 100 + (n.n * 1337) % 4800 END,
           CASE WHEN n.n % 23 = 0 THEN 0 ELSE 20 + n.n % 480 END,
           CAST(120 + n.n * 7.5 AS DECIMAL(8,1)),
           300, 'CNY',
           N'{"origin":"' + CHOOSE(1 + n.n % 4, N'杭州', N'深圳', N'义乌', N'成都')
             + N'","warranty_month":' + CAST(6 + n.n % 19 AS NVARCHAR(4)) + N'}',
           N'["key' + CAST(n.n % 7 AS NVARCHAR(2)) + N'","hot' + CAST(n.n % 2 AS NVARCHAR(2)) + N'"]',
           CASE WHEN n.n % 31 = 0 THEN 0 ELSE 1 END
    FROM #nums n
    WHERE n.n <= 60;
    SET IDENTITY_INSERT sqlkit.products OFF;
    DBCC CHECKIDENT ('sqlkit.products', RESEED, 60) WITH NO_INFOMSGS;

    -- ── 5. 客户: 400 名（JSON 属性 / JSON 标签数组 / XML / hash） ─────
    -- 显式 id 保证 orders.user_id 的 1..400 永远可解析（IDENTITY 重跑不会回归 1）
    SET IDENTITY_INSERT sqlkit.users ON;
    INSERT INTO sqlkit.users (id, username, email, phone, password_hash, locale,
                              balance, registered_at, last_login_at, attrs, tags,
                              signup_ip, avatar, bio, wallet_cents, wallet_ccy, is_active)
    SELECT n.n,
           N'user_' + CAST(n.n AS NVARCHAR(4)),
           N'user' + CAST(n.n AS NVARCHAR(4)) + N'@mail.example.com',
           CASE WHEN n.n % 7 = 0 THEN NULL
                ELSE '138' + RIGHT('00000000' + CAST((n.n * 7919) % 10000 AS VARCHAR(4)), 8) END,
           CONVERT(CHAR(64), HASHBYTES('SHA2_256', 'pw' + CAST(n.n AS VARCHAR(10))), 2),
           CASE WHEN n.n % 11 = 0 THEN 'en_US' ELSE 'zh_CN' END,
           CASE WHEN n.n % 10 = 0 THEN 250.50 ELSE n.n % 300 END,
           DATEADD(HOUR, n.n * 17, CAST('2025-11-01T00:00:00' AS DATETIMEOFFSET(3))),
           DATEADD(HOUR, n.n * 3,  CAST('2026-01-05T00:00:00' AS DATETIMEOFFSET(3))),
           N'{"age":' + CAST(18 + n.n % 42 AS NVARCHAR(4))
             + N',"city":"' + CHOOSE(1 + n.n % 6, N'杭州', N'成都', N'北京', N'上海', N'深圳', N'广州') + N'"'
             + N',"vip":' + CASE WHEN n.n % 17 = 0 THEN N'true' ELSE N'false' END
             + N',"profile":{"nick":"nick_' + CAST(n.n % 97 AS NVARCHAR(4)) + N'"}}',
           N'["tag' + CAST(n.n % 9 AS NVARCHAR(2)) + N'","'
             + CASE WHEN n.n % 3 = 0 THEN N'reader' ELSE N'buyer' END + N'"]',
           '10.0.' + CAST(n.n % 250 AS VARCHAR(3)) + '.' + CAST(1 + n.n % 250 AS VARCHAR(3)),
           CONVERT(VARBINARY(MAX), HASHBYTES('SHA2_256', 'avatar-' + CAST(n.n AS VARCHAR(10)))),
           CAST(N'<bio><nick>nick_' + CAST(n.n % 97 AS NVARCHAR(4)) + N'</nick></bio>' AS XML),
           (n.n * 137) % 100000, 'CNY', 1
    FROM #nums n
    WHERE n.n <= 400;
    SET IDENTITY_INSERT sqlkit.users OFF;
    DBCC CHECKIDENT ('sqlkit.users', RESEED, 400) WITH NO_INFOMSGS;

    -- ── 6. 订单: 500 单横跨 2026-01-01 ~ 2026-03-30（自动路由到三个分区） ──
    SET IDENTITY_INSERT sqlkit.orders ON;
    INSERT INTO sqlkit.orders (id, order_no, user_id, status, order_date,
                               shipping_fee, shipping_addr, remarks,
                               placed_at, shipped_at, completed_at)
    SELECT n.n,
           N'SO-2026'
             + RIGHT(N'0' + CAST(MONTH(d.dt) AS NVARCHAR(2)), 2)
             + RIGHT(N'00000' + CAST(n.n AS NVARCHAR(5)), 5),
           1 + n.n % 400,
           CASE n.n % 6 WHEN 0 THEN 'paid' WHEN 1 THEN 'completed' WHEN 2 THEN 'completed'
                        WHEN 3 THEN 'shipped' WHEN 4 THEN 'cancelled' ELSE 'refunded' END,
           d.dt,
           CASE WHEN n.n % 13 = 0 THEN 0 ELSE 8.00 END,
           N'{"province":"' + CHOOSE(1 + n.n % 5, N'浙江', N'四川', N'北京', N'上海', N'广东') + N'"'
             + N',"city":"' + CHOOSE(1 + n.n % 5, N'杭州', N'成都', N'北京', N'上海', N'深圳') + N'"'
             + N',"detail":"仿真路 88 号 ' + CAST(n.n AS NVARCHAR(4)) + N' 栋"'
             + N',"receiver":"收件人' + CAST(n.n % 100 AS NVARCHAR(4)) + N'"}',
           CASE WHEN n.n % 9 = 0 THEN N'新用户求加急发货' ELSE NULL END,
           DATEADD(HOUR, n.n % 24, d.dt_ts),
           CASE WHEN n.n % 6 NOT IN (1, 5) THEN DATEADD(DAY, 2, d.dt_ts) ELSE NULL END,
           CASE WHEN n.n % 6 = 2 THEN DATEADD(DAY, 5, d.dt_ts) ELSE NULL END
    FROM #nums n
    CROSS APPLY (SELECT CAST(DATEADD(DAY, (n.n * 11) % 85, CAST('2026-01-01' AS DATE)) AS DATE) AS dt,
                        DATEADD(DAY, (n.n * 11) % 85, CAST('2026-01-01T09:00:00' AS DATETIME2(3))) AS dt_ts) d
    WHERE n.n <= 500;
    SET IDENTITY_INSERT sqlkit.orders OFF;
    DBCC CHECKIDENT ('sqlkit.orders', RESEED, 500) WITH NO_INFOMSGS;

    -- ── 7. 明细: 每单 1-4 行（数量 1-3、CROSS APPLY 独立取品打散） ────
    INSERT INTO sqlkit.order_items (order_id, order_date, product_id, quantity, unit_price)
    SELECT o.id, o.order_date, p.id, 1 + n.n % 3, p.price
    FROM sqlkit.orders o
    JOIN #nums n ON n.n <= 1 + o.id % 4
    CROSS APPLY (
        SELECT TOP (1) pr.id, pr.price
        FROM sqlkit.products pr
        WHERE pr.is_active = 1
        ORDER BY (pr.id * 31 + o.id * 7) % 97, pr.id
    ) p;

    -- 合计回填到订单主表
    UPDATE o
       SET o.total_amount = agg.amount
    FROM sqlkit.orders o
    JOIN (SELECT order_id, SUM(quantity * unit_price) AS amount
          FROM sqlkit.order_items GROUP BY order_id) agg
      ON agg.order_id = o.id;

    -- ── 8. 支付: 已取消订单不产生支付；复合外键为 ON DELETE SET NULL ──
    INSERT INTO sqlkit.payments (order_id, order_date, external_no, amount,
                                 payment_channel, status, paid_at, payload)
    SELECT o.id, o.order_date, 'PAY' + o.order_no, o.total_amount,
           CHOOSE(1 + o.id % 3, 'alipay', 'wechat', 'unionpay'),
           o.status,
           DATEADD(MINUTE, 3, o.placed_at),
           CONVERT(VARBINARY(MAX), HASHBYTES('SHA2_256', o.order_no))
    FROM sqlkit.orders o
    WHERE o.status <> 'cancelled';

    -- ── 9. 评价: 已完成订单用户对最贵一件商品打分（防重复评价） ──────
    INSERT INTO sqlkit.reviews (payment_id, user_id, product_id, rating, content)
    SELECT pm.id, o.user_id, top1.product_id,
           1 + (o.id + top1.oi_id) % 5,
           CHOOSE(1 + o.id % 5, N'很好用，物流很快', N'一般般吧', N'回购无数次', N'包装精美', N'凑单买的，还行')
    FROM sqlkit.orders o
    CROSS APPLY (
        SELECT TOP (1) oi.id AS oi_id, oi.product_id
        FROM sqlkit.order_items oi
        WHERE oi.order_id = o.id AND oi.order_date = o.order_date
        ORDER BY oi.unit_price DESC, oi.id
    ) top1
    JOIN sqlkit.payments pm ON pm.order_id = o.id
    WHERE o.status = 'completed'
      AND NOT EXISTS (SELECT 1 FROM sqlkit.reviews r
                      WHERE r.user_id = o.user_id AND r.product_id = top1.product_id);

    -- ── 10. 券与领取（code 由序列默认值生成范围镜像; 之后把序列推到 100024） ──
    SET IDENTITY_INSERT sqlkit.coupons ON;
    INSERT INTO sqlkit.coupons (id, code, discount, min_amount, valid_until, claimed_by, claimed_at)
    SELECT n.n,
           CAST(100000 + n.n - 1 AS VARCHAR(32)),
           5 + n.n * 2.5,
           100 + n.n * 10,
           CAST('2026-12-31T00:00:00' AS DATETIMEOFFSET(3)),
           CASE WHEN n.n % 4 = 0 THEN 1 + (n.n * 3) % 400 ELSE NULL END,
           CASE WHEN n.n % 4 = 0 THEN DATEADD(DAY, -n.n, CAST('2026-01-01T00:00:00' AS DATETIMEOFFSET(3))) ELSE NULL END
    FROM #nums n
    WHERE n.n <= 24;
    SET IDENTITY_INSERT sqlkit.coupons OFF;
    DBCC CHECKIDENT ('sqlkit.coupons', RESEED, 24) WITH NO_INFOMSGS;
    ALTER SEQUENCE sqlkit.coupon_code_seq RESTART WITH 100024;

    -- ── 11. 库存预留（时段互斥由 trg_reservations_no_overlap 兜底） ──
    INSERT INTO sqlkit.reservations (resource_id, valid_from, valid_to, note_cents, note_ccy) VALUES
      (1, '2026-06-01', '2026-06-10', 1200, 'CNY'),
      (2, '2026-06-01', '2026-06-30',  980, 'CNY');

    -- ── 12. 时态表: 先建价再改价 → 自动生成历史行 ────────────────────
    INSERT INTO sqlkit.price_book (product_id, list_price, note) VALUES
      (1, 199.00, N'初始定价'),
      (2, 299.00, N'初始定价'),
      (3, 399.00, N'初始定价');
    UPDATE sqlkit.price_book SET list_price = 189.00, note = N'大促调价' WHERE product_id = 1;
    UPDATE sqlkit.price_book SET list_price = 279.00, note = N'渠道调价' WHERE product_id = 2;
    DBCC CHECKIDENT ('sqlkit.price_book', RESEED, 3) WITH NO_INFOMSGS;

    -- ── 13. 结算发票（跨 schema 复合外键 + ON DELETE CASCADE） ───────
    INSERT INTO sqlkit_billing.invoices (order_id, order_date, invoice_no, issued_on,
                                         due_on, amount, tax, is_paid)
    SELECT o.id, o.order_date, 'INV' + o.order_no, o.order_date,
           DATEADD(DAY, 30, o.order_date), o.total_amount,
           CAST(o.total_amount * 0.06 AS DECIMAL(12,2)),
           CASE WHEN o.status = 'completed' THEN 1 ELSE 0 END
    FROM sqlkit.orders o
    WHERE o.status <> 'cancelled';

    -- ── 14. 指标表: 一张直接聚合, 一张走带 OUTPUT 参数的存储过程 ────
    INSERT INTO sqlkit_analytics.metrics (metric_key, period_ym, amount, sample_cnt)
    SELECT 'daily_revenue', CONVERT(CHAR(6), o.order_date, 112),
           SUM(o.total_amount), COUNT(*)
    FROM sqlkit.orders o
    GROUP BY CONVERT(CHAR(6), o.order_date, 112);

    DECLARE @rows_written INT = 0;
    EXEC sqlkit_billing.usp_refresh_metrics @year = 2026, @rows_written = @rows_written OUTPUT;

    COMMIT TRANSACTION;
END TRY
BEGIN CATCH
    IF XACT_STATE() <> 0 ROLLBACK TRANSACTION;
    PRINT N'灌数失败: ' + ERROR_MESSAGE();
    THROW;
END CATCH;
GO

-- ── 15. 校验卡片（单一结果集） ─────────────────────────────────────
SELECT N'users'             AS what, COUNT_BIG(*) AS cnt FROM sqlkit.users
UNION ALL SELECT N'merchants',            COUNT_BIG(*) FROM sqlkit.merchants
UNION ALL SELECT N'categories',           COUNT_BIG(*) FROM sqlkit.categories
UNION ALL SELECT N'products',             COUNT_BIG(*) FROM sqlkit.products
UNION ALL SELECT N'orders',               COUNT_BIG(*) FROM sqlkit.orders
UNION ALL SELECT N'order_items',          COUNT_BIG(*) FROM sqlkit.order_items
UNION ALL SELECT N'payments',             COUNT_BIG(*) FROM sqlkit.payments
UNION ALL SELECT N'reviews',              COUNT_BIG(*) FROM sqlkit.reviews
UNION ALL SELECT N'coupons',              COUNT_BIG(*) FROM sqlkit.coupons
UNION ALL SELECT N'coupons_claimed',      COUNT_BIG(*) FROM sqlkit.coupons WHERE claimed_by IS NOT NULL
UNION ALL SELECT N'reservations',         COUNT_BIG(*) FROM sqlkit.reservations
UNION ALL SELECT N'price_book',           COUNT_BIG(*) FROM sqlkit.price_book
UNION ALL SELECT N'price_book_history',   COUNT_BIG(*) FROM sqlkit.price_book_history
UNION ALL SELECT N'analytics_metrics',    COUNT_BIG(*) FROM sqlkit_analytics.metrics
UNION ALL SELECT N'billing_metrics',      COUNT_BIG(*) FROM sqlkit_billing.metrics
UNION ALL SELECT N'invoices',             COUNT_BIG(*) FROM sqlkit_billing.invoices
UNION ALL SELECT N'audit_log',            COUNT_BIG(*) FROM sqlkit.audit_log
ORDER BY what;

DROP TABLE IF EXISTS #nums;
