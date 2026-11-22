-- =====================================================================
-- SQLKit Oracle 压测 1/3 —— Schema / 对象全景
-- 目标库: oracle-test:1521/FREEPDB1 (sqlkit/testpass, docker ssh-tunnel 栈)
-- 执行顺序: 01_schema.sql → 02_seed.sql → 03_complex_queries.sql
-- 覆盖面板: Schemas / Tables / Columns / Table Info / DDL / Drop / Rename
--           Indexes / Foreign Keys / Triggers / Views / Procedures / Functions
--           Sequences
-- 方言映射: Oracle 一个用户即一个 schema，PG 的 store/analytics/billing 三 schema
--           在此收敛到 SQLKIT 一个 schema：核心交易域沿用原表名，analytics 域
--           加 analytics_ 前缀，billing 域用 billing_invoices 空表镜像。
-- 注意: 全部幂等（先 DROP ... IF EXISTS 再 CREATE），可反复执行。
--       PL/SQL 对象（函数/过程/包/触发器）的定义体必然含内部分号，SQL*Plus 需要
--       紧随其后的 "/" 才能提交；SQLKit 的分号分隔器无法整段执行它们，请在
--       SQLKit 里逐个对象单独执行（详见 README「PL/SQL 对象怎么跑」）。
-- =====================================================================

-- ── 0. 清场（Oracle 23ai 原生支持 IF EXISTS） ──────────────────────
-- 旧版本（12c~19c）没有 DROP ... IF EXISTS，需改写为守卫块，例如:
--   BEGIN EXECUTE IMMEDIATE 'DROP TABLE orders PURGE';
--   EXCEPTION WHEN OTHERS THEN NULL; END;
--   /
-- 这类块含内部分号，会打乱 SQLKit 的分号分隔器，因此本套件统一使用 23ai 的
-- IF EXISTS 形式：单条语句、不含内部分号，SQLKit 与 SQL*Plus 都能正确切分。
DROP MATERIALIZED VIEW IF EXISTS analytics_daily_sales;
DROP VIEW IF EXISTS analytics_sales_dashboard;
DROP VIEW IF EXISTS merchant_sales_summary;
DROP VIEW IF EXISTS shop_with_check;
DROP VIEW IF EXISTS product_catalog;
DROP VIEW IF EXISTS active_products;
DROP TRIGGER IF EXISTS trg_orders_audit;
DROP TRIGGER IF EXISTS trg_products_before_set;
DROP TRIGGER IF EXISTS trg_reviews_statement;
DROP TRIGGER IF EXISTS trg_items_compound;
DROP PACKAGE IF EXISTS coupon_pkg;
DROP PACKAGE IF EXISTS analytics_pkg;
DROP PROCEDURE IF EXISTS refresh_daily_sales;
DROP PROCEDURE IF EXISTS void_order;
DROP PROCEDURE IF EXISTS demo_savepoint_rescue;
DROP PROCEDURE IF EXISTS demo_deferrable_swap;
DROP PROCEDURE IF EXISTS demo_deferrable_restore;
DROP FUNCTION IF EXISTS get_product_rank;
DROP FUNCTION IF EXISTS get_product_revenue;
DROP FUNCTION IF EXISTS stale_paid_orders;
DROP TABLE IF EXISTS billing_invoices PURGE;
DROP TABLE IF EXISTS stock_diff PURGE;
DROP TABLE IF EXISTS audit_log PURGE;
DROP TABLE IF EXISTS reservations PURGE;
DROP TABLE IF EXISTS coupons PURGE;
DROP TABLE IF EXISTS reviews PURGE;
DROP TABLE IF EXISTS payments PURGE;
DROP TABLE IF EXISTS order_items PURGE;
DROP TABLE IF EXISTS orders_archive PURGE;
DROP TABLE IF EXISTS orders PURGE;
DROP TABLE IF EXISTS products PURGE;
DROP TABLE IF EXISTS categories PURGE;
DROP TABLE IF EXISTS merchants PURGE;
DROP TABLE IF EXISTS users PURGE;
DROP TABLE IF EXISTS product_condition PURGE;
DROP TABLE IF EXISTS order_status PURGE;
DROP SEQUENCE IF EXISTS coupon_code_seq;
DROP TYPE IF EXISTS money_amount FORCE;
DROP DOMAIN IF EXISTS cny_price;
PURGE RECYCLEBIN;

-- ── 1. 独立序列（sequences 面板；CYCLE 必须显式给 MAXVALUE） ────────
CREATE SEQUENCE coupon_code_seq START WITH 100000 MAXVALUE 999999999
    INCREMENT BY 1 CACHE 20 CYCLE;

