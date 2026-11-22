-- =====================================================================
-- SQLKit MySQL 压测 1/3 —— Schema / 对象全景
-- 目标库: sqlkit-demo（docker ssh-tunnel 栈: mysql-test:3306）
-- 执行顺序: 01_schema.sql → 02_seed.sql → 03_complex_queries.sql
-- 覆盖面板: Schemas / Tables / Columns / Table Info / DDL / Drop / Rename
--           Indexes / Foreign Keys / Triggers / Views / Procedures / Functions
-- 注意: 全部幂等（先 DROP DATABASE 再 CREATE），可反复执行
-- 连接: root/testpass（建库需要），库名 sqlkit-demo
-- =====================================================================

-- ── 0. 清场（整个库删除，视图/触发器/例程一并清除） ────────────────
DROP DATABASE IF EXISTS `sqlkit-demo`;

-- ── 1. 建库（MySQL 无 schema 层: database 即 schema，见 Schemas 面板） ──
CREATE DATABASE `sqlkit-demo`
    CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE DATABASE IF NOT EXISTS `sqlkit-analytics`
    CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE DATABASE IF NOT EXISTS `sqlkit-billing`
    CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
-- 跨库只读视图需要 SELECT 授权（例程/视图的安全上下文）
GRANT SELECT ON `sqlkit-demo`.*   TO 'tester'@'%';
GRANT SELECT ON `sqlkit-analytics`.* TO 'tester'@'%';
GRANT SELECT ON `sqlkit-billing`.* TO 'tester'@'%';
FLUSH PRIVILEGES;

USE `sqlkit-demo`;

-- ── 2. 枚举字典表（MySQL 无 CREATE TYPE: 用字典表模拟状态机） ──────
CREATE TABLE order_status (
    code  VARCHAR(16) NOT NULL PRIMARY KEY,
    sort  TINYINT UNSIGNED NOT NULL,
    label VARCHAR(32) NOT NULL
) ENGINE = InnoDB COMMENT = '订单生命周期状态机';

INSERT INTO order_status (code, sort, label) VALUES
  ('draft', 1, '草稿'), ('paid', 2, '已支付'), ('shipped', 3, '已发货'),
  ('completed', 4, '已完成'), ('cancelled', 5, '已取消'), ('refunded', 6, '已退款');

-- ── 3. 用户表（长列 + 类型全覆盖 + JSON + 空间类型 + 生成列） ──────
CREATE TABLE users (
    id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    username      VARCHAR(64)  NOT NULL,
    email         VARCHAR(191) NOT NULL,
    phone         VARCHAR(32)  DEFAULT NULL,
    password_hash CHAR(32)     NOT NULL,
    locale        CHAR(5)      NOT NULL DEFAULT 'zh_CN',
    balance       DECIMAL(12,2) NOT NULL DEFAULT 0.00,
    -- 生成列: 邮箱小写（唯一索引落在生成列上）
    email_lower   VARCHAR(191) GENERATED ALWAYS AS (LOWER(email)) STORED,
    -- 虚拟生成列: 用户名长度（不占存储）
    name_len      SMALLINT     GENERATED ALWAYS AS (CHAR_LENGTH(username)) VIRTUAL,
    registered_at DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    last_login_at DATETIME(3)  DEFAULT NULL,
    attrs         JSON         NOT NULL,
    tags          JSON         NOT NULL,
    signup_ip     VARBINARY(16) DEFAULT NULL,
    -- 空间列必须 NOT NULL 才能建 SPATIAL 索引（MySQL 限制）
    geo           POINT        NOT NULL SRID 4326,
    avatar        MEDIUMBLOB   DEFAULT NULL,
    bio           TEXT,
    is_active     TINYINT   NOT NULL DEFAULT 1,
    created_at    DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at    DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
                               ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    CONSTRAINT users_email_unique UNIQUE (email),
    CONSTRAINT uq_users_lower_email UNIQUE (email_lower),
    CONSTRAINT chk_users_username CHECK (CHAR_LENGTH(username) BETWEEN 2 AND 64),
    CONSTRAINT chk_users_balance CHECK (balance >= 0 AND balance <= 99999999)
) ENGINE = InnoDB COMMENT = '平台注册用户（含 PII 测试字段）';

CREATE INDEX idx_users_registered ON users (registered_at DESC);
CREATE INDEX idx_users_city ON users ((CAST(attrs ->> '$.city' AS CHAR(16))));
-- 多值索引: JSON 数组元素检索（MySQL 8.0.17+）
-- 注意: 多值索引只接受数值/日期元素，因此 tags 用数字数组
CREATE INDEX idx_users_tags_mv ON users ((CAST(tags -> '$' AS UNSIGNED ARRAY)));
-- 空间索引: 地理点
CREATE SPATIAL INDEX idx_users_geo ON users (geo);

