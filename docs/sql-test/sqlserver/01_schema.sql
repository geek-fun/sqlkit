-- =====================================================================
-- SQLKit SQL Server 压测 1/3 —— Schema / 对象全景
-- 目标库: sqlserver-test:1433/testdb (sa/TestPass!2024, docker ssh-tunnel 栈)
-- 执行顺序: 01_schema.sql → 02_seed.sql → 03_complex_queries.sql
-- 覆盖面板: Schemas / Tables / Columns / Table Info / DDL / Drop / Rename
--           Indexes / Foreign Keys / Triggers / Views / Procedures / Functions
--           Sequences / Types / Partitions
-- 注意: 全部幂等（先 DROP 再 CREATE），可反复执行
-- 运行方式: 本文件含 GO 批分隔符, 请用 sqlcmd 整体执行;
--           在 SQLKit 编辑器里请按 GO 逐批执行（见 README）
-- =====================================================================

-- ── 0. 清场（依赖从叶子到根; T-SQL 无 DROP SCHEMA CASCADE） ────────
-- 库不存在时先建库（sqlcmd 默认连到 master, 所以必须显式切换）
IF DB_ID(N'testdb') IS NULL CREATE DATABASE testdb;
GO
USE testdb;
GO

-- 触发器 / 视图 / 过程 / 函数: 先显式点名, 再用动态 SQL 兜底扫描
DROP TRIGGER IF EXISTS sqlkit.trg_orders_audit;
DROP TRIGGER IF EXISTS sqlkit.trg_products_audit;
DROP TRIGGER IF EXISTS sqlkit.trg_reviews_audit;
DROP TRIGGER IF EXISTS sqlkit.trg_reservations_no_overlap;
DROP TRIGGER IF EXISTS sqlkit.trg_active_products_delete;
DROP VIEW IF EXISTS sqlkit.v_active_products;
DROP VIEW IF EXISTS sqlkit.product_catalog;
DROP VIEW IF EXISTS sqlkit.active_products;
DROP VIEW IF EXISTS sqlkit.v_user_summary;
DROP VIEW IF EXISTS sqlkit_analytics.sales_dashboard;
DROP VIEW IF EXISTS sqlkit_analytics.daily_sales;
DROP VIEW IF EXISTS sqlkit_analytics.product_catalog;
DROP VIEW IF EXISTS sqlkit_billing.v_invoice_aging;
DROP PROCEDURE IF EXISTS sqlkit_billing.usp_refresh_metrics;
DROP PROCEDURE IF EXISTS sqlkit.usp_void_order;
DROP PROCEDURE IF EXISTS sqlkit.usp_claim_coupon;
DROP FUNCTION IF EXISTS sqlkit.fn_top_products;
DROP FUNCTION IF EXISTS sqlkit.fn_products_by_merchant;
DROP FUNCTION IF EXISTS sqlkit.fn_user_ltv;
DROP FUNCTION IF EXISTS sqlkit.get_product_rank;
DROP FUNCTION IF EXISTS sqlkit_analytics.get_product_rank;

-- 时态表: 必须先关掉系统版本控制, 历史表才能单独删除
IF OBJECT_ID(N'sqlkit.price_book', N'U') IS NOT NULL
    ALTER TABLE sqlkit.price_book SET (SYSTEM_VERSIONING = OFF);
GO

-- 表: 从叶子到根, 避免外键阻塞
DROP TABLE IF EXISTS sqlkit.audit_log;
DROP TABLE IF EXISTS sqlkit.reviews;
DROP TABLE IF EXISTS sqlkit.coupons;
DROP TABLE IF EXISTS sqlkit.reservations;
DROP TABLE IF EXISTS sqlkit.payments;
DROP TABLE IF EXISTS sqlkit.order_items;
DROP TABLE IF EXISTS sqlkit_billing.invoices;
DROP TABLE IF EXISTS sqlkit.orders;
DROP TABLE IF EXISTS sqlkit.price_book_history;
DROP TABLE IF EXISTS sqlkit.price_book;
DROP TABLE IF EXISTS sqlkit.products;
DROP TABLE IF EXISTS sqlkit.categories;
DROP TABLE IF EXISTS sqlkit.merchants;
DROP TABLE IF EXISTS sqlkit.users;
DROP TABLE IF EXISTS sqlkit_billing.metrics;
DROP TABLE IF EXISTS sqlkit_analytics.metrics;
GO

-- 动态 SQL 兜底: 三个 schema 下若还有残留对象（视图/过程/函数/触发器）则逐一清掉
DECLARE @sweep NVARCHAR(MAX) = N'';
SELECT @sweep = @sweep
     + N'DROP ' + CASE o.type
                      WHEN 'V'  THEN N'VIEW'
                      WHEN 'P'  THEN N'PROCEDURE'
                      WHEN 'FN' THEN N'FUNCTION'
                      WHEN 'IF' THEN N'FUNCTION'
                      WHEN 'TF' THEN N'FUNCTION'
                      WHEN 'AF' THEN N'AGGREGATE'
                      WHEN 'TR' THEN N'TRIGGER'
                      WHEN 'SN' THEN N'SYNONYM'
                  END
     + N' ' + QUOTENAME(SCHEMA_NAME(o.schema_id)) + N'.' + QUOTENAME(o.name) + N';' + CHAR(10)
FROM sys.objects o
WHERE o.type IN ('V', 'P', 'FN', 'IF', 'TF', 'AF', 'TR', 'SN')
  AND SCHEMA_NAME(o.schema_id) IN (N'sqlkit', N'sqlkit_analytics', N'sqlkit_billing')
  AND NOT (o.type = 'TR' AND o.parent_object_id = 0);   -- 表上的触发器随表一起消失
IF @sweep <> N'' EXEC sys.sp_executesql @sweep;
GO

DROP TYPE IF EXISTS sqlkit.tt_money_amount;
DROP TYPE IF EXISTS sqlkit.order_status_t;
DROP TYPE IF EXISTS sqlkit.cny_price;
DROP SEQUENCE IF EXISTS sqlkit.coupon_code_seq;
GO

