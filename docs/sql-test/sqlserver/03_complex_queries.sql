-- =====================================================================
-- SQLKit SQL Server 压测 3/3 —— 复杂查询演练场
-- 前置: 01_schema.sql + 02_seed.sql 已执行, 且连接库为 testdb
-- 用途: 在 SQLKit 的查询编辑器里逐段执行,验证执行/结果集/耗时/报错
--       路径覆盖: 递归 CTE、窗口函数、CROSS APPLY、聚合集、PIVOT/UNPIVOT、
--                 MERGE、事务+保存点、TRY/CATCH、执行计划、JSON、
--                 OUTPUT 子句、OFFSET/FETCH 分页、分区路由
-- 约束: 本文件不含 GO,每个 A..T 段都是自包含的单个批次 —— SQLKit 的
--       SQL Server 适配器把编辑器内容一次性发给服务器,段内不能再分批
-- 注意: I 段是唯一一处「故意报错」,详见该段注释
-- =====================================================================


-- ── A. 多 CTE 商业报表: 周复购 + 客单价分位 ────────────────────────
WITH weekly AS (
    SELECT o.user_id,
           DATETRUNC(week, o.order_date) AS wk,
           COUNT_BIG(*)        AS orders,
           SUM(o.total_amount) AS spend
    FROM sqlkit.orders o
    WHERE o.status IN ('paid', 'completed', 'shipped')
    GROUP BY o.user_id, DATETRUNC(week, o.order_date)
),
repurchase AS (
    SELECT w.user_id,
           COUNT(DISTINCT w.wk) AS active_weeks,
           SUM(w.orders)        AS total_orders,
           SUM(w.spend)         AS total_spend
    FROM weekly w
    GROUP BY w.user_id
)
SELECT TOP (25)
       r.user_id,
       u.username,
       r.active_weeks,
       r.total_orders,
       CAST(r.total_spend AS DECIMAL(12,2))                     AS total_spend,
       PERCENT_RANK() OVER (ORDER BY r.total_spend)             AS spend_pct,
       CASE WHEN r.total_spend > 4000 THEN 'whale'
            WHEN r.total_spend > 1000 THEN 'big'
            ELSE 'small' END                                    AS segment
FROM repurchase r
JOIN sqlkit.users u ON u.id = r.user_id
ORDER BY r.total_spend DESC;


-- ── B. 递归 CTE 分类树（自引用） ───────────────────────────────────
WITH tree AS (
    SELECT c.id, c.parent_id, c.name, 0 AS lvl,
           CAST(c.name AS NVARCHAR(400)) AS path
    FROM sqlkit.categories c
    WHERE c.parent_id IS NULL
    UNION ALL
    SELECT c.id, c.parent_id, c.name, t.lvl + 1,
           CAST(t.path + N' → ' + c.name AS NVARCHAR(400))
    FROM sqlkit.categories c
    JOIN tree t ON c.parent_id = t.id
)
SELECT t.id,
       t.lvl,
       t.path,
       COUNT(p.id)              AS product_cnt,
       ISNULL(SUM(p.stock), 0)  AS stock_sum
FROM tree t
LEFT JOIN sqlkit.products p ON p.category_id = t.id
GROUP BY t.id, t.lvl, t.path
ORDER BY t.path
OPTION (MAXRECURSION 100);


-- ── C. CROSS APPLY: 每商户 Top-3 销量商品（T-SQL 的 LATERAL） ──────
SELECT m.name AS merchant,
       top3.product_id,
       top3.product_name,
       top3.sold,
       RANK() OVER (PARTITION BY m.id ORDER BY top3.sold DESC) AS rnk
FROM sqlkit.merchants m
CROSS APPLY (
    SELECT TOP (3) p.id AS product_id, p.name AS product_name,
           SUM(oi.quantity) AS sold
    FROM sqlkit.products p
    JOIN sqlkit.order_items oi ON oi.product_id = p.id
    WHERE p.merchant_id = m.id
    GROUP BY p.id, p.name
    ORDER BY SUM(oi.quantity) DESC, p.id
) top3
ORDER BY m.name, rnk;