-- ── 2. 对象类型 + 域（PG 的 composite type / domain 的 Oracle 对应物） ──
-- CREATE TYPE 在 SQL*Plus 里必须用 "/" 提交，否则后续语句会被一起吞进缓冲区；
-- SQLKit（JDBC）直接把这条 DDL 单独发下去即可，不需要 "/"。
CREATE TYPE money_amount AS OBJECT (cents NUMBER(20), currency CHAR(3));
/

CREATE DOMAIN cny_price AS NUMBER(12,2)
    CONSTRAINT ck_cny_price_range CHECK (VALUE >= 0 AND VALUE <= 99999999);

-- ── 3. 枚举字典表（PG 的 ENUM 类型 → 字典表 + 外键，Oracle 无 CREATE TYPE AS ENUM） ──
CREATE TABLE order_status (
    code       VARCHAR2(16) PRIMARY KEY,
    sort_order NUMBER(3)    NOT NULL,
    label      VARCHAR2(32) NOT NULL
);

INSERT ALL
    INTO order_status (code, sort_order, label) VALUES ('draft',     1, '草稿')
    INTO order_status (code, sort_order, label) VALUES ('paid',      2, '已支付')
    INTO order_status (code, sort_order, label) VALUES ('shipped',   3, '已发货')
    INTO order_status (code, sort_order, label) VALUES ('completed', 4, '已完成')
    INTO order_status (code, sort_order, label) VALUES ('cancelled', 5, '已取消')
    INTO order_status (code, sort_order, label) VALUES ('refunded',  6, '已退款')
SELECT * FROM dual;

CREATE TABLE product_condition (
    code       VARCHAR2(16) PRIMARY KEY,
    sort_order NUMBER(3)    NOT NULL,
    label      VARCHAR2(32) NOT NULL
);

INSERT ALL
    INTO product_condition (code, sort_order, label) VALUES ('new',         1, '全新')
    INTO product_condition (code, sort_order, label) VALUES ('refurbished', 2, '官翻')
    INTO product_condition (code, sort_order, label) VALUES ('used',        3, '二手')
SELECT * FROM dual;

-- ── 4. users（全类型覆盖: NUMBER/VARCHAR2/CHAR/CLOB/BLOB/RAW/BOOLEAN/JSON/域/对象类型） ──
-- 注意: Oracle 要求 DEFAULT 写在 NOT NULL 之前（PG 的 "NOT NULL DEFAULT x" 顺序会报 ORA-03076）。
CREATE TABLE users (
    id                 NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    username           VARCHAR2(64)  NOT NULL,
    email              VARCHAR2(320) NOT NULL,
    phone              VARCHAR2(20),
    password_hash      VARCHAR2(64)  NOT NULL,
    locale             CHAR(5)   DEFAULT 'zh_CN' NOT NULL,
    balance            NUMBER(12,2) DEFAULT 0 NOT NULL,
    registered_at      TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    last_login_at      TIMESTAMP WITH TIME ZONE,
    attrs              JSON      DEFAULT '{}' NOT NULL,
    tags               JSON      DEFAULT '[]' NOT NULL,
    signup_ip          VARCHAR2(45),
    geo_lat            NUMBER(9,6),
    geo_lng            NUMBER(9,6),
    avatar             BLOB,
    bio                CLOB,
    wallet             money_amount,
    device_fingerprint RAW(16),
    is_active          BOOLEAN   DEFAULT TRUE NOT NULL,
    created_at         TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    updated_at         TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    CONSTRAINT users_pk PRIMARY KEY (id),
    CONSTRAINT users_email_unique UNIQUE (email) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT ck_users_username_len CHECK (LENGTH(username) BETWEEN 2 AND 64),
    CONSTRAINT ck_users_email_fmt CHECK (REGEXP_LIKE(email, '^[^@]+@[^@]+\.[^@]+$')),
    CONSTRAINT ck_users_balance CHECK (balance >= 0 AND balance <= 99999999),
    -- 故意留一个 DISABLE 约束，供 Table Info / Constraints 面板展示状态
    CONSTRAINT ck_users_phone_len CHECK (phone IS NULL OR LENGTH(phone) = 11) DISABLE,
    -- 先建为 DISABLE，再用 ALTER ... ENABLE 打开（ENABLE/DISABLE 两种元数据都覆盖）
    CONSTRAINT ck_users_locale CHECK (locale IN ('zh_CN', 'en_US', 'ja_JP')) DISABLE
);

ALTER TABLE users ENABLE CONSTRAINT ck_users_locale;