-- 分区方案/函数必须在表全部删除之后才能删;
-- T-SQL 的 DROP PARTITION SCHEME / FUNCTION 没有 IF EXISTS 分支 → 动态 SQL 守护
IF EXISTS (SELECT 1 FROM sys.partition_schemes WHERE name = N'ps_orders_by_date')
    EXEC(N'DROP PARTITION SCHEME ps_orders_by_date');
IF EXISTS (SELECT 1 FROM sys.partition_functions WHERE name = N'pf_orders_by_date')
    EXEC(N'DROP PARTITION FUNCTION pf_orders_by_date');
GO

IF SCHEMA_ID(N'sqlkit')           IS NOT NULL EXEC(N'DROP SCHEMA sqlkit');
IF SCHEMA_ID(N'sqlkit_analytics') IS NOT NULL EXEC(N'DROP SCHEMA sqlkit_analytics');
IF SCHEMA_ID(N'sqlkit_billing')   IS NOT NULL EXEC(N'DROP SCHEMA sqlkit_billing');
GO

-- ── 1. 会话 SET 选项（索引视图 / 过滤索引 / 持久化计算列的前置条件） ──
-- sqlcmd 默认 QUOTED_IDENTIFIER OFF, 不设置会直接建不出这些对象
SET ANSI_NULLS ON;
SET ANSI_PADDING ON;
SET ANSI_WARNINGS ON;
SET ARITHABORT ON;
SET CONCAT_NULL_YIELDS_NULL ON;
SET QUOTED_IDENTIFIER ON;
SET NUMERIC_ROUNDABORT OFF;
GO

-- ── 2. Schemas（多 schema 覆盖; 库已在第 0 步建好） ───────────────
IF SCHEMA_ID(N'sqlkit')           IS NULL EXEC(N'CREATE SCHEMA sqlkit');
IF SCHEMA_ID(N'sqlkit_analytics') IS NULL EXEC(N'CREATE SCHEMA sqlkit_analytics');
IF SCHEMA_ID(N'sqlkit_billing')   IS NULL EXEC(N'CREATE SCHEMA sqlkit_billing');
GO

-- ── 3. 用户定义类型（T-SQL 无 enum / domain / 复合类型） ───────────
-- 别名类型替代 PG 的 DOMAIN, 表类型替代 PG 的复合类型
CREATE TYPE sqlkit.cny_price        FROM DECIMAL(12,2) NOT NULL;
CREATE TYPE sqlkit.order_status_t   FROM VARCHAR(16)   NOT NULL;
CREATE TYPE sqlkit.tt_money_amount  AS TABLE (cents BIGINT NOT NULL, currency CHAR(3) NOT NULL);
GO

-- ── 4. 独立序列（Sequences 面板） ──────────────────────────────────
CREATE SEQUENCE sqlkit.coupon_code_seq
    AS BIGINT
    START WITH 100000
    INCREMENT BY 1
    MINVALUE 100000
    MAXVALUE 999999
    CYCLE
    CACHE 50;
GO

-- ── 5. 分区函数 + 分区方案（RANGE RIGHT, 单文件库 → TO [PRIMARY]） ──
CREATE PARTITION FUNCTION pf_orders_by_date (DATE)
    AS RANGE RIGHT FOR VALUES ('2026-02-01', '2026-03-01', '2026-04-01');
GO
CREATE PARTITION SCHEME ps_orders_by_date
    AS PARTITION pf_orders_by_date ALL TO ([PRIMARY]);
GO

-- ── 6. sqlkit.users（长列 + 全类型覆盖 + 计算列 + 过滤索引） ────────
CREATE TABLE sqlkit.users (
    id            BIGINT IDENTITY(1,1) NOT NULL,
    username      NVARCHAR(64)  NOT NULL,
    email         NVARCHAR(320) NOT NULL,
    phone         VARCHAR(20)   SPARSE NULL,
    password_hash CHAR(64)      NOT NULL,
    locale        CHAR(5)       NOT NULL CONSTRAINT df_users_locale   DEFAULT ('zh_CN'),
    balance       sqlkit.cny_price NOT NULL CONSTRAINT df_users_balance DEFAULT (0),
    registered_at DATETIMEOFFSET(3) NOT NULL CONSTRAINT df_users_registered DEFAULT (SYSDATETIMEOFFSET()),
    last_login_at DATETIMEOFFSET(3) NULL,
    attrs         NVARCHAR(MAX) NOT NULL CONSTRAINT df_users_attrs    DEFAULT (N'{}'),
    tags          NVARCHAR(400) NOT NULL CONSTRAINT df_users_tags     DEFAULT (N'[]'),
    signup_ip     VARCHAR(45)   NULL,
    geo_lat       DECIMAL(9,6)  SPARSE NULL,
    geo_lon       DECIMAL(9,6)  SPARSE NULL,
    avatar        VARBINARY(MAX) NULL,
    bio           XML           NULL,
    wallet_cents  BIGINT        NULL,
    wallet_ccy    CHAR(3)       NULL,
    is_active     BIT           NOT NULL CONSTRAINT df_users_active   DEFAULT (1),
    row_guid      UNIQUEIDENTIFIER ROWGUIDCOL NOT NULL CONSTRAINT df_users_guid DEFAULT (NEWSEQUENTIALID()),
    -- 非持久化计算列
    display_name  AS (LEFT(username, 3) + N'***'),
    -- 持久化计算列（可建索引）
    email_domain  AS (CONVERT(NVARCHAR(128), SUBSTRING(email, CHARINDEX(N'@', email) + 1, 128))) PERSISTED,
    created_at    DATETIME2(3)  NOT NULL DEFAULT (SYSDATETIME()),  -- 未命名 DEFAULT 约束
    updated_at    DATETIME2(3)  NOT NULL DEFAULT (SYSDATETIME()),
    CONSTRAINT pk_users PRIMARY KEY CLUSTERED (id),
    CONSTRAINT ck_users_username_len CHECK (LEN(username) BETWEEN 2 AND 64),
    CONSTRAINT ck_users_email_fmt    CHECK (email LIKE N'%@%.%'),
    CONSTRAINT ck_users_attrs_json   CHECK (ISJSON(attrs) = 1),
    CONSTRAINT ck_users_tags_json    CHECK (ISJSON(tags)  = 1),
    CONSTRAINT ck_users_wallet_ccy   CHECK (wallet_ccy IS NULL OR wallet_ccy LIKE '[A-Z][A-Z][A-Z]'),
    CONSTRAINT ck_users_geo          CHECK (geo_lat IS NULL OR geo_lat BETWEEN -90 AND 90),
    CONSTRAINT uq_users_email        UNIQUE NONCLUSTERED (email),
    CONSTRAINT uq_users_username     UNIQUE NONCLUSTERED (username)
);
GO
-- 过滤索引（filtered index）+ 包含列（INCLUDE）
CREATE NONCLUSTERED INDEX ix_users_active_email
    ON sqlkit.users (email) INCLUDE (username, balance) WHERE is_active = 1;