-- ── 4. 商户表（唯一键 + CHECK + 边界值） ──────────────────────────
CREATE TABLE merchants (
    id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    name          VARCHAR(64) NOT NULL,
    commission    DECIMAL(5,4) NOT NULL DEFAULT 0.0300,
    contact_email VARCHAR(191) NOT NULL,
    joined_at     DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    verified      TINYINT NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    CONSTRAINT uq_merchants_name UNIQUE (name),
    CONSTRAINT chk_merchants_commission CHECK (commission BETWEEN 0 AND 0.5)
) ENGINE = InnoDB;

-- ── 5. 分类表（自引用树 → 递归 CTE 压测） ─────────────────────────
CREATE TABLE categories (
    id        INT NOT NULL AUTO_INCREMENT,
    parent_id INT DEFAULT NULL,
    name      VARCHAR(64) NOT NULL,
    is_leaf   TINYINT NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    CONSTRAINT fk_categories_parent FOREIGN KEY (parent_id)
        REFERENCES categories (id) ON DELETE SET NULL ON UPDATE CASCADE
) ENGINE = InnoDB;

-- ── 6. 商品表（双外键 + 全文索引 + 前缀索引 + 生成列 + JSON） ─────
CREATE TABLE products (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    merchant_id  BIGINT UNSIGNED NOT NULL,
    category_id  INT DEFAULT NULL,
    sku          VARCHAR(32) NOT NULL,
    name         VARCHAR(191) NOT NULL,
    item_condition ENUM('new','refurbished','used') NOT NULL DEFAULT 'new',
    price        DECIMAL(12,2) NOT NULL,
    stock        INT NOT NULL DEFAULT 0,
    weight_g     DECIMAL(8,1) DEFAULT NULL,
    specs        JSON NOT NULL,
    keywords     JSON NOT NULL,
    -- 生成列: 含税价（索引与查询都能用）
    price_with_tax DECIMAL(12,2) GENERATED ALWAYS AS (ROUND(price * 1.13, 2)) STORED,
    is_active    TINYINT NOT NULL DEFAULT 1,
    created_at   DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at   DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
                            ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    CONSTRAINT uq_products_sku UNIQUE (merchant_id, sku),
    CONSTRAINT chk_products_price CHECK (price > 0),
    CONSTRAINT chk_products_stock CHECK (stock >= 0),
    CONSTRAINT fk_products_merchant FOREIGN KEY (merchant_id)
        REFERENCES merchants (id) ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT fk_products_category FOREIGN KEY (category_id)
        REFERENCES categories (id) ON DELETE SET NULL
) ENGINE = InnoDB;

CREATE INDEX idx_products_category ON products (category_id);
CREATE INDEX idx_products_price_active ON products (price, is_active);
-- 前缀索引
CREATE INDEX idx_products_name_prefix ON products (name(24));
-- 全文索引（ngram 解析器 → 中文分词）
CREATE FULLTEXT INDEX ft_products_name ON products (name) WITH PARSER ngram;

-- ── 7. 订单表（复合唯一键 + JSON 地址 + 外键） ────────────────────
-- 注意: MySQL 不允许分区表带外键，因此这里保持普通表以保留完整 FK；
--       分区能力由第 8 节的 order_events（无 FK）单独演示。
CREATE TABLE orders (
    id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    order_no      VARCHAR(32) NOT NULL,
    user_id       BIGINT UNSIGNED NOT NULL,
    status        VARCHAR(16) NOT NULL DEFAULT 'draft',
    order_date    DATE NOT NULL,
    total_amount  DECIMAL(12,2) NOT NULL DEFAULT 0.00,
    discount      DECIMAL(12,2) NOT NULL DEFAULT 0.00,
    shipping_fee  DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    shipping_addr JSON NOT NULL,
    remarks       VARCHAR(255) DEFAULT NULL,
    placed_at     DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    shipped_at    DATETIME(3) DEFAULT NULL,
    completed_at  DATETIME(3) DEFAULT NULL,
    PRIMARY KEY (id),
    CONSTRAINT uq_orders_no UNIQUE (order_no),
    CONSTRAINT fk_orders_user FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT chk_orders_status CHECK (status IN
        ('draft','paid','shipped','completed','cancelled','refunded'))
) ENGINE = InnoDB;

CREATE INDEX idx_orders_user_status ON orders (user_id, status);
CREATE INDEX idx_orders_placed ON orders (placed_at);
CREATE INDEX idx_orders_city ON orders ((CAST(shipping_addr ->> '$.city' AS CHAR(16))));