COMMENT ON TABLE  users IS '平台注册用户（含 PII 测试字段）';
COMMENT ON COLUMN users.balance IS '钱包余额（人民币）';
COMMENT ON COLUMN users.email  IS '登录邮箱';

-- ── 5. merchants（UNIQUE ... USING INDEX 变体） ─────────────────────
CREATE TABLE merchants (
    id            NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    name          VARCHAR2(80) NOT NULL,
    commission    NUMBER(5,4) DEFAULT 0.0300 NOT NULL,
    contact_email VARCHAR2(120) NOT NULL,
    joined_at     TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    verified      BOOLEAN DEFAULT FALSE NOT NULL,
    CONSTRAINT merchants_pk PRIMARY KEY (id),
    CONSTRAINT ck_merchants_commission CHECK (commission BETWEEN 0 AND 0.5),
    CONSTRAINT uq_merchants_name UNIQUE (name)
        USING INDEX (CREATE UNIQUE INDEX uq_merchants_name_idx ON merchants (name))
);

-- ── 6. categories（自引用树 → CONNECT BY / 递归 CTE 压测） ─────────
CREATE TABLE categories (
    id        NUMBER(10) GENERATED BY DEFAULT AS IDENTITY,
    parent_id NUMBER(10),
    name      VARCHAR2(80) NOT NULL,
    is_leaf   BOOLEAN DEFAULT FALSE NOT NULL,
    CONSTRAINT categories_pk PRIMARY KEY (id),
    CONSTRAINT fk_categories_parent FOREIGN KEY (parent_id)
        REFERENCES categories (id) ON DELETE SET NULL
);

-- ── 7. products（复合主键 + 双外键 + 枚举字典外键 + 虚拟列） ────────
CREATE TABLE products (
    id          NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    merchant_id NUMBER(19) NOT NULL,
    category_id NUMBER(10),
    sku         VARCHAR2(40)  NOT NULL,
    name        VARCHAR2(200) NOT NULL,
    condition   VARCHAR2(16) DEFAULT 'new' NOT NULL,
    price       cny_price NOT NULL,
    stock       NUMBER(10) DEFAULT 0 NOT NULL,
    weight_g    NUMBER(8,1),
    dimensions  money_amount,
    specs       JSON DEFAULT '{}' NOT NULL,
    keywords    JSON DEFAULT '[]' NOT NULL,
    is_active   BOOLEAN DEFAULT TRUE NOT NULL,
    created_at  TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    updated_at  TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    price_level VARCHAR2(10) GENERATED ALWAYS AS
        (CASE WHEN price >= 3000 THEN 'high'
              WHEN price >= 1000 THEN 'mid'
              ELSE 'low' END) VIRTUAL,
    CONSTRAINT products_pk PRIMARY KEY (id, merchant_id),      -- 复合主键
    CONSTRAINT products_id_unique UNIQUE (id),                 -- 供单列 FK 引用
    CONSTRAINT fk_products_merchant FOREIGN KEY (merchant_id)
        REFERENCES merchants (id),                             -- Oracle 无 ON UPDATE CASCADE，默认即限制删除
    CONSTRAINT fk_products_category FOREIGN KEY (category_id)
        REFERENCES categories (id) ON DELETE SET NULL,
    CONSTRAINT fk_products_condition FOREIGN KEY (condition)
        REFERENCES product_condition (code),
    CONSTRAINT ck_products_price CHECK (price > 0),
    CONSTRAINT ck_products_stock CHECK (stock >= 0)
);

-- ── 8. orders（RANGE + INTERVAL 自动分区 + 复合主键 + 复合外键源头） ──
-- Oracle 的 INTERVAL 分区表禁止显式 MAXVALUE 分区（ORA-14761），
-- 兜底分区由 09 的 orders_archive 镜像；未来日期会按需自动建分区。
CREATE TABLE orders (
    id            NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    order_no      VARCHAR2(30) NOT NULL,
    user_id       NUMBER(19) NOT NULL,
    status        VARCHAR2(16) DEFAULT 'draft' NOT NULL,
    order_date    DATE NOT NULL,
    total_amount  NUMBER(12,2) DEFAULT 0 NOT NULL,
    discount      NUMBER(12,2) DEFAULT 0 NOT NULL,
    shipping_fee  NUMBER(10,2) DEFAULT 0 NOT NULL,
    shipping_addr JSON NOT NULL,
    remarks       VARCHAR2(400),
    placed_at     TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    shipped_at    TIMESTAMP WITH TIME ZONE,
    completed_at  TIMESTAMP WITH TIME ZONE,
    CONSTRAINT orders_pk PRIMARY KEY (id, order_date),          -- 分区键必须进主键
    CONSTRAINT orders_no_unique UNIQUE (order_no, order_date),
    CONSTRAINT fk_orders_user FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT fk_orders_status FOREIGN KEY (status) REFERENCES order_status (code),
    CONSTRAINT ck_orders_amounts CHECK (total_amount >= 0 AND discount >= 0 AND shipping_fee >= 0)
)
PARTITION BY RANGE (order_date) INTERVAL (NUMTOYMINTERVAL(1, 'MONTH'))
(
    PARTITION p2026_01 VALUES LESS THAN (DATE '2026-02-01'),
    PARTITION p2026_02 VALUES LESS THAN (DATE '2026-03-01'),
    PARTITION p2026_03 VALUES LESS THAN (DATE '2026-04-01')
)
ENABLE ROW MOVEMENT;