-- 持久化计算列上的索引 + 非聚集降序索引
CREATE NONCLUSTERED INDEX ix_users_email_domain ON sqlkit.users (email_domain);
CREATE NONCLUSTERED INDEX ix_users_registered   ON sqlkit.users (registered_at DESC) INCLUDE (username);
CREATE NONCLUSTERED INDEX ix_users_balance      ON sqlkit.users (balance DESC, id ASC);
GO

-- ── 7. sqlkit.merchants（命名 + 默认命名唯一约束 + CHECK） ─────────
CREATE TABLE sqlkit.merchants (
    id            BIGINT IDENTITY(1,1) NOT NULL,
    name          NVARCHAR(80)  NOT NULL,
    commission    DECIMAL(5,4)  NOT NULL CONSTRAINT df_merchants_commission DEFAULT (0.0300),
    contact_email NVARCHAR(320) NOT NULL,
    joined_at     DATETIME2(3)  NOT NULL CONSTRAINT df_merchants_joined DEFAULT (SYSDATETIME()),
    verified      BIT           NOT NULL CONSTRAINT df_merchants_verified DEFAULT (0),
    CONSTRAINT pk_merchants PRIMARY KEY CLUSTERED (id),
    CONSTRAINT ck_merchants_commission CHECK (commission BETWEEN 0 AND 0.5),
    CONSTRAINT uq_merchants_name UNIQUE (name),
    UNIQUE (contact_email)                 -- 未命名 → 由系统生成默认名
);
GO

-- ── 8. sqlkit.categories（自引用树 → 递归 CTE 压测） ───────────────
CREATE TABLE sqlkit.categories (
    id       INT IDENTITY(1,1) NOT NULL,
    parent_id INT NULL,
    name     NVARCHAR(60) NOT NULL,
    span_x1  DECIMAL(9,4) SPARSE NULL,     -- PG 的 lseg 替代为两个 SPARSE 列
    span_x2  DECIMAL(9,4) SPARSE NULL,
    is_leaf  BIT NOT NULL CONSTRAINT df_categories_is_leaf DEFAULT (0),
    CONSTRAINT pk_categories PRIMARY KEY CLUSTERED (id),
    -- 自引用外键: T-SQL 不允许自引用 + ON DELETE SET NULL（多重级联路径限制）
    CONSTRAINT fk_categories_parent FOREIGN KEY (parent_id)
        REFERENCES sqlkit.categories (id) ON DELETE NO ACTION
);
GO

-- ── 9. sqlkit.products（复合主键 + 双外键 + 计算列 + 过滤索引） ────
CREATE TABLE sqlkit.products (
    id          BIGINT IDENTITY(1,1) NOT NULL,
    merchant_id BIGINT NOT NULL,
    category_id INT    NULL,
    sku         VARCHAR(40)   NOT NULL,
    name        NVARCHAR(200) NOT NULL,
    condition_t VARCHAR(16)   NOT NULL CONSTRAINT df_products_condition DEFAULT ('new'),
    price       sqlkit.cny_price NOT NULL,
    stock       INT           NOT NULL CONSTRAINT df_products_stock DEFAULT (0),
    weight_g    DECIMAL(8,1)  SPARSE NULL,
    dim_cents   BIGINT        NULL,
    dim_ccy     CHAR(3)       NULL,
    specs       NVARCHAR(MAX) NOT NULL CONSTRAINT df_products_specs DEFAULT (N'{}'),
    tags        NVARCHAR(400) NOT NULL CONSTRAINT df_products_tags  DEFAULT (N'[]'),
    is_active   BIT           NOT NULL CONSTRAINT df_products_active DEFAULT (1),
    created_at  DATETIME2(3)  NOT NULL DEFAULT (SYSDATETIME()),   -- 未命名 DEFAULT
    updated_at  DATETIME2(3)  NOT NULL DEFAULT (SYSDATETIME()),
    -- 持久化计算列（毛利）
    line_margin AS (CAST(price * 0.1500 AS DECIMAL(12,2))) PERSISTED,
    CONSTRAINT pk_products PRIMARY KEY CLUSTERED (id, merchant_id),   -- 复合主键
    CONSTRAINT uq_products_id UNIQUE NONCLUSTERED (id),               -- 供单列 FK 引用
    CONSTRAINT fk_products_merchant FOREIGN KEY (merchant_id)
        REFERENCES sqlkit.merchants (id) ON UPDATE CASCADE ON DELETE NO ACTION,
    CONSTRAINT ck_products_condition CHECK (condition_t IN ('new', 'refurbished', 'used')),
    CONSTRAINT ck_products_price     CHECK (price > 0),
    CONSTRAINT ck_products_stock     CHECK (stock >= 0),
    CONSTRAINT ck_products_specs_json CHECK (ISJSON(specs) = 1),
    CONSTRAINT ck_products_tags_json  CHECK (ISJSON(tags)  = 1)
);
GO
CREATE NONCLUSTERED INDEX ix_products_category ON sqlkit.products (category_id);
CREATE UNIQUE NONCLUSTERED INDEX ux_products_sku_merchant ON sqlkit.products (sku, merchant_id);
-- 过滤索引 + INCLUDE 覆盖列
CREATE NONCLUSTERED INDEX ix_products_active_price
    ON sqlkit.products (price DESC) INCLUDE (name, stock, merchant_id)
    WHERE is_active = 1 AND price > 100;