-- ── 8. 订单明细（外键 CASCADE + 生成列） ──────────────────────────
CREATE TABLE order_items (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    order_id   BIGINT UNSIGNED NOT NULL,
    product_id BIGINT UNSIGNED NOT NULL,
    quantity   INT NOT NULL,
    unit_price DECIMAL(12,2) NOT NULL,
    line_total DECIMAL(14,2) GENERATED ALWAYS AS (quantity * unit_price) STORED,
    PRIMARY KEY (id),
    CONSTRAINT chk_items_qty CHECK (quantity > 0),
    CONSTRAINT chk_items_price CHECK (unit_price > 0),
    CONSTRAINT fk_items_order FOREIGN KEY (order_id)
        REFERENCES orders (id) ON DELETE CASCADE,
    CONSTRAINT fk_items_product FOREIGN KEY (product_id) REFERENCES products (id)
) ENGINE = InnoDB;

CREATE INDEX idx_order_items_product ON order_items (product_id);

-- ── 8b. 订单事件表（RANGE 分区演示: 无外键才能分区） ───────────────
-- 分区键必须是每个唯一键的成员 → 主键 (id, event_year)
CREATE TABLE order_events (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    order_id   BIGINT UNSIGNED NOT NULL,
    event_year SMALLINT NOT NULL,
    event_type VARCHAR(32) NOT NULL,
    event_at   DATETIME(3) NOT NULL,
    detail     JSON NOT NULL,
    PRIMARY KEY (id, event_year)
) ENGINE = InnoDB
PARTITION BY RANGE (event_year) (
    PARTITION p2025 VALUES LESS THAN (2026),
    PARTITION p2026 VALUES LESS THAN (2027),
    PARTITION p2027 VALUES LESS THAN (2028),
    PARTITION pmax  VALUES LESS THAN MAXVALUE
);

CREATE INDEX idx_order_events_order ON order_events (order_id, event_at);

-- ── 9. 支付表（未显式命名的外键 → 默认约束名分组） ────────────────
CREATE TABLE payments (
    id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    order_id        BIGINT UNSIGNED DEFAULT NULL,
    external_no     VARCHAR(64) NOT NULL,
    amount          DECIMAL(12,2) NOT NULL,
    payment_channel ENUM('alipay','wechat','unionpay') NOT NULL,
    status          VARCHAR(16) NOT NULL DEFAULT 'draft',
    paid_at         DATETIME(3) DEFAULT NULL,
    refunded_at     DATETIME(3) DEFAULT NULL,
    payload         MEDIUMBLOB,
    PRIMARY KEY (id),
    CONSTRAINT uq_payments_external UNIQUE (external_no),
    CONSTRAINT chk_payments_amount CHECK (amount > 0)
) ENGINE = InnoDB;

ALTER TABLE payments
    ADD CONSTRAINT fk_payments_order FOREIGN KEY (order_id)
        REFERENCES orders (id) ON DELETE SET NULL;

-- ── 10. 评价表（自引用反范式 + 复合索引 + 布尔软删） ──────────────
CREATE TABLE reviews (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    payment_id BIGINT UNSIGNED DEFAULT NULL,
    user_id    BIGINT UNSIGNED NOT NULL,
    product_id BIGINT UNSIGNED NOT NULL,
    rating     TINYINT UNSIGNED NOT NULL,
    content    VARCHAR(500) NOT NULL,
    is_deleted TINYINT NOT NULL DEFAULT 0,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    CONSTRAINT chk_reviews_rating CHECK (rating BETWEEN 1 AND 5),
    CONSTRAINT fk_reviews_payment FOREIGN KEY (payment_id)
        REFERENCES payments (id) ON DELETE SET NULL,
    CONSTRAINT fk_reviews_user FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT fk_reviews_product FOREIGN KEY (product_id) REFERENCES products (id)
) ENGINE = InnoDB;

CREATE INDEX idx_reviews_rating ON reviews (rating DESC, created_at DESC);
CREATE UNIQUE INDEX uq_reviews_user_product ON reviews (user_id, product_id);