-- ── 9. orders_archive（RANGE 分区 + MAXVALUE 兜底分区，PG orders_default 的镜像） ──
CREATE TABLE orders_archive (
    id          NUMBER(19) NOT NULL,
    order_no    VARCHAR2(30) NOT NULL,
    order_date  DATE NOT NULL,
    archived_at TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    remarks     VARCHAR2(200),
    CONSTRAINT orders_archive_pk PRIMARY KEY (id, order_date)
)
PARTITION BY RANGE (order_date)
(
    PARTITION p2025 VALUES LESS THAN (DATE '2026-01-01'),
    PARTITION p_maxvalue VALUES LESS THAN (MAXVALUE)
)
ENABLE ROW MOVEMENT;

-- ── 10. order_items（复合外键 + 虚拟列生成列） ─────────────────────
CREATE TABLE order_items (
    id         NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    order_id   NUMBER(19) NOT NULL,
    order_date DATE NOT NULL,
    product_id NUMBER(19) NOT NULL,
    quantity   NUMBER(6) NOT NULL,
    unit_price NUMBER(12,2) NOT NULL,
    line_total NUMBER(14,2) GENERATED ALWAYS AS (quantity * unit_price) VIRTUAL,
    CONSTRAINT order_items_pk PRIMARY KEY (id, order_date),
    CONSTRAINT fk_items_order FOREIGN KEY (order_id, order_date)          -- 复合外键
        REFERENCES orders (id, order_date) ON DELETE CASCADE,
    CONSTRAINT fk_items_product FOREIGN KEY (product_id)
        REFERENCES products (id),
    CONSTRAINT ck_items_qty CHECK (quantity > 0),
    CONSTRAINT ck_items_price CHECK (unit_price > 0)
);

-- ── 11. payments（ON DELETE SET NULL / 系统命名 FK / 复合外键） ─────
CREATE TABLE payments (
    id              NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    order_id        NUMBER(19),
    order_date      DATE,
    external_no     VARCHAR2(40) NOT NULL,
    amount          NUMBER(12,2) NOT NULL,
    payment_channel VARCHAR2(20) DEFAULT 'alipay' NOT NULL,
    status          VARCHAR2(16) DEFAULT 'draft' NOT NULL,
    paid_at         TIMESTAMP WITH TIME ZONE,
    refunded_at     TIMESTAMP WITH TIME ZONE,
    payload         BLOB,
    CONSTRAINT payments_pk PRIMARY KEY (id),
    CONSTRAINT payments_external_uq UNIQUE (external_no),
    CONSTRAINT fk_payments_order FOREIGN KEY (order_id, order_date)
        REFERENCES orders (id, order_date) ON DELETE SET NULL,
    CONSTRAINT fk_payments_status FOREIGN KEY (status) REFERENCES order_status (code),
    CONSTRAINT ck_payments_amount CHECK (amount > 0)
);

-- ── 12. reservations（CHECK 区间约束 = PG EXCLUDE 时段互斥的降级替代） ──
-- Oracle 没有 EXCLUDE 约束；这里用 CHECK(valid_to > valid_from) 保证区间合法，
-- 再用 (resource_id, valid_from) 唯一约束防重复占位（无法表达真正的区间重叠互斥）。
-- PG 的 tstzrange 拆成两个 TIMESTAMP 列：Oracle 不允许 TSTZ 列进唯一键（ORA-02329），
-- TSTZ 类型覆盖已由 users/orders/payments 等列承担。
CREATE TABLE reservations (
    id          NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    resource_id NUMBER(19) NOT NULL,
    valid_from  TIMESTAMP NOT NULL,
    valid_to    TIMESTAMP NOT NULL,
    note        money_amount,
    CONSTRAINT reservations_pk PRIMARY KEY (id),
    CONSTRAINT ck_reservations_range CHECK (valid_to > valid_from),
    CONSTRAINT uq_reservations_slot UNIQUE (resource_id, valid_from)
);