CREATE NONCLUSTERED INDEX ix_products_line_margin ON sqlkit.products (line_margin) INCLUDE (sku);
GO

-- ── 10. sqlkit.orders（RANGE RIGHT 分区 + 复合主键） ───────────────
CREATE TABLE sqlkit.orders (
    id            BIGINT IDENTITY(1,1) NOT NULL,
    order_no      NVARCHAR(32) NOT NULL,
    user_id       BIGINT NOT NULL,
    status        sqlkit.order_status_t NOT NULL CONSTRAINT df_orders_status DEFAULT ('draft'),
    order_date    DATE NOT NULL,
    total_amount  DECIMAL(12,2) NOT NULL CONSTRAINT df_orders_total    DEFAULT (0),
    discount      DECIMAL(12,2) NOT NULL CONSTRAINT df_orders_discount DEFAULT (0),
    shipping_fee  DECIMAL(10,2) NOT NULL CONSTRAINT df_orders_ship_fee DEFAULT (0),
    shipping_addr NVARCHAR(MAX) NOT NULL,
    remarks       NVARCHAR(400) NULL,
    placed_at     DATETIME2(3)  NOT NULL CONSTRAINT df_orders_placed DEFAULT (SYSDATETIME()),
    shipped_at    DATETIME2(3)  NULL,
    completed_at  DATETIME2(3)  NULL,
    -- 分区键必须进主键
    CONSTRAINT pk_orders PRIMARY KEY CLUSTERED (id, order_date) ON ps_orders_by_date (order_date),
    CONSTRAINT uq_orders_no UNIQUE NONCLUSTERED (order_no, order_date) ON ps_orders_by_date (order_date),
    CONSTRAINT fk_orders_user FOREIGN KEY (user_id) REFERENCES sqlkit.users (id) ON DELETE NO ACTION,
    CONSTRAINT ck_orders_status CHECK (status IN ('draft', 'paid', 'shipped', 'completed', 'cancelled', 'refunded')),
    CONSTRAINT ck_orders_addr_json CHECK (ISJSON(shipping_addr) = 1),
    CONSTRAINT ck_orders_amount CHECK (total_amount >= 0 AND discount >= 0)
) ON ps_orders_by_date (order_date);
GO
-- 分区对齐的非聚集索引
CREATE NONCLUSTERED INDEX ix_orders_user_status
    ON sqlkit.orders (user_id, status) INCLUDE (order_no, total_amount)
    ON ps_orders_by_date (order_date);
CREATE NONCLUSTERED INDEX ix_orders_placed_at ON sqlkit.orders (placed_at) ON ps_orders_by_date (order_date);
GO

-- ── 11. sqlkit.order_items（复合外键 + 持久化计算列） ──────────────
CREATE TABLE sqlkit.order_items (
    id         BIGINT IDENTITY(1,1) NOT NULL,
    order_id   BIGINT NOT NULL,
    order_date DATE   NOT NULL,
    product_id BIGINT NOT NULL,
    quantity   INT    NOT NULL,
    unit_price DECIMAL(12,2) NOT NULL,
    line_total AS (CAST(quantity * unit_price AS DECIMAL(14,2))) PERSISTED,
    CONSTRAINT pk_order_items PRIMARY KEY CLUSTERED (id, order_date),
    -- 复合外键 → 分区父表的复合主键
    CONSTRAINT fk_items_order FOREIGN KEY (order_id, order_date)
        REFERENCES sqlkit.orders (id, order_date) ON DELETE CASCADE,
    CONSTRAINT fk_items_product FOREIGN KEY (product_id)
        REFERENCES sqlkit.products (id) ON DELETE NO ACTION,
    CONSTRAINT ck_items_quantity   CHECK (quantity > 0),
    CONSTRAINT ck_items_unit_price CHECK (unit_price > 0)
);
GO
CREATE NONCLUSTERED INDEX ix_order_items_product ON sqlkit.order_items (product_id) INCLUDE (quantity, unit_price);
CREATE NONCLUSTERED INDEX ix_order_items_order   ON sqlkit.order_items (order_id, order_date);
GO

-- ── 12. sqlkit.payments（ON DELETE SET NULL + 默认命名唯一约束） ───
CREATE TABLE sqlkit.payments (
    id              BIGINT IDENTITY(1,1) NOT NULL,
    order_id        BIGINT NULL,
    order_date      DATE   NULL,
    external_no     VARCHAR(48) NOT NULL,
    amount          DECIMAL(12,2) NOT NULL,
    payment_channel VARCHAR(20) NOT NULL,
    status          sqlkit.order_status_t NOT NULL CONSTRAINT df_payments_status DEFAULT ('draft'),
    paid_at         DATETIMEOFFSET(3) NULL,
    refunded_at     DATETIMEOFFSET(3) NULL,
    payload         VARBINARY(MAX) NULL,
    CONSTRAINT pk_payments PRIMARY KEY CLUSTERED (id),
    -- 分区父表不能建单列唯一约束 → 与 order_items 一致走复合外键
    CONSTRAINT uq_payments_external_no UNIQUE NONCLUSTERED (external_no),
    CONSTRAINT ck_payments_amount CHECK (amount > 0),
    CONSTRAINT ck_payments_channel CHECK (payment_channel IN ('alipay', 'wechat', 'unionpay', 'card')),
    -- 一个显式命名、一个由系统默认命名（见 reviews.product_id / coupons.claimed_by）
    CONSTRAINT fk_payments_order FOREIGN KEY (order_id, order_date)
        REFERENCES sqlkit.orders (id, order_date) ON DELETE SET NULL
);
GO