-- ── 11. 优惠券（外键 CASCADE + 唯一码） ───────────────────────────
CREATE TABLE coupons (
    id          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    code        VARCHAR(32) NOT NULL,
    discount    DECIMAL(12,2) NOT NULL,
    min_amount  DECIMAL(12,2) NOT NULL DEFAULT 0.00,
    valid_until DATETIME(3) NOT NULL,
    claimed_by  BIGINT UNSIGNED DEFAULT NULL,
    claimed_at  DATETIME(3) DEFAULT NULL,
    PRIMARY KEY (id),
    CONSTRAINT uq_coupons_code UNIQUE (code),
    CONSTRAINT chk_coupons_discount CHECK (discount > 0),
    CONSTRAINT fk_coupons_user FOREIGN KEY (claimed_by)
        REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB;

-- ── 12. 审计表（触发器落点，跨库写入） ────────────────────────────
CREATE TABLE audit_log (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    db_name    VARCHAR(64) NOT NULL,
    table_name VARCHAR(64) NOT NULL,
    op_type    VARCHAR(16) NOT NULL,
    row_pk     BIGINT UNSIGNED DEFAULT NULL,
    snapshot   JSON NOT NULL,
    at         DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id)
) ENGINE = InnoDB;

CREATE INDEX idx_audit_at ON audit_log (at);

-- ── 13. 库存预留（无 EXCLUDE 约束，用唯一键 + 时间区间列近似） ────
CREATE TABLE reservations (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    resource_id  BIGINT UNSIGNED NOT NULL,
    valid_from   DATETIME NOT NULL,
    valid_to     DATETIME NOT NULL,
    note_cents   BIGINT DEFAULT NULL,
    note_currency CHAR(3) DEFAULT NULL,
    PRIMARY KEY (id),
    CONSTRAINT chk_reservations_range CHECK (valid_to > valid_from)
) ENGINE = InnoDB;

CREATE INDEX idx_reservations_resource ON reservations (resource_id, valid_from, valid_to);

-- ── 14. 函数（确定性 + 非确定性 + 跨库同名函数） ──────────────────
DROP FUNCTION IF EXISTS get_product_rank;
DROP FUNCTION IF EXISTS cny_format;

DELIMITER $$

CREATE FUNCTION get_product_rank(pid BIGINT)
RETURNS DECIMAL(12,2)
DETERMINISTIC READS SQL DATA
BEGIN
    DECLARE rev DECIMAL(12,2);
    SELECT COALESCE(SUM(line_total), 0) INTO rev
    FROM order_items WHERE product_id = pid;
    RETURN rev;
END$$

CREATE FUNCTION cny_format(cents BIGINT)
RETURNS VARCHAR(32)
DETERMINISTIC NO SQL
BEGIN
    RETURN CONCAT('¥', FORMAT(cents / 100, 2));
END$$

DELIMITER ;

-- ── 15. 存储过程（含 IN/OUT 参数 + 事务） ─────────────────────────
DROP PROCEDURE IF EXISTS claim_coupon;
DROP PROCEDURE IF EXISTS void_order;
DROP PROCEDURE IF EXISTS merchant_report;

DELIMITER $$

CREATE PROCEDURE claim_coupon(IN p_code VARCHAR(32), IN p_user BIGINT, OUT p_ok TINYINT)
BEGIN
    DECLARE affected INT DEFAULT 0;
    UPDATE coupons
       SET claimed_by = p_user, claimed_at = NOW(3)
     WHERE code = p_code AND claimed_by IS NULL AND valid_until > NOW();
    SET affected = ROW_COUNT();
    SET p_ok = IF(affected > 0, 1, 0);
END$$

CREATE PROCEDURE void_order(IN p_order_no VARCHAR(32))
BEGIN
    UPDATE orders
       SET status = 'cancelled'
     WHERE order_no = p_order_no AND status IN ('draft', 'paid');
END$$

CREATE PROCEDURE merchant_report(IN p_min_sold INT)
BEGIN
    SELECT m.name AS merchant, COUNT(DISTINCT p.id) AS product_cnt,
           COALESCE(SUM(oi.quantity), 0) AS sold
    FROM merchants m
    JOIN products p ON p.merchant_id = m.id
    LEFT JOIN order_items oi ON oi.product_id = p.id
    GROUP BY m.id, m.name
    HAVING sold >= p_min_sold
    ORDER BY sold DESC;
END$$

DELIMITER ;

-- ── 16. 触发器（BEFORE/AFTER × INSERT/UPDATE/DELETE + 跨库写审计） ──
DROP TRIGGER IF EXISTS trg_orders_ai;
DROP TRIGGER IF EXISTS trg_orders_au;
DROP TRIGGER IF EXISTS trg_orders_ad;
DROP TRIGGER IF EXISTS trg_products_bu;
DROP TRIGGER IF EXISTS trg_users_bu;

DELIMITER $$