-- ── 13. reviews / coupons（含 0 个/1 个约束、部分唯一索引替代） ─────
CREATE TABLE reviews (
    id         NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    payment_id NUMBER(19),
    user_id    NUMBER(19) NOT NULL,
    product_id NUMBER(19) NOT NULL,
    rating     NUMBER(1) NOT NULL,
    content    VARCHAR2(1000) NOT NULL,
    is_deleted BOOLEAN DEFAULT FALSE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    CONSTRAINT reviews_pk PRIMARY KEY (id),
    FOREIGN KEY (payment_id) REFERENCES payments (id) ON DELETE SET NULL,  -- 系统命名 FK
    CONSTRAINT fk_reviews_user FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT fk_reviews_product FOREIGN KEY (product_id) REFERENCES products (id),
    CONSTRAINT ck_reviews_rating CHECK (rating BETWEEN 1 AND 5)
);

CREATE TABLE coupons (
    id          NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    code        VARCHAR2(30) NOT NULL,
    discount    NUMBER(12,2) NOT NULL,
    min_amount  NUMBER(12,2) DEFAULT 0 NOT NULL,
    valid_until TIMESTAMP WITH TIME ZONE NOT NULL,
    claimed_by  NUMBER(19),
    claimed_at  TIMESTAMP WITH TIME ZONE,
    CONSTRAINT coupons_pk PRIMARY KEY (id),
    CONSTRAINT coupons_code_unique UNIQUE (code),
    CONSTRAINT fk_coupons_user FOREIGN KEY (claimed_by)
        REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT ck_coupons_discount CHECK (discount > 0)
);

-- ── 14. audit_log（触发器落点）/ stock_diff（MERGE 对账暂存）/ billing 域 ──
CREATE TABLE audit_log (
    id         NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    table_name VARCHAR2(30) NOT NULL,
    op_type    VARCHAR2(16) NOT NULL,
    row_pk     NUMBER(19),
    snapshot   JSON NOT NULL,
    at         TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    CONSTRAINT audit_log_pk PRIMARY KEY (id)
);

CREATE TABLE stock_diff (
    sku   VARCHAR2(40) NOT NULL,
    delta NUMBER(10)   NOT NULL,
    CONSTRAINT ck_stock_diff_delta CHECK (delta <> 0)
);

CREATE TABLE billing_invoices (
    id         NUMBER(19) GENERATED ALWAYS AS IDENTITY,
    invoice_no VARCHAR2(40) NOT NULL,
    order_id   NUMBER(19),
    order_date DATE,
    billed_at  TIMESTAMP WITH TIME ZONE DEFAULT SYSTIMESTAMP NOT NULL,
    amount     NUMBER(12,2) DEFAULT 0 NOT NULL,
    status     VARCHAR2(16) DEFAULT 'issued' NOT NULL,
    CONSTRAINT billing_invoices_pk PRIMARY KEY (id),
    CONSTRAINT billing_invoices_no_uq UNIQUE (invoice_no),
    CONSTRAINT fk_billing_invoice_order FOREIGN KEY (order_id, order_date)
        REFERENCES orders (id, order_date),
    CONSTRAINT ck_billing_amount CHECK (amount >= 0)
);

-- ── 15. 索引（B-tree / 唯一 / 函数 / 复合 / 位图 / 降序 / 压缩 全谱系） ──
-- 注意: 每条语句后面不要再跟 "-- 注释"（Oracle 23ai 的 SQL*Plus 会认为语句未结束，
-- 报 ORA-03405），说明性注释统一写在语句上方。
-- 函数索引
CREATE INDEX idx_users_lower_email ON users (LOWER(email));
-- PG BRIN 的替代（Oracle 无 BRIN）
CREATE INDEX idx_users_registered ON users (registered_at);
-- 普通 B-tree
CREATE INDEX idx_products_category ON products (category_id);
-- 函数索引（UPPER）
CREATE INDEX idx_products_name_upper ON products (UPPER(name));
-- 唯一 + 复合 + 函数
CREATE UNIQUE INDEX uq_products_sku ON products (LOWER(sku), merchant_id);
-- 位图索引（低基数列）
CREATE BITMAP INDEX bidx_products_condition ON products (condition);
-- 复合 + 分区局部索引
CREATE INDEX idx_orders_user_status ON orders (user_id, status) LOCAL;
-- PG BRIN 的替代
CREATE INDEX idx_orders_placed ON orders (placed_at);
CREATE INDEX idx_order_items_product ON order_items (product_id);
-- 复合 + 压缩索引（COMPRESS）
CREATE INDEX idx_order_items_order ON order_items (order_id, order_date) COMPRESS 1;
CREATE INDEX idx_payments_order ON payments (order_id, order_date);
-- 降序索引
CREATE INDEX idx_reviews_rating ON reviews (rating DESC);
CREATE INDEX idx_reviews_product ON reviews (product_id);
-- PG 的部分索引 / 部分 EXCLUDE 在 Oracle 无对应语法，用 CASE 函数索引表达"仅已领取的行唯一"
CREATE UNIQUE INDEX uq_coupons_claim_once
    ON coupons (CASE WHEN claimed_by IS NOT NULL THEN id END);