-- ── 13. sqlkit.reservations（CHECK + 唯一约束 + 重叠触发器替代 EXCLUDE） ──
CREATE TABLE sqlkit.reservations (
    id          BIGINT IDENTITY(1,1) NOT NULL,
    resource_id BIGINT NOT NULL,
    valid_from  DATE NOT NULL,
    valid_to    DATE NOT NULL,
    note_cents  BIGINT NULL,
    note_ccy    CHAR(3) NULL,
    CONSTRAINT pk_reservations PRIMARY KEY CLUSTERED (id),
    CONSTRAINT ck_reservations_range CHECK (valid_to > valid_from),
    CONSTRAINT ck_reservations_note  CHECK (note_ccy IS NULL OR note_ccy LIKE '[A-Z][A-Z][A-Z]'),
    CONSTRAINT uq_reservations_slot  UNIQUE NONCLUSTERED (resource_id, valid_from, valid_to),
    UNIQUE NONCLUSTERED (id, resource_id)          -- 未命名唯一约束
);
GO

-- ── 14. reviews / coupons（ON DELETE SET NULL / CASCADE + 未命名 FK） ──
CREATE TABLE sqlkit.reviews (
    id         BIGINT IDENTITY(1,1) NOT NULL,
    payment_id BIGINT NULL,
    user_id    BIGINT NOT NULL,
    -- 内联未命名外键 → 测无约束名的 FK 分组
    product_id BIGINT NOT NULL REFERENCES sqlkit.products (id),
    rating     TINYINT NOT NULL,
    content    NVARCHAR(500) NOT NULL,
    is_deleted BIT NOT NULL CONSTRAINT df_reviews_is_deleted DEFAULT (0),
    created_at DATETIME2(3) NOT NULL DEFAULT (SYSDATETIME()),   -- 未命名 DEFAULT
    CONSTRAINT pk_reviews PRIMARY KEY CLUSTERED (id),
    CONSTRAINT ck_reviews_rating CHECK (rating BETWEEN 1 AND 5),
    CONSTRAINT fk_reviews_payment FOREIGN KEY (payment_id)
        REFERENCES sqlkit.payments (id) ON DELETE SET NULL,
    CONSTRAINT fk_reviews_user FOREIGN KEY (user_id)
        REFERENCES sqlkit.users (id) ON DELETE NO ACTION
);
GO
CREATE NONCLUSTERED INDEX ix_reviews_rating ON sqlkit.reviews (rating DESC) INCLUDE (product_id, user_id);
CREATE NONCLUSTERED INDEX ix_reviews_active ON sqlkit.reviews (created_at DESC) INCLUDE (rating) WHERE is_deleted = 0;
GO

CREATE TABLE sqlkit.coupons (
    id          BIGINT IDENTITY(1,1) NOT NULL,
    code        VARCHAR(32) NOT NULL CONSTRAINT df_coupons_code DEFAULT (CAST(NEXT VALUE FOR sqlkit.coupon_code_seq AS VARCHAR(32))),
    discount    DECIMAL(12,2) NOT NULL,
    min_amount  DECIMAL(12,2) NOT NULL CONSTRAINT df_coupons_min_amount DEFAULT (0),
    valid_until DATETIMEOFFSET(3) NOT NULL,
    -- 内联未命名外键 + ON DELETE CASCADE
    claimed_by  BIGINT NULL REFERENCES sqlkit.users (id) ON DELETE CASCADE,
    claimed_at  DATETIMEOFFSET(3) NULL,
    CONSTRAINT pk_coupons PRIMARY KEY CLUSTERED (id),
    CONSTRAINT uq_coupons_code UNIQUE NONCLUSTERED (code),
    CONSTRAINT ck_coupons_discount CHECK (discount > 0),
    CONSTRAINT ck_coupons_claim CHECK ((claimed_by IS NULL AND claimed_at IS NULL) OR (claimed_by IS NOT NULL))
);
GO

-- ── 15. sqlkit.audit_log（AFTER 触发器落点） ───────────────────────
CREATE TABLE sqlkit.audit_log (
    id         BIGINT IDENTITY(1,1) NOT NULL,
    table_name SYSNAME      NOT NULL,
    op_type    VARCHAR(10)  NOT NULL,
    row_pk     BIGINT       NULL,
    snapshot   NVARCHAR(MAX) NOT NULL,
    host_user  NVARCHAR(128) NULL CONSTRAINT df_audit_host_user DEFAULT (SUSER_SNAME()),
    at         DATETIME2(3) NOT NULL CONSTRAINT df_audit_at DEFAULT (SYSDATETIME()),
    CONSTRAINT pk_audit_log PRIMARY KEY CLUSTERED (id),
    CONSTRAINT ck_audit_op CHECK (op_type IN ('INSERT', 'UPDATE', 'DELETE')),
    CONSTRAINT ck_audit_snapshot_json CHECK (ISJSON(snapshot) = 1)
);
GO
CREATE NONCLUSTERED INDEX ix_audit_at ON sqlkit.audit_log (at DESC) INCLUDE (table_name, op_type, row_pk);
CREATE NONCLUSTERED INDEX ix_audit_table_op ON sqlkit.audit_log (table_name, op_type) INCLUDE (row_pk);
GO

-- ── 16. sqlkit.price_book（系统版本时态表 → 自动生成历史表） ───────
CREATE TABLE sqlkit.price_book (
    id         BIGINT IDENTITY(1,1) NOT NULL,
    product_id BIGINT NOT NULL,
    list_price DECIMAL(12,2) NOT NULL,
    note       NVARCHAR(200) NULL,
    valid_from DATETIME2 GENERATED ALWAYS AS ROW START NOT NULL,
    valid_to   DATETIME2 GENERATED ALWAYS AS ROW END   NOT NULL,
    PERIOD FOR SYSTEM_TIME (valid_from, valid_to),
    CONSTRAINT pk_price_book PRIMARY KEY CLUSTERED (id)
)
WITH (SYSTEM_VERSIONING = ON (HISTORY_TABLE = sqlkit.price_book_history, DATA_CONSISTENCY_CHECK = ON));
GO
CREATE NONCLUSTERED INDEX ix_price_book_product ON sqlkit.price_book (product_id, list_price);
GO