-- ── D. 窗口函数全家桶: 分区移动/滑动平均/间隙滞后 ──────────────────
SELECT TOP (60)
       o.order_date,
       o.id,
       o.total_amount,
       SUM(o.total_amount) OVER (PARTITION BY o.order_date)                AS day_total,
       CAST(AVG(o.total_amount) OVER (PARTITION BY o.user_id
            ORDER BY o.order_date, o.id
            ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS DECIMAL(12,2))   AS moving_avg3,
       LAG(o.total_amount)  OVER (PARTITION BY o.user_id ORDER BY o.order_date, o.id) AS prev_amount,
       LEAD(o.total_amount) OVER (PARTITION BY o.user_id ORDER BY o.order_date, o.id) AS next_amount,
       FIRST_VALUE(o.status) OVER (PARTITION BY o.user_id ORDER BY o.order_date, o.id) AS first_status,
       COUNT_BIG(*) OVER (PARTITION BY o.user_id)                          AS user_orders,
       ROW_NUMBER() OVER (PARTITION BY o.user_id ORDER BY o.order_date, o.id) AS user_seq
FROM sqlkit.orders o
ORDER BY o.user_id, o.order_date, o.id;


-- ── E. GROUPING SETS / CUBE / ROLLUP（T-SQL 无 FILTER 子句 → CASE 聚合） ──
-- T-SQL 不支持 PG 的 count(*) FILTER (WHERE ...) → 用 SUM(CASE WHEN ...) 等价改写
WITH src AS (
    SELECT o.status,
           DATETRUNC(month, o.order_date) AS ym,
           o.total_amount,
           o.user_id
    FROM sqlkit.orders o
)
SELECT CASE WHEN GROUPING(s.status) = 1 THEN N'ALL' ELSE s.status END             AS status,
       CASE WHEN GROUPING(s.ym) = 1 THEN N'ALL'
            ELSE CONVERT(CHAR(7), s.ym, 120) END                                  AS ym,
       SUM(CASE WHEN s.total_amount > 500 THEN 1 ELSE 0 END)                      AS big_orders,
       SUM(CASE WHEN s.status = 'completed' THEN s.total_amount ELSE 0 END)       AS completed_sum,
       COUNT(DISTINCT s.user_id)                                                  AS buyers
FROM src s
GROUP BY GROUPING SETS ((s.status), (s.ym), (s.status, s.ym), ())
ORDER BY status, ym;

-- E2: ROLLUP / CUBE 对照（同一批数据的三种小计粒度）
SELECT CASE WHEN GROUPING(s.status) = 1 THEN N'ALL' ELSE s.status END AS status,
       CASE WHEN GROUPING(s.ym) = 1 THEN N'ALL'
            ELSE CONVERT(CHAR(7), s.ym, 120) END                      AS ym,
       COUNT_BIG(*)        AS order_cnt,
       SUM(s.total_amount) AS amount
FROM (SELECT o.status,
             DATETRUNC(month, o.order_date) AS ym,
             o.total_amount
      FROM sqlkit.orders o) s
GROUP BY CUBE (s.status, s.ym)
ORDER BY status, ym;

-- E3: ROLLUP（按 status 逐级小计, 最外层是总计）
SELECT CASE WHEN GROUPING(s.status) = 1 THEN N'ALL' ELSE s.status END AS status,
       CASE WHEN GROUPING(s.ym) = 1 THEN N'ALL'
            ELSE CONVERT(CHAR(7), s.ym, 120) END                      AS ym,
       COUNT_BIG(*)        AS order_cnt,
       SUM(s.total_amount) AS amount
FROM (SELECT o.status,
             DATETRUNC(month, o.order_date) AS ym,
             o.total_amount
      FROM sqlkit.orders o) s
GROUP BY ROLLUP (s.status, s.ym)
ORDER BY status, ym;


-- ── F. JSON / XML / 类型演练（ISJSON / JSON_VALUE / OPENJSON） ─────
-- XML 数据类型方法与过滤索引同样要求这些 SET 选项
SET ANSI_NULLS ON; SET ANSI_PADDING ON; SET ANSI_WARNINGS ON; SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON; SET QUOTED_IDENTIFIER ON; SET NUMERIC_ROUNDABORT OFF;

SELECT TOP (20)
       u.username,
       JSON_VALUE(u.attrs, '$.city')             AS city,
       JSON_VALUE(u.attrs, '$.vip')              AS is_vip_text,
       CAST(JSON_VALUE(u.attrs, '$.age') AS INT) AS age,
       j.tag_cnt,
       JSON_VALUE(u.tags, '$[0]')                AS first_tag,
       u.signup_ip,
       u.balance,
       u.bio.value('(/bio/nick)[1]', 'NVARCHAR(64)') AS xml_nick
FROM sqlkit.users u
CROSS APPLY (
    SELECT COUNT(*) AS tag_cnt,
           MAX(CASE WHEN [value] = N'reader' THEN 1 ELSE 0 END) AS has_reader
    FROM OPENJSON(u.tags)
) j
WHERE JSON_VALUE(u.attrs, '$.vip') = 'true'
   OR j.has_reader = 1
ORDER BY u.balance DESC;


-- ── G. MERGE upsert（替代 PG 的 ON CONFLICT ... DO NOTHING / DO UPDATE） ──
-- 目标表带触发器时, MERGE 的 OUTPUT 必须配 INTO（否则报错 334）→ 先落表变量再 SELECT
-- G1: 命中即跳过（等价 ON CONFLICT DO NOTHING）, 重复执行不产生新行
DECLARE @merge_res TABLE (merge_action NVARCHAR(10), id BIGINT, resource_id BIGINT,
                         valid_from DATE, valid_to DATE);

MERGE sqlkit.reservations AS t
USING (SELECT CAST(1 AS BIGINT) AS resource_id,
              CAST('2026-07-01' AS DATE) AS valid_from,
              CAST('2026-07-05' AS DATE) AS valid_to,
              CAST(500 AS BIGINT) AS note_cents,
              CAST('CNY' AS CHAR(3)) AS note_ccy) AS s
   ON t.resource_id = s.resource_id
  AND t.valid_from  = s.valid_from
  AND t.valid_to    = s.valid_to
WHEN NOT MATCHED THEN
    INSERT (resource_id, valid_from, valid_to, note_cents, note_ccy)
    VALUES (s.resource_id, s.valid_from, s.valid_to, s.note_cents, s.note_ccy)
OUTPUT $action, inserted.id, inserted.resource_id, inserted.valid_from, inserted.valid_to
INTO @merge_res (merge_action, id, resource_id, valid_from, valid_to);

SELECT * FROM @merge_res;

-- G2: 命中即取较大折扣（等价 ON CONFLICT DO UPDATE + greatest）
DECLARE @merge_coupon TABLE (merge_action NVARCHAR(10), id BIGINT, code VARCHAR(32), discount DECIMAL(12,2));

MERGE sqlkit.coupons AS t
USING (SELECT CAST('SUMMER2026' AS VARCHAR(32)) AS code,
              CAST(30.00 AS DECIMAL(12,2)) AS discount,
              CAST(200.00 AS DECIMAL(12,2)) AS min_amount,
              CAST('2026-08-31T00:00:00' AS DATETIMEOFFSET(3)) AS valid_until) AS s
   ON t.code = s.code
WHEN MATCHED AND t.discount < s.discount THEN
    UPDATE SET discount = s.discount, min_amount = s.min_amount, valid_until = s.valid_until
WHEN NOT MATCHED THEN
    INSERT (code, discount, min_amount, valid_until)
    VALUES (s.code, s.discount, s.min_amount, s.valid_until)
OUTPUT $action, inserted.id, inserted.code, inserted.discount
INTO @merge_coupon (merge_action, id, code, discount);

SELECT * FROM @merge_coupon;
SELECT id, code, discount FROM sqlkit.coupons WHERE code = 'SUMMER2026';


-- ── H. MERGE 库存对账（临时表 + OUTPUT + 事务） ───────────────────
SET ANSI_NULLS ON; SET ANSI_PADDING ON; SET ANSI_WARNINGS ON; SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON; SET QUOTED_IDENTIFIER ON; SET NUMERIC_ROUNDABORT OFF;

IF OBJECT_ID('tempdb..#stock_diff') IS NOT NULL DROP TABLE #stock_diff;
CREATE TABLE #stock_diff (product_id BIGINT NOT NULL, delta INT NOT NULL);
INSERT INTO #stock_diff (product_id, delta) VALUES (1, -3), (2, 5), (1, 1), (99, 2);

-- products 上挂着 trg_products_audit → OUTPUT 必须配 INTO, 结果先收集到表变量
DECLARE @stock_log TABLE (merge_action NVARCHAR(10), id BIGINT, sku VARCHAR(40),
                          stock INT, old_stock INT);

BEGIN TRY
    BEGIN TRANSACTION;
    MERGE sqlkit.products AS p
    USING (SELECT product_id, SUM(delta) AS delta FROM #stock_diff GROUP BY product_id) AS s
       ON p.id = s.product_id
    WHEN MATCHED THEN
        UPDATE SET stock = CASE WHEN p.stock + s.delta < 0 THEN 0 ELSE p.stock + s.delta END,
                   updated_at = SYSDATETIME()
    WHEN NOT MATCHED THEN
        INSERT (merchant_id, sku, name, price, stock)
        VALUES (1,
                'SKU-NEW-' + CAST(s.product_id AS VARCHAR(10)),
                N'自动对账商品-' + CAST(s.product_id AS NVARCHAR(10)),
                9.90,
                CASE WHEN s.delta < 0 THEN 0 ELSE s.delta END)
    OUTPUT $action, inserted.id, inserted.sku, inserted.stock, deleted.stock
    INTO @stock_log (merge_action, id, sku, stock, old_stock);
    COMMIT TRANSACTION;
END TRY
BEGIN CATCH
    IF XACT_STATE() <> 0 ROLLBACK TRANSACTION;
    THROW;
END CATCH;

SELECT * FROM @stock_log ORDER BY id;

SELECT id, sku, name, stock FROM sqlkit.products
WHERE id IN (1, 2, 99) OR sku LIKE 'SKU-NEW-%'
ORDER BY id;

-- 清理本段造出的对账商品, 保证重复执行可复现
DELETE FROM sqlkit.products WHERE sku LIKE 'SKU-NEW-%';
DROP TABLE IF EXISTS #stock_diff;


-- ── I. 事务 + 保存点回滚（⚠ 本文件唯一一处故意报错） ────────────────
-- 故意触发的错误: 第 4 条语句插入的预订时段与已有记录重叠,
-- 由 sqlkit.trg_reservations_no_overlap 抛错（替代 PG 的 EXCLUDE 约束冲突）。
-- ROLLBACK TRANSACTION <保存点> 会把事务从失败状态救回, 之后仍可继续修改并提交。
-- ⚠ 用 sqlcmd -b 执行本段会在该错误处直接退出（退出码 1, 这是预期现象）;
--   去掉 -b（或只在 SQLKit 编辑器里执行）才能看到后面救援成功的查询结果。
SET XACT_ABORT OFF;      -- 关键: 出错后事务仍可提交, 保存点才有意义
SET ANSI_NULLS ON; SET ANSI_PADDING ON; SET ANSI_WARNINGS ON; SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON; SET QUOTED_IDENTIFIER ON; SET NUMERIC_ROUNDABORT OFF;

DELETE FROM sqlkit.users WHERE username = N'tx_probe';
BEGIN TRANSACTION;
    INSERT INTO sqlkit.users (username, email, password_hash)
    VALUES (N'tx_probe', N'txprobe@example.com', REPLICATE('0', 64));

    SAVE TRANSACTION before_bad_insert;
    INSERT INTO sqlkit.reservations (resource_id, valid_from, valid_to, note_cents, note_ccy)
    VALUES (1, '2026-06-03', '2026-06-05', 10, 'CNY');

    ROLLBACK TRANSACTION before_bad_insert;   -- 回滚到保存点, 事务重新可提交
    UPDATE sqlkit.users SET balance = balance + 100 WHERE username = N'tx_probe';
COMMIT TRANSACTION;

SELECT username, balance FROM sqlkit.users WHERE username = N'tx_probe';

-- 保存点回滚应把被拒的那一行彻底清掉 → 期望 0
SELECT COUNT(*) AS bad_row_present
FROM sqlkit.reservations
WHERE resource_id = 1 AND valid_from = '2026-06-03' AND valid_to = '2026-06-05';


-- ── J. TRY...CATCH + THROW 错误处理（替代 PG 的 DO 匿名块） ────────
BEGIN TRY
    DECLARE @stale INT;
    SELECT @stale = COUNT(*)
    FROM sqlkit.orders o
    WHERE o.status = 'paid'
      AND o.placed_at < DATEADD(DAY, -30, SYSDATETIME());
    PRINT N'超期未发货订单: ' + CAST(@stale AS NVARCHAR(10));

    -- 主动抛出可捕获异常, 演示 CATCH 分支与错误上下文函数
    THROW 51000, N'演练用异常: paid 订单超过 30 天未发货', 1;
END TRY
BEGIN CATCH
    SELECT ERROR_NUMBER()    AS err_number,
           ERROR_SEVERITY()  AS err_severity,
           ERROR_STATE()     AS err_state,
           ERROR_PROCEDURE() AS err_procedure,
           ERROR_LINE()      AS err_line,
           ERROR_MESSAGE()   AS err_message;
END CATCH;

SELECT COUNT(*) AS paid_orders FROM sqlkit.orders WHERE status = 'paid';


-- ── K. EXEC 存储过程（OUTPUT 参数） ────────────────────────────────
SET ANSI_NULLS ON; SET ANSI_PADDING ON; SET ANSI_WARNINGS ON; SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON; SET QUOTED_IDENTIFIER ON; SET NUMERIC_ROUNDABORT OFF;

DECLARE @claimed BIT = 0, @affected INT = 0, @affected2 INT = 0;
EXEC sqlkit.usp_claim_coupon @code = '100010', @user_id = 42, @claimed = @claimed OUTPUT;
-- 该单状态为 refunded → 0 行（与 PG 压测数据一致, 只为验证 OUTPUT 参数回传）
EXEC sqlkit.usp_void_order    @order_no = N'SO-20260100017', @affected = @affected OUTPUT;
-- 该单状态为 paid → 1 行, 验证作废真的生效
EXEC sqlkit.usp_void_order    @order_no = N'SO-20260200012', @affected = @affected2 OUTPUT;

SELECT @claimed AS coupon_claimed,
       @affected  AS voided_refunded_order,
       @affected2 AS voided_paid_order;
SELECT code, claimed_by, claimed_at FROM sqlkit.coupons WHERE code = '100010';
SELECT order_no, status, order_date FROM sqlkit.orders
WHERE order_no IN (N'SO-20260100017', N'SO-20260200012')
ORDER BY order_no;

-- 复位本段改动, 保证可重复执行
UPDATE sqlkit.coupons SET claimed_by = NULL, claimed_at = NULL WHERE code = '100010';
UPDATE sqlkit.orders  SET status = 'paid' WHERE order_no = N'SO-20260200012';


-- ── L. 集合操作与反连接: FULL OUTER / INTERSECT / EXCEPT ──────────
-- T-SQL 没有 IS DISTINCT FROM → 用显式 NULL 安全比较替代
SELECT COALESCE(m1.merchant_id, m2.merchant_id) AS merchant_id,
       m1.active_cnt AS via_active_products,
       m2.all_cnt    AS via_products
FROM (SELECT merchant_id, COUNT(*) AS active_cnt
      FROM sqlkit.active_products GROUP BY merchant_id) m1
FULL OUTER JOIN
     (SELECT merchant_id, COUNT(*) AS all_cnt
      FROM sqlkit.products GROUP BY merchant_id) m2
  ON m1.merchant_id = m2.merchant_id
WHERE m1.active_cnt <> m2.all_cnt
   OR m1.active_cnt IS NULL
   OR m2.all_cnt IS NULL
ORDER BY merchant_id;

SELECT p.name FROM sqlkit.products p WHERE p.price > 1000
INTERSECT
SELECT p.name FROM sqlkit.products p WHERE p.stock < 100;

SELECT username FROM sqlkit.users WHERE is_active = 1
EXCEPT
SELECT username FROM sqlkit.users WHERE JSON_VALUE(attrs, '$.vip') = 'true';


-- ── M. PIVOT / UNPIVOT（T-SQL 特有写法） ──────────────────────────
SELECT *
FROM (SELECT o.status,
             CASE MONTH(o.order_date) WHEN 1 THEN 'M01' WHEN 2 THEN 'M02' ELSE 'M03' END AS ym,
             o.total_amount
      FROM sqlkit.orders o) src
PIVOT (SUM(total_amount) FOR ym IN ([M01], [M02], [M03])) AS pvt
ORDER BY status;

-- PIVOT 之后再 UNPIVOT, 回到长表形态
SELECT status, ym, amount
FROM (SELECT o.status,
             CASE MONTH(o.order_date) WHEN 1 THEN 'M01' WHEN 2 THEN 'M02' ELSE 'M03' END AS ym,
             o.total_amount
      FROM sqlkit.orders o) src
PIVOT (SUM(total_amount) FOR ym IN ([M01], [M02], [M03])) AS pvt
UNPIVOT (amount FOR ym IN ([M01], [M02], [M03])) AS unp
ORDER BY status, ym;

-- M3: PG 的 DISTINCT ON (category_id) 在 T-SQL 不存在 → ROW_NUMBER() 取每组最新一条
SELECT t.category_id, t.id, t.name, t.price, t.created_at
FROM (SELECT p.category_id, p.id, p.name, p.price, p.created_at,
             ROW_NUMBER() OVER (PARTITION BY p.category_id
                                ORDER BY p.created_at DESC, p.id) AS rn
      FROM sqlkit.products p
      WHERE p.category_id IS NOT NULL) t
WHERE t.rn = 1
ORDER BY t.category_id;


-- ── N. JSON 聚合: STRING_AGG + FOR JSON PATH ──────────────────────
-- N1: 分组内做字符串聚合与最大值
SELECT JSON_VALUE(o.shipping_addr, '$.province') AS province,
       COUNT_BIG(*)                              AS order_cnt,
       STRING_AGG(o.order_no, N',') WITHIN GROUP (ORDER BY o.total_amount DESC) AS order_csv,
       MAX(o.total_amount)                       AS biggest
FROM sqlkit.orders o
WHERE JSON_VALUE(o.shipping_addr, '$.city') = N'杭州'
GROUP BY JSON_VALUE(o.shipping_addr, '$.province');

-- N2: 每个省份把订单打包成一个 JSON 数组（替代 PG 的 jsonb_agg）
WITH prov AS (
    SELECT DISTINCT JSON_VALUE(o.shipping_addr, '$.province') AS province
    FROM sqlkit.orders o
)
SELECT TOP (6)
       p.province,
       (SELECT o2.order_no AS [order], o2.total_amount AS amount
        FROM sqlkit.orders o2
        WHERE JSON_VALUE(o2.shipping_addr, '$.province') = p.province
        ORDER BY o2.total_amount DESC
        FOR JSON PATH) AS orders_bundle
FROM prov p
ORDER BY p.province;


-- ── O. 执行计划: STATISTICS XML（替代 PG 的 EXPLAIN） ──────────────
-- 也可以用 SET SHOWPLAN_XML ON（只出计划不执行）, 但那必须是批次中的唯一语句;
-- 这里用 SET STATISTICS XML, 语句照常执行并额外返回一个 XML 计划结果集
SET STATISTICS XML ON;

-- O1: 前导通配搜索（非 SARGable）→ 计划里应看到索引/表扫描
SELECT p.name, oi.quantity
FROM sqlkit.products p
JOIN sqlkit.order_items oi ON oi.product_id = p.id
WHERE p.sku LIKE '%SKU-001%'
ORDER BY oi.quantity DESC, p.id;

-- O2: 前缀搜索 → 可命中 ux_products_sku_merchant 的唯一索引
-- （T-SQL 的默认排序规则 SQL_Latin1_General_CP1_CI_AS 本身大小写不敏感,
--   不需要 PG 的 lower(p.sku) 表达式写法）
SELECT p.id, p.sku, p.name
FROM sqlkit.products p
WHERE p.sku LIKE 'SKU-001%';

-- 分区裁剪验证: 单日查询只应落在 orders 的 2 月分区
SELECT COUNT(*) AS cnt
FROM sqlkit.orders
WHERE order_date = '2026-02-10';

SET STATISTICS XML OFF;


-- ── P. UPDATE ... FROM / DELETE ... FROM + OUTPUT 子句 ────────────
SET ANSI_NULLS ON; SET ANSI_PADDING ON; SET ANSI_WARNINGS ON; SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON; SET QUOTED_IDENTIFIER ON; SET NUMERIC_ROUNDABORT OFF;

DECLARE @upd TABLE (id BIGINT, content NVARCHAR(500));
UPDATE r
   SET r.content = r.content + N' [reviewed]'
OUTPUT inserted.id, inserted.content INTO @upd (id, content)
FROM sqlkit.reviews r
JOIN sqlkit.users u ON u.id = r.user_id
WHERE JSON_VALUE(u.attrs, '$.vip') = 'true';

SELECT COUNT(*) AS updated_rows FROM @upd;
SELECT TOP (10) * FROM @upd ORDER BY id;

-- 退款订单的评价整行删除（OUTPUT deleted.* 收集被删主键）
DECLARE @del TABLE (id BIGINT);
DELETE r
OUTPUT deleted.id INTO @del (id)
FROM sqlkit.reviews r
JOIN sqlkit.payments pm ON pm.id = r.payment_id
WHERE pm.status = 'refunded';

SELECT COUNT(*) AS deleted_rows FROM @del;


-- ── Q. 约束立即校验（PG 的 deferrable 在 T-SQL 不存在） ────────────
-- PG 可以用 DEFERRABLE INITIALLY DEFERRED 在同一事务里互换两行唯一键;
-- T-SQL 的唯一约束/唯一索引一律立即校验 → 直接互换必定撞车, 只能借道临时值。
-- 本段故意让第一次互换失败, 但用 TRY/CATCH 捕获, 因此不会向外抛错。
SET XACT_ABORT ON;       -- 出错即让事务进入不可提交状态, 便于演示回滚
SET ANSI_NULLS ON; SET ANSI_PADDING ON; SET ANSI_WARNINGS ON; SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON; SET QUOTED_IDENTIFIER ON; SET NUMERIC_ROUNDABORT OFF;

BEGIN TRY
    BEGIN TRANSACTION;
    UPDATE sqlkit.users SET email = N'user2@mail.example.com' WHERE username = N'user_1';
    UPDATE sqlkit.users SET email = N'user1@mail.example.com' WHERE username = N'user_2';
    COMMIT TRANSACTION;
    SELECT N'意外成功（不应出现）' AS outcome;
END TRY
BEGIN CATCH
    IF XACT_STATE() <> 0 ROLLBACK TRANSACTION;
    SELECT ERROR_NUMBER() AS err_number,
           ERROR_MESSAGE() AS err_message,
           N'T-SQL 唯一约束立即校验, 事务内直接互换两行必然失败' AS note;
END CATCH;

-- 正确姿势: 三步交换法（第一步先落到临时邮箱）
BEGIN TRANSACTION;
    UPDATE sqlkit.users SET email = N'tmp_swap@example.com' WHERE username = N'user_1';
    UPDATE sqlkit.users SET email = N'user1@mail.example.com' WHERE username = N'user_2';
    UPDATE sqlkit.users SET email = N'user2@mail.example.com' WHERE username = N'user_1';
COMMIT TRANSACTION;

-- 再换回原值, 保证本段可重复执行
BEGIN TRANSACTION;
    UPDATE sqlkit.users SET email = N'tmp_swap@example.com' WHERE username = N'user_1';
    UPDATE sqlkit.users SET email = N'user2@mail.example.com' WHERE username = N'user_2';
    UPDATE sqlkit.users SET email = N'user1@mail.example.com' WHERE username = N'user_1';
COMMIT TRANSACTION;

SELECT username, email FROM sqlkit.users
WHERE username IN (N'user_1', N'user_2')
ORDER BY username;


-- ── R. 函数三形态 + 跨 schema 同名函数 ────────────────────────────
-- R1: 标量 UDF
SELECT u.id, u.username, sqlkit.fn_user_ltv(u.id) AS lifetime_value
FROM sqlkit.users u
WHERE u.id <= 5
ORDER BY u.id;

-- R2: 内联 TVF（含跨 schema 同名: sqlkit 与 sqlkit_analytics 各有一个 get_product_rank）
SELECT * FROM sqlkit.get_product_rank(1);
SELECT * FROM sqlkit_analytics.get_product_rank(5);

-- R3: 单语句 TVF / 多语句 TVF
SELECT * FROM sqlkit.fn_products_by_merchant(2);
SELECT * FROM sqlkit.fn_top_products(5);


-- ── S. 窗口框架 ROWS vs RANGE + 7 日滚动（索引视图即时可见） ───────
-- T-SQL 的 RANGE 只支持 UNBOUNDED PRECEDING / CURRENT ROW,
-- 没有 PG 的 RANGE BETWEEN INTERVAL '7 days' PRECEDING → 用 ROWS 或 OUTER APPLY 等价实现。
-- 索引视图由基表 DML 自动维护, 不需要 PG 的 REFRESH MATERIALIZED VIEW。
SELECT d.sales_day,
       d.revenue,
       SUM(d.revenue) OVER (ORDER BY d.sales_day
            ROWS BETWEEN 6 PRECEDING AND CURRENT ROW)      AS rev_7row,
       SUM(d.revenue) OVER (ORDER BY d.sales_day
            RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS rev_cum,
       w.rev_7d
FROM sqlkit_analytics.daily_sales d
OUTER APPLY (
    SELECT SUM(x.revenue) AS rev_7d
    FROM sqlkit_analytics.daily_sales x
    WHERE x.sales_day BETWEEN DATEADD(DAY, -6, d.sales_day) AND d.sales_day
) w
ORDER BY d.sales_day;


-- ── T. 分区路由验证 + OFFSET/FETCH 分页 ───────────────────────────
SET ANSI_NULLS ON; SET ANSI_PADDING ON; SET ANSI_WARNINGS ON; SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON; SET QUOTED_IDENTIFIER ON; SET NUMERIC_ROUNDABORT OFF;

-- T1: 未来日期订单落到第 4 个分区; $PARTITION 直接给出分区号, 验证完即回滚
BEGIN TRANSACTION;
    INSERT INTO sqlkit.orders (order_no, user_id, status, order_date, shipping_addr, placed_at)
    VALUES (N'SO-2026FUTURE', 1, 'draft', '2026-12-25',
            N'{"province":"未知","city":"default"}', SYSDATETIME());

    SELECT $PARTITION.pf_orders_by_date(order_date) AS partition_no,
           COUNT_BIG(*)                             AS order_cnt,
           MIN(order_date)                          AS from_day,
           MAX(order_date)                          AS to_day
    FROM sqlkit.orders
    GROUP BY $PARTITION.pf_orders_by_date(order_date)
    ORDER BY partition_no;

    SELECT $PARTITION.pf_orders_by_date(CAST('2026-02-10' AS DATE)) AS feb_partition,
           $PARTITION.pf_orders_by_date(CAST('2026-12-25' AS DATE)) AS future_partition;
ROLLBACK TRANSACTION;

-- T2: OFFSET / FETCH 分页（PG 的 LIMIT / OFFSET 对应写法）
SELECT o.id, o.order_no, o.order_date, o.total_amount, o.status
FROM sqlkit.orders o
ORDER BY o.id
OFFSET 40 ROWS FETCH NEXT 10 ROWS ONLY;

-- T3: 分页同时给出总行数
SELECT COUNT(*) OVER () AS total_rows,
       o.id, o.order_no, o.status
FROM sqlkit.orders o
ORDER BY o.id
OFFSET 100 ROWS FETCH NEXT 5 ROWS ONLY;

-- 完成。清理演示残余数据（如需彻底复位请重跑 01 → 02）:
DELETE FROM sqlkit.users WHERE username = N'tx_probe';