CREATE INDEX idx_audit_at ON audit_log (at);

-- ── 16. 视图（普通 / 连接 / 聚合 / WITH CHECK OPTION 可更新视图） ──
CREATE OR REPLACE VIEW active_products AS
SELECT id, merchant_id, category_id, sku, name, price, stock, keywords
FROM products WHERE is_active AND stock > 0;

-- PG 的 security_barrier 在 Oracle 无对应写法，用 READ ONLY 视图近似
CREATE OR REPLACE VIEW product_catalog AS
SELECT p.id, p.name, p.price, m.name AS merchant_name
FROM products p JOIN merchants m ON m.id = p.merchant_id
WITH READ ONLY;

CREATE OR REPLACE VIEW shop_with_check AS
SELECT * FROM active_products WHERE price < 10000
WITH CHECK OPTION;

CREATE OR REPLACE VIEW merchant_sales_summary AS
SELECT m.id AS merchant_id,
       m.name AS merchant_name,
       COUNT(DISTINCT o.id) AS order_cnt,
       SUM(oi.quantity) AS units_sold,
       SUM(oi.line_total) AS revenue
FROM merchants m
JOIN products p ON p.merchant_id = m.id
JOIN order_items oi ON oi.product_id = p.id
JOIN orders o ON o.id = oi.order_id AND o.order_date = oi.order_date
GROUP BY m.id, m.name;

-- ── 17. 物化视图（REFRESH COMPLETE ON DEMAND + 唯一索引 + 嵌套视图） ──
CREATE MATERIALIZED VIEW analytics_daily_sales
    REFRESH COMPLETE ON DEMAND
AS
SELECT o.order_date AS sales_day,
       COUNT(DISTINCT o.id) AS order_cnt,
       SUM(oi.quantity) AS units_sold,
       SUM(oi.line_total) AS revenue
FROM orders o
JOIN order_items oi ON oi.order_id = o.id AND oi.order_date = o.order_date
GROUP BY o.order_date;

CREATE UNIQUE INDEX uq_analytics_daily_sales_day ON analytics_daily_sales (sales_day);
CREATE INDEX idx_analytics_daily_sales_rev ON analytics_daily_sales (revenue DESC);

CREATE OR REPLACE VIEW analytics_sales_dashboard AS
SELECT sales_day, order_cnt, revenue FROM analytics_daily_sales
UNION ALL
SELECT TRUNC(SYSDATE), 0, 0 FROM dual;

-- ── 18. 函数 / 过程 / 包（Functions & Procedures 面板：含同名重载） ──
-- 以下每个 PL/SQL 对象都是独立一条 CREATE OR REPLACE 语句，以 "/" 提交。
-- SQLKit 里请逐个对象单独执行（见 README）；SQL*Plus 可直接整文件跑。
CREATE OR REPLACE FUNCTION get_product_rank(p_pid NUMBER) RETURN NUMBER AS
    v_qty NUMBER;
BEGIN
    SELECT NVL(SUM(oi.quantity), 0) INTO v_qty
    FROM order_items oi WHERE oi.product_id = p_pid;
    RETURN v_qty;
END;
/

-- 同名重载：参数类型 NUMBER / VARCHAR2 不同 → Functions 面板显示两个签名
CREATE OR REPLACE FUNCTION get_product_rank(p_sku VARCHAR2) RETURN NUMBER AS
    v_qty NUMBER;
BEGIN
    SELECT NVL(SUM(oi.quantity), 0) INTO v_qty
    FROM order_items oi JOIN products p ON p.id = oi.product_id
    WHERE LOWER(p.sku) = LOWER(p_sku);
    RETURN v_qty;
END;
/

CREATE OR REPLACE FUNCTION get_product_revenue(p_pid NUMBER) RETURN NUMBER AS
    v_rev NUMBER;