-- ── 17. 跨 schema 同名表（schema 作用域校验） ──────────────────────
CREATE TABLE sqlkit_analytics.metrics (
    metric_key   VARCHAR(64) NOT NULL,
    period_ym    CHAR(6)     NOT NULL,
    amount       DECIMAL(18,2) NOT NULL CONSTRAINT df_an_metrics_amount DEFAULT (0),
    sample_cnt   BIGINT      NOT NULL CONSTRAINT df_an_metrics_sample DEFAULT (0),
    refreshed_at DATETIME2(3) NOT NULL CONSTRAINT df_an_metrics_at DEFAULT (SYSDATETIME()),
    CONSTRAINT pk_analytics_metrics PRIMARY KEY CLUSTERED (metric_key, period_ym)
);
GO
CREATE TABLE sqlkit_billing.metrics (
    metric_key   VARCHAR(64) NOT NULL,
    period_ym    CHAR(6)     NOT NULL,
    amount       DECIMAL(18,2) NOT NULL CONSTRAINT df_bi_metrics_amount DEFAULT (0),
    currency     CHAR(3)     NOT NULL CONSTRAINT df_bi_metrics_ccy DEFAULT ('CNY'),
    refreshed_at DATETIME2(3) NOT NULL CONSTRAINT df_bi_metrics_at DEFAULT (SYSDATETIME()),
    CONSTRAINT pk_billing_metrics PRIMARY KEY CLUSTERED (metric_key, period_ym)
);
GO

-- ── 18. sqlkit_billing.invoices（跨 schema 复合外键 + 级联） ───────
CREATE TABLE sqlkit_billing.invoices (
    id         BIGINT IDENTITY(1,1) NOT NULL,
    order_id   BIGINT NOT NULL,
    order_date DATE   NOT NULL,
    invoice_no VARCHAR(32) NOT NULL,
    issued_on  DATE NOT NULL,
    due_on     DATE NOT NULL,
    amount     DECIMAL(12,2) NOT NULL,
    tax        DECIMAL(12,2) NOT NULL CONSTRAINT df_invoices_tax DEFAULT (0),
    is_paid    BIT NOT NULL CONSTRAINT df_invoices_paid DEFAULT (0),
    CONSTRAINT pk_invoices PRIMARY KEY CLUSTERED (id),
    CONSTRAINT uq_invoices_no UNIQUE NONCLUSTERED (invoice_no),
    CONSTRAINT fk_invoices_order FOREIGN KEY (order_id, order_date)
        REFERENCES sqlkit.orders (id, order_date) ON DELETE CASCADE,
    CONSTRAINT ck_invoices_amount CHECK (amount >= 0),
    CONSTRAINT ck_invoices_due CHECK (due_on >= issued_on)
);
GO

-- ── 19. 注释（扩展属性 MS_Description，代替 PG 的 COMMENT ON） ──────
EXEC sys.sp_addextendedproperty @name = N'MS_Description', @value = N'核心交易域: 用户/商品/订单',
    @level0type = N'SCHEMA', @level0name = N'sqlkit';
EXEC sys.sp_addextendedproperty @name = N'MS_Description', @value = N'报表域',
    @level0type = N'SCHEMA', @level0name = N'sqlkit_analytics';
EXEC sys.sp_addextendedproperty @name = N'MS_Description', @value = N'结算域',
    @level0type = N'SCHEMA', @level0name = N'sqlkit_billing';
EXEC sys.sp_addextendedproperty @name = N'MS_Description', @value = N'平台注册用户（含 PII 测试字段）',
    @level0type = N'SCHEMA', @level0name = N'sqlkit',
    @level1type = N'TABLE',  @level1name = N'users';
EXEC sys.sp_addextendedproperty @name = N'MS_Description', @value = N'钱包余额（人民币）',
    @level0type = N'SCHEMA', @level0name = N'sqlkit',
    @level1type = N'TABLE',  @level1name = N'users',
    @level2type = N'COLUMN', @level2name = N'balance';
EXEC sys.sp_addextendedproperty @name = N'MS_Description', @value = N'登录邮箱',
    @level0type = N'SCHEMA', @level0name = N'sqlkit',
    @level1type = N'TABLE',  @level1name = N'users',
    @level2type = N'COLUMN', @level2name = N'email';
EXEC sys.sp_addextendedproperty @name = N'MS_Description', @value = N'订单生命周期状态机',
    @level0type = N'SCHEMA', @level0name = N'sqlkit',
    @level1type = N'TABLE',  @level1name = N'orders',
    @level2type = N'COLUMN', @level2name = N'status';
GO

-- ── 20. 函数（标量 / 内联 TVF / 多语句 TVF + 跨 schema 同名） ──────
-- 标量 UDF
CREATE FUNCTION sqlkit.fn_user_ltv (@user_id BIGINT)
RETURNS DECIMAL(14,2)
AS
BEGIN
    DECLARE @total DECIMAL(14,2);
    SELECT @total = ISNULL(SUM(total_amount), 0)
    FROM sqlkit.orders
    WHERE user_id = @user_id AND status IN ('paid', 'completed', 'shipped');
    RETURN ISNULL(@total, 0);
END;
GO

-- 内联表值函数
CREATE FUNCTION sqlkit.get_product_rank (@product_id BIGINT)
RETURNS TABLE
AS
RETURN
(
    SELECT oi.product_id,
           SUM(CAST(oi.quantity AS BIGINT)) AS sold_qty,
           SUM(oi.quantity * oi.unit_price) AS revenue
    FROM sqlkit.order_items oi
    WHERE oi.product_id = @product_id
    GROUP BY oi.product_id
);
GO

-- 跨 schema 同名函数（schema 作用域校验; T-SQL 不支持函数重载, 故同 schema 内用不同名）
CREATE FUNCTION sqlkit_analytics.get_product_rank (@top_n INT)
RETURNS TABLE
AS
RETURN
(
    SELECT TOP (@top_n)
           oi.product_id,
           SUM(CAST(oi.quantity AS BIGINT)) AS sold_qty,
           SUM(oi.quantity * oi.unit_price) AS revenue
    FROM sqlkit.order_items oi
    GROUP BY oi.product_id
    ORDER BY SUM(oi.quantity) DESC
);
GO