CREATE TRIGGER trg_orders_ai AFTER INSERT ON orders FOR EACH ROW
BEGIN
    INSERT INTO `sqlkit-demo`.audit_log (db_name, table_name, op_type, row_pk, snapshot)
    VALUES (DATABASE(), 'orders', 'INSERT', NEW.id,
            JSON_OBJECT('id', NEW.id, 'order_no', NEW.order_no, 'status', NEW.status));
END$$

CREATE TRIGGER trg_orders_au AFTER UPDATE ON orders FOR EACH ROW
BEGIN
    INSERT INTO `sqlkit-demo`.audit_log (db_name, table_name, op_type, row_pk, snapshot)
    VALUES (DATABASE(), 'orders', 'UPDATE', NEW.id,
            JSON_OBJECT('id', NEW.id, 'old_status', OLD.status, 'new_status', NEW.status));
END$$

CREATE TRIGGER trg_orders_ad AFTER DELETE ON orders FOR EACH ROW
BEGIN
    INSERT INTO `sqlkit-demo`.audit_log (db_name, table_name, op_type, row_pk, snapshot)
    VALUES (DATABASE(), 'orders', 'DELETE', OLD.id,
            JSON_OBJECT('id', OLD.id, 'order_no', OLD.order_no));
END$$

-- BEFORE 触发器: 负库存归零（数据纠偏）
CREATE TRIGGER trg_products_bu BEFORE UPDATE ON products FOR EACH ROW
BEGIN
    IF NEW.stock < 0 THEN SET NEW.stock = 0; END IF;
END$$

CREATE TRIGGER trg_users_bu BEFORE UPDATE ON users FOR EACH ROW
BEGIN
    IF NEW.email <> OLD.email THEN
        INSERT INTO audit_log (db_name, table_name, op_type, row_pk, snapshot)
        VALUES (DATABASE(), 'users', 'EMAIL_CHANGED', NEW.id,
                JSON_OBJECT('old', OLD.email, 'new', NEW.email));
    END IF;
END$$

DELIMITER ;

-- ── 17. 跨库只读视图（analytics 库指向 demo 库） ──────────────────
USE `sqlkit-analytics`;

DROP VIEW IF EXISTS v_daily_sales;
DROP VIEW IF EXISTS v_user_summary;

CREATE VIEW v_daily_sales AS
SELECT o.order_date AS sales_day,
       COUNT(DISTINCT o.id) AS order_cnt,
       COALESCE(SUM(oi.quantity), 0) AS units_sold,
       COALESCE(SUM(oi.line_total), 0) AS revenue
FROM `sqlkit-demo`.orders o
LEFT JOIN `sqlkit-demo`.order_items oi
       ON oi.order_id = o.id
GROUP BY o.order_date;

CREATE OR REPLACE VIEW v_user_summary AS
SELECT u.id, u.username, u.balance,
       COUNT(DISTINCT o.id) AS order_cnt,
       COALESCE(SUM(o.total_amount), 0) AS total_spend
FROM `sqlkit-demo`.users u
LEFT JOIN `sqlkit-demo`.orders o ON o.user_id = u.id
GROUP BY u.id, u.username, u.balance;

-- billing 库: 对账视图（跨库 JOIN）
USE `sqlkit-billing`;

DROP VIEW IF EXISTS v_reconciliation;

CREATE VIEW v_reconciliation AS
SELECT p.external_no, p.amount AS paid_amount, o.total_amount AS order_amount,
       p.amount - o.total_amount AS diff
FROM `sqlkit-demo`.payments p
JOIN `sqlkit-demo`.orders o
  ON o.id = p.order_id;

-- ── 18. 业务库视图（可更新视图 + WITH CHECK OPTION + 聚合视图） ───
USE `sqlkit-demo`;

DROP VIEW IF EXISTS v_active_products;
DROP VIEW IF EXISTS v_product_catalog;
DROP VIEW IF EXISTS v_shop_with_check;

CREATE VIEW v_active_products AS
SELECT id, merchant_id, category_id, sku, name, price, stock, is_active
FROM products
WHERE is_active = 1 AND stock > 0;

-- 可更新视图（单表 + 无聚合）→ 面板 Drop/Rename 与 DML 演练
CREATE VIEW v_shop_with_check AS
SELECT id, merchant_id, sku, name, price, stock, is_active
FROM products
WHERE price < 10000
WITH CASCADED CHECK OPTION;

CREATE VIEW v_product_catalog AS
SELECT p.id, p.name, p.price, m.name AS merchant_name, c.name AS category_name
FROM products p
JOIN merchants m ON m.id = p.merchant_id
LEFT JOIN categories c ON c.id = p.category_id;

-- 完成。数据灌入请依次执行 02_seed.sql → 03_complex_queries.sql