BEGIN
    SELECT NVL(SUM(oi.line_total), 0) INTO v_rev
    FROM order_items oi WHERE oi.product_id = p_pid;
    RETURN v_rev;
END;
/

-- 动态 SQL（对应 PG 的 DO 匿名块 + 动态 SQL 演练）
CREATE OR REPLACE FUNCTION stale_paid_orders(p_days NUMBER) RETURN NUMBER AS
    v_cnt NUMBER;
BEGIN
    EXECUTE IMMEDIATE
        'SELECT COUNT(*) FROM orders WHERE status = ''paid'''
        || ' AND placed_at < SYSTIMESTAMP - NUMTODSINTERVAL(:1, ''DAY'')'
        INTO v_cnt USING p_days;
    RETURN v_cnt;
END;
/

CREATE OR REPLACE PROCEDURE refresh_daily_sales AS
BEGIN
    DBMS_MVIEW.REFRESH('ANALYTICS_DAILY_SALES', 'C');
END;
/

CREATE OR REPLACE PROCEDURE void_order(p_order_no VARCHAR2) AS
BEGIN
    UPDATE orders SET status = 'cancelled'
     WHERE order_no = p_order_no
       AND status IN ('draft', 'paid');
END;
/

CREATE OR REPLACE PACKAGE coupon_pkg AS
    PROCEDURE claim_coupon(p_code VARCHAR2, p_user NUMBER);
    FUNCTION claimed_count RETURN NUMBER;
END coupon_pkg;
/

CREATE OR REPLACE PACKAGE BODY coupon_pkg AS
    PROCEDURE claim_coupon(p_code VARCHAR2, p_user NUMBER) AS
    BEGIN
        UPDATE coupons
           SET claimed_by = p_user, claimed_at = SYSTIMESTAMP
         WHERE code = p_code
           AND claimed_by IS NULL
           AND valid_until > SYSTIMESTAMP;
    END claim_coupon;

    FUNCTION claimed_count RETURN NUMBER AS
        v_cnt NUMBER;
    BEGIN
        SELECT COUNT(*) INTO v_cnt FROM coupons WHERE claimed_by IS NOT NULL;
        RETURN v_cnt;
    END claimed_count;
END coupon_pkg;
/

CREATE OR REPLACE PACKAGE analytics_pkg AS
    FUNCTION product_units(p_pid NUMBER) RETURN NUMBER;
    FUNCTION product_revenue(p_pid NUMBER) RETURN NUMBER;
END analytics_pkg;
/

CREATE OR REPLACE PACKAGE BODY analytics_pkg AS
    FUNCTION product_units(p_pid NUMBER) RETURN NUMBER AS
        v_qty NUMBER;
    BEGIN
        SELECT NVL(SUM(quantity), 0) INTO v_qty
        FROM order_items WHERE product_id = p_pid;
        RETURN v_qty;
    END product_units;

    FUNCTION product_revenue(p_pid NUMBER) RETURN NUMBER AS
        v_rev NUMBER;
    BEGIN
        SELECT NVL(SUM(line_total), 0) INTO v_rev
        FROM order_items WHERE product_id = p_pid;
        RETURN v_rev;
    END product_revenue;
END analytics_pkg;
/

-- ── 19. 触发器（BEFORE/AFTER 行级 + 语句级 + 复合触发器，全部落 audit_log） ──
CREATE OR REPLACE TRIGGER trg_orders_audit
AFTER INSERT OR UPDATE OR DELETE ON orders
FOR EACH ROW
BEGIN
    IF INSERTING THEN
        INSERT INTO audit_log (table_name, op_type, row_pk, snapshot)
        VALUES ('ORDERS', 'INSERT', :NEW.id,
                JSON_OBJECT('id' VALUE :NEW.id, 'order_no' VALUE :NEW.order_no,
                            'status' VALUE :NEW.status, 'total_amount' VALUE :NEW.total_amount));
    ELSIF UPDATING THEN
        INSERT INTO audit_log (table_name, op_type, row_pk, snapshot)
        VALUES ('ORDERS', 'UPDATE', :NEW.id,
                JSON_OBJECT('id' VALUE :NEW.id, 'order_no' VALUE :NEW.order_no,
                            'status' VALUE :NEW.status, 'total_amount' VALUE :NEW.total_amount));
    ELSE
        INSERT INTO audit_log (table_name, op_type, row_pk, snapshot)
        VALUES ('ORDERS', 'DELETE', :OLD.id,
                JSON_OBJECT('id' VALUE :OLD.id, 'order_no' VALUE :OLD.order_no,
                            'status' VALUE :OLD.status, 'total_amount' VALUE :OLD.total_amount));
    END IF;