-- 单语句内联 TVF: 某商户的在售商品
CREATE FUNCTION sqlkit.fn_products_by_merchant (@merchant_id BIGINT)
RETURNS TABLE
AS
RETURN
(
    SELECT p.id, p.sku, p.name, p.price, p.stock
    FROM sqlkit.products p
    WHERE p.merchant_id = @merchant_id AND p.is_active = 1
);
GO

-- 多语句表值函数
CREATE FUNCTION sqlkit.fn_top_products (@top_n INT)
RETURNS @result TABLE (
    product_id BIGINT,
    product_name NVARCHAR(200),
    sold_qty   BIGINT,
    revenue    DECIMAL(18,2)
)
AS
BEGIN
    INSERT INTO @result (product_id, product_name, sold_qty, revenue)
    SELECT TOP (@top_n)
           oi.product_id,
           p.name,
           SUM(CAST(oi.quantity AS BIGINT)),
           SUM(oi.quantity * oi.unit_price)
    FROM sqlkit.order_items oi
    JOIN sqlkit.products p ON p.id = oi.product_id
    GROUP BY oi.product_id, p.name
    ORDER BY SUM(oi.quantity) DESC;
    RETURN;
END;
GO

-- ── 21. 存储过程（输入参数 + OUTPUT 参数） ─────────────────────────
CREATE PROCEDURE sqlkit.usp_claim_coupon
    @code    VARCHAR(32),
    @user_id BIGINT,
    @claimed BIT = 0 OUTPUT
AS
BEGIN
    SET NOCOUNT ON;
    UPDATE sqlkit.coupons
       SET claimed_by = @user_id,
           claimed_at = SYSDATETIMEOFFSET()
     WHERE code = @code
       AND claimed_by IS NULL
       AND valid_until > SYSDATETIMEOFFSET();
    SET @claimed = CASE WHEN @@ROWCOUNT > 0 THEN 1 ELSE 0 END;
END;
GO

CREATE PROCEDURE sqlkit.usp_void_order
    @order_no NVARCHAR(32),
    @affected INT = 0 OUTPUT
AS
BEGIN
    SET NOCOUNT ON;
    UPDATE sqlkit.orders
       SET status = 'cancelled'
     WHERE order_no = @order_no
       AND status IN ('draft', 'paid');
    SET @affected = @@ROWCOUNT;
END;
GO

CREATE PROCEDURE sqlkit_billing.usp_refresh_metrics
    @year         INT,
    @rows_written INT = 0 OUTPUT
AS
BEGIN
    SET NOCOUNT ON;
    MERGE sqlkit_billing.metrics AS t
    USING (
        SELECT 'monthly_revenue' AS metric_key,
               CONVERT(CHAR(6), o.order_date, 112) AS period_ym,
               SUM(o.total_amount) AS amount
        FROM sqlkit.orders o
        WHERE YEAR(o.order_date) = @year
          AND o.status IN ('paid', 'completed', 'shipped')
        GROUP BY CONVERT(CHAR(6), o.order_date, 112)
    ) AS s
    ON t.metric_key = s.metric_key AND t.period_ym = s.period_ym
    WHEN MATCHED THEN
        UPDATE SET amount = s.amount, refreshed_at = SYSDATETIME()
    WHEN NOT MATCHED THEN
        INSERT (metric_key, period_ym, amount) VALUES (s.metric_key, s.period_ym, s.amount);
    SET @rows_written = @@ROWCOUNT;
END;
GO

-- ── 22. 触发器（AFTER INSERT/UPDATE/DELETE 审计 + INSTEAD OF + 重叠兜底） ──
-- 审计触发器: 必须靠 inserted / deleted 是否为空区分三种操作
-- （UPDATE 时 inserted 与 deleted 同时非空, 不能只看 inserted）
CREATE TRIGGER sqlkit.trg_orders_audit
ON sqlkit.orders
AFTER INSERT, UPDATE, DELETE
AS
BEGIN
    SET NOCOUNT ON;

    -- INSERT: 只有 inserted
    IF EXISTS (SELECT 1 FROM inserted) AND NOT EXISTS (SELECT 1 FROM deleted)
    BEGIN
        INSERT INTO sqlkit.audit_log (table_name, op_type, row_pk, snapshot)
        SELECT N'orders', N'INSERT', i.id,
               (SELECT i.id, i.order_no, i.user_id, i.status, i.order_date, i.total_amount
                FOR JSON PATH, WITHOUT_ARRAY_WRAPPER)
        FROM inserted i;
    END

    -- UPDATE: inserted 与 deleted 同时非空
    IF EXISTS (SELECT 1 FROM inserted) AND EXISTS (SELECT 1 FROM deleted)
    BEGIN
        INSERT INTO sqlkit.audit_log (table_name, op_type, row_pk, snapshot)
        SELECT N'orders', N'UPDATE', i.id,
               (SELECT i.id, i.order_no, i.user_id, i.status, i.order_date, i.total_amount
                FOR JSON PATH, WITHOUT_ARRAY_WRAPPER)
        FROM inserted i
        JOIN deleted d ON d.id = i.id AND d.order_date = i.order_date;
    END

    -- DELETE: 只有 deleted
    IF NOT EXISTS (SELECT 1 FROM inserted) AND EXISTS (SELECT 1 FROM deleted)
    BEGIN
        INSERT INTO sqlkit.audit_log (table_name, op_type, row_pk, snapshot)
        SELECT N'orders', N'DELETE', d.id,
               (SELECT d.id, d.order_no, d.user_id, d.status, d.order_date, d.total_amount
                FOR JSON PATH, WITHOUT_ARRAY_WRAPPER)
        FROM deleted d;
    END
END;
GO

CREATE TRIGGER sqlkit.trg_products_audit
ON sqlkit.products
AFTER UPDATE
AS
BEGIN
    SET NOCOUNT ON;
    INSERT INTO sqlkit.audit_log (table_name, op_type, row_pk, snapshot)
    SELECT N'products', N'UPDATE', i.id,
           (SELECT i.id, i.merchant_id, i.sku, i.price, i.stock, i.is_active
            FOR JSON PATH, WITHOUT_ARRAY_WRAPPER)
    FROM inserted i
    JOIN deleted d ON d.id = i.id
    WHERE i.price <> d.price OR i.stock <> d.stock;
END;
GO

CREATE TRIGGER sqlkit.trg_reviews_audit
ON sqlkit.reviews
AFTER INSERT
AS
BEGIN
    SET NOCOUNT ON;
    INSERT INTO sqlkit.audit_log (table_name, op_type, row_pk, snapshot)
    SELECT N'reviews', N'INSERT', i.id,
           (SELECT i.id, i.user_id, i.product_id, i.rating
            FOR JSON PATH, WITHOUT_ARRAY_WRAPPER)
    FROM inserted i;
END;
GO

-- T-SQL 没有 EXCLUDE 约束 → 用 AFTER 触发器兜底时段互斥（03 的 I 段故意触发它）
CREATE TRIGGER sqlkit.trg_reservations_no_overlap
ON sqlkit.reservations
AFTER INSERT, UPDATE
AS
BEGIN
    SET NOCOUNT ON;
    IF EXISTS (
        SELECT 1
        FROM inserted i
        JOIN sqlkit.reservations r
          ON r.resource_id = i.resource_id
         AND r.id <> i.id
         AND r.valid_from < i.valid_to
         AND i.valid_from < r.valid_to
    )
    BEGIN
        RAISERROR (N'预订时段与该资源既有记录重叠（SQL Server 无 EXCLUDE 约束, 由 trg_reservations_no_overlap 替代）', 16, 1);
    END
END;
GO

-- ── 23. 视图（普通 / WITH CHECK OPTION / 索引视图 / INSTEAD OF 触发器） ──
CREATE VIEW sqlkit.active_products AS
SELECT id, merchant_id, category_id, sku, name, price, stock, tags
FROM sqlkit.products
WHERE is_active = 1 AND stock > 0;
GO

CREATE VIEW sqlkit.product_catalog AS
SELECT p.id, p.name, p.price, m.name AS merchant_name
FROM sqlkit.products p
JOIN sqlkit.merchants m ON m.id = p.merchant_id;
GO

-- WITH CHECK OPTION: 透过视图写入必须仍满足视图谓词
CREATE VIEW sqlkit.v_active_products
AS
SELECT * FROM sqlkit.active_products WHERE price < 10000
WITH CHECK OPTION;
GO

CREATE VIEW sqlkit.v_user_summary AS
SELECT u.id AS user_id,
       u.username,
       COUNT(o.id) AS order_cnt,
       ISNULL(SUM(o.total_amount), 0) AS lifetime_amount
FROM sqlkit.users u
LEFT JOIN sqlkit.orders o ON o.user_id = u.id
GROUP BY u.id, u.username;
GO

CREATE VIEW sqlkit_analytics.product_catalog AS
SELECT p.id, p.sku, p.name, p.price, c.name AS category_name, m.name AS merchant_name
FROM sqlkit.products p
LEFT JOIN sqlkit.categories c ON c.id = p.category_id
JOIN sqlkit.merchants m ON m.id = p.merchant_id;
GO

CREATE VIEW sqlkit_billing.v_invoice_aging AS
SELECT i.id, i.invoice_no, i.issued_on, i.due_on, i.amount, i.is_paid,
       DATEDIFF(DAY, i.due_on, CAST(SYSDATETIME() AS DATE)) AS overdue_days
FROM sqlkit_billing.invoices i;
GO

-- 索引视图（WITH SCHEMABINDING + 唯一聚集索引）替代 PG 的物化视图
CREATE VIEW sqlkit_analytics.daily_sales WITH SCHEMABINDING AS
SELECT o.order_date                 AS sales_day,
       COUNT_BIG(*)                 AS order_cnt,
       SUM(oi.quantity)             AS units_sold,
       SUM(oi.quantity * oi.unit_price) AS revenue
FROM sqlkit.orders o
JOIN sqlkit.order_items oi
  ON oi.order_id = o.id AND oi.order_date = o.order_date
GROUP BY o.order_date;
GO
CREATE UNIQUE CLUSTERED INDEX ucx_daily_sales_day ON sqlkit_analytics.daily_sales (sales_day);
CREATE NONCLUSTERED INDEX ix_daily_sales_revenue ON sqlkit_analytics.daily_sales (revenue DESC);
GO

CREATE VIEW sqlkit_analytics.sales_dashboard AS
SELECT sales_day, order_cnt, revenue FROM sqlkit_analytics.daily_sales
UNION ALL
SELECT CAST(SYSDATETIME() AS DATE), CAST(0 AS BIGINT), CAST(0 AS DECIMAL(38,2));
GO

-- INSTEAD OF 触发器: 视图上的删除改为软删除 + 审计
-- 注意: 带 WITH CHECK OPTION 的视图不允许挂 INSTEAD OF 触发器, 故挂在 active_products 上
CREATE TRIGGER sqlkit.trg_active_products_delete
ON sqlkit.active_products
INSTEAD OF DELETE
AS
BEGIN
    SET NOCOUNT ON;
    UPDATE p
       SET p.is_active = 0,
           p.updated_at = SYSDATETIME()
    FROM sqlkit.products p
    JOIN deleted d ON d.id = p.id AND d.merchant_id = p.merchant_id;

    INSERT INTO sqlkit.audit_log (table_name, op_type, row_pk, snapshot)
    SELECT N'v_active_products', N'DELETE', d.id,
           (SELECT d.id, d.sku, N'soft-delete' AS action FOR JSON PATH, WITHOUT_ARRAY_WRAPPER)
    FROM deleted d;
END;
GO

-- 完成。数据灌入请依次执行 02_seed.sql → 03_complex_queries.sql