END;
/

CREATE OR REPLACE TRIGGER trg_products_before_set
BEFORE UPDATE ON products
FOR EACH ROW
BEGIN
    :NEW.updated_at := SYSTIMESTAMP;
    INSERT INTO audit_log (table_name, op_type, row_pk, snapshot)
    VALUES ('PRODUCTS', 'BEFORE_UPDATE', :NEW.id,
            JSON_OBJECT('id' VALUE :NEW.id, 'sku' VALUE :NEW.sku,
                        'stock' VALUE :NEW.stock, 'old_stock' VALUE :OLD.stock));
END;
/

CREATE OR REPLACE TRIGGER trg_reviews_statement
AFTER INSERT ON reviews
BEGIN
    INSERT INTO audit_log (table_name, op_type, row_pk, snapshot)
    VALUES ('REVIEWS', 'INSERT_STMT', NULL,
            JSON_OBJECT('statement_op' VALUE 'INSERT'));
END;
/

-- Oracle 专有复合触发器：按语句汇总 order_items 的写入行数
CREATE OR REPLACE TRIGGER trg_items_compound
FOR INSERT ON order_items COMPOUND TRIGGER
    v_cnt NUMBER := 0;
    BEFORE STATEMENT IS
    BEGIN
        v_cnt := 0;
    END BEFORE STATEMENT;
    AFTER EACH ROW IS
    BEGIN
        v_cnt := v_cnt + 1;
    END AFTER EACH ROW;
    AFTER STATEMENT IS
    BEGIN
        INSERT INTO audit_log (table_name, op_type, row_pk, snapshot)
        VALUES ('ORDER_ITEMS', 'COMPOUND_STMT', v_cnt,
                JSON_OBJECT('statement_op' VALUE 'INSERT', 'row_count' VALUE v_cnt));
    END AFTER STATEMENT;
END trg_items_compound;
/

-- ── 20. 事务语义演示过程（供 03 的 I / Q 段在 SQLKit 里一次调用） ──
-- SQLKit 的 Oracle 通道经 JDBC 桥逐条自动提交（每条语句一个事务），因此
-- 「SAVEPOINT 救援」和「可延迟约束互换」这类多语句事务无法逐条下发。
-- 把事务封进存储过程后，SQLKit 只需 CALL 一次即可跑出正确语义，
-- 且过程内的预期错误被异常处理器吞掉，界面上不会出现噪声报错。
CREATE OR REPLACE PROCEDURE demo_savepoint_rescue AS
BEGIN
    DELETE FROM users WHERE username = 'tx_probe';
    INSERT INTO users (username, email, password_hash)
    VALUES ('tx_probe', 'txprobe@example.com',
            LOWER(RAWTOHEX(STANDARD_HASH('x', 'MD5'))));

    SAVEPOINT before_bad_insert;
    BEGIN
        -- 故意违反 ck_reservations_range（ORA-02290），由内层处理器吞掉
        INSERT INTO reservations (resource_id, valid_from, valid_to, note)
        VALUES (1, TIMESTAMP '2026-06-05 00:00:00', TIMESTAMP '2026-06-03 00:00:00',
                money_amount(10, 'CNY'));
    EXCEPTION
        WHEN OTHERS THEN NULL;
    END;

    ROLLBACK TO SAVEPOINT before_bad_insert;   -- 只回退这一段，事务继续
    UPDATE users SET balance = balance + 100 WHERE username = 'tx_probe';
    COMMIT;
END demo_savepoint_rescue;
/

-- 可延迟唯一约束：两条 UPDATE 在同一事务内瞬时冲突，提交时才校验并通过
CREATE OR REPLACE PROCEDURE demo_deferrable_swap AS
BEGIN
    UPDATE users SET email = 'user2@mail.example.com' WHERE username = 'user_1';
    UPDATE users SET email = 'user1@mail.example.com' WHERE username = 'user_2';
    COMMIT;
EXCEPTION
    WHEN OTHERS THEN ROLLBACK;
END demo_deferrable_swap;
/

CREATE OR REPLACE PROCEDURE demo_deferrable_restore AS
BEGIN
    UPDATE users SET email = 'user1@mail.example.com' WHERE username = 'user_1';
    UPDATE users SET email = 'user2@mail.example.com' WHERE username = 'user_2';
    COMMIT;
EXCEPTION
    WHEN OTHERS THEN ROLLBACK;
END demo_deferrable_restore;
/

-- 完成。数据灌入请依次执行 02_seed.sql → 03_complex_queries.sql
