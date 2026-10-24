//! Purchase-owned copies of products and services already used on purchases.
//!
//! Purchase lines stop referencing the shared `products` and `services` tables
//! and point at `purchase_products` and `purchase_services`. Only catalog rows
//! referenced by purchase goods or service lines are copied, with the same ids,
//! plus their taxes. Journal templates are not copied: a sales template debits
//! receivable and credits revenue, and the purchase seed writes the payable
//! pair instead.

use lariv_core::db::trigram;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE IF NOT EXISTS purchase_products (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    stock_id bigint NOT NULL REFERENCES inventory_stocks(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    reference text NULL,
    remarks text NULL,
    name text NOT NULL,
    hsn_code bigint NOT NULL,
    variables text NOT NULL DEFAULT '{}',
    base_price_formula text NOT NULL,
    sales_price_formula text NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_purchase_products_reference ON purchase_products (reference);
CREATE INDEX IF NOT EXISTS idx_purchase_products_stock_id ON purchase_products (stock_id);

CREATE TABLE IF NOT EXISTS purchase_product_taxes (
    product_id bigint NOT NULL REFERENCES purchase_products(id) ON UPDATE CASCADE ON DELETE CASCADE,
    tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    PRIMARY KEY (product_id, tax_id)
);

CREATE TABLE IF NOT EXISTS purchase_product_preferences (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL
);

CREATE TABLE IF NOT EXISTS purchase_product_preferences_taxes (
    product_preferences_id bigint NOT NULL REFERENCES purchase_product_preferences(id) ON UPDATE CASCADE ON DELETE CASCADE,
    tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    PRIMARY KEY (product_preferences_id, tax_id)
);

CREATE TABLE IF NOT EXISTS purchase_product_journal_entry_item_templates (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    product_id bigint NOT NULL REFERENCES purchase_products(id) ON UPDATE CASCADE ON DELETE CASCADE,
    journal_id bigint NOT NULL REFERENCES journals(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    account_id bigint NOT NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    formula text NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_purchase_product_journal_entry_item_templates_product_id
    ON purchase_product_journal_entry_item_templates (product_id);
CREATE INDEX IF NOT EXISTS idx_purchase_product_journal_entry_item_templates_journal_id
    ON purchase_product_journal_entry_item_templates (journal_id);
CREATE INDEX IF NOT EXISTS idx_purchase_product_journal_entry_item_templates_account_id
    ON purchase_product_journal_entry_item_templates (account_id);

CREATE TABLE IF NOT EXISTS purchase_services (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    reference text NULL,
    remarks text NULL,
    name text NOT NULL,
    sac_code bigint NOT NULL,
    variables text NOT NULL DEFAULT '{}',
    base_price_formula text NOT NULL,
    sales_price_formula text NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_purchase_services_reference ON purchase_services (reference);

CREATE TABLE IF NOT EXISTS purchase_service_taxes (
    service_id bigint NOT NULL REFERENCES purchase_services(id) ON UPDATE CASCADE ON DELETE CASCADE,
    tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    PRIMARY KEY (service_id, tax_id)
);

CREATE TABLE IF NOT EXISTS purchase_service_preferences (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL
);

CREATE TABLE IF NOT EXISTS purchase_service_preferences_taxes (
    service_preferences_id bigint NOT NULL REFERENCES purchase_service_preferences(id) ON UPDATE CASCADE ON DELETE CASCADE,
    tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    PRIMARY KEY (service_preferences_id, tax_id)
);

CREATE TABLE IF NOT EXISTS purchase_service_journal_entry_item_templates (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    service_id bigint NOT NULL REFERENCES purchase_services(id) ON UPDATE CASCADE ON DELETE CASCADE,
    journal_id bigint NOT NULL REFERENCES journals(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    account_id bigint NOT NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    formula text NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_purchase_service_journal_entry_item_templates_service_id
    ON purchase_service_journal_entry_item_templates (service_id);
CREATE INDEX IF NOT EXISTS idx_purchase_service_journal_entry_item_templates_journal_id
    ON purchase_service_journal_entry_item_templates (journal_id);
CREATE INDEX IF NOT EXISTS idx_purchase_service_journal_entry_item_templates_account_id
    ON purchase_service_journal_entry_item_templates (account_id);

DO $$
BEGIN
    IF to_regclass('public.products') IS NULL THEN
        RAISE EXCEPTION 'purchase products require the products table';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM products
        WHERE stock_id IS NULL
          AND id IN (
              SELECT product_id FROM draft_purchase_goods_lines
              UNION
              SELECT product_id FROM posted_purchase_goods_lines
              UNION
              SELECT product_id FROM cancelled_purchase_goods_lines
          )
    ) THEN
        RAISE EXCEPTION 'purchase goods reference a product without a stock; assign inventory_stocks.id before copying into purchase_products';
    END IF;

    INSERT INTO purchase_products (
        id, created_at, updated_at, stock_id, reference, remarks, name, hsn_code,
        variables, base_price_formula, sales_price_formula
    )
    SELECT
        id, created_at, updated_at, stock_id, reference, remarks, name, hsn_code,
        variables, base_price_formula, sales_price_formula
    FROM products
    WHERE id IN (
        SELECT product_id FROM draft_purchase_goods_lines
        UNION
        SELECT product_id FROM posted_purchase_goods_lines
        UNION
        SELECT product_id FROM cancelled_purchase_goods_lines
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO purchase_product_taxes (product_id, tax_id)
    SELECT product_id, tax_id
    FROM product_taxes
    WHERE product_id IN (SELECT id FROM purchase_products)
    ON CONFLICT DO NOTHING;

    IF to_regclass('public.services') IS NULL THEN
        RAISE EXCEPTION 'purchase services require the services table';
    END IF;

    INSERT INTO purchase_services (
        id, created_at, updated_at, reference, remarks, name, sac_code,
        variables, base_price_formula, sales_price_formula
    )
    SELECT
        id, created_at, updated_at, reference, remarks, name, sac_code,
        variables, base_price_formula, sales_price_formula
    FROM services
    WHERE id IN (
        SELECT service_id FROM draft_purchase_service_lines
        UNION
        SELECT service_id FROM posted_purchase_service_lines
        UNION
        SELECT service_id FROM cancelled_purchase_service_lines
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO purchase_service_taxes (service_id, tax_id)
    SELECT service_id, tax_id
    FROM service_taxes
    WHERE service_id IN (SELECT id FROM purchase_services)
    ON CONFLICT DO NOTHING;

    PERFORM setval(
        pg_get_serial_sequence('purchase_products', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM purchase_products), 1), 1),
        EXISTS (SELECT 1 FROM purchase_products)
    );
    PERFORM setval(
        pg_get_serial_sequence('purchase_product_preferences', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM purchase_product_preferences), 1), 1),
        EXISTS (SELECT 1 FROM purchase_product_preferences)
    );
    PERFORM setval(
        pg_get_serial_sequence('purchase_product_journal_entry_item_templates', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM purchase_product_journal_entry_item_templates), 1), 1),
        EXISTS (SELECT 1 FROM purchase_product_journal_entry_item_templates)
    );
    PERFORM setval(
        pg_get_serial_sequence('purchase_services', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM purchase_services), 1), 1),
        EXISTS (SELECT 1 FROM purchase_services)
    );
    PERFORM setval(
        pg_get_serial_sequence('purchase_service_preferences', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM purchase_service_preferences), 1), 1),
        EXISTS (SELECT 1 FROM purchase_service_preferences)
    );
    PERFORM setval(
        pg_get_serial_sequence('purchase_service_journal_entry_item_templates', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM purchase_service_journal_entry_item_templates), 1), 1),
        EXISTS (SELECT 1 FROM purchase_service_journal_entry_item_templates)
    );
END $$;

DO $$
DECLARE
    r record;
BEGIN
    FOR r IN
        SELECT c.conname AS constraint_name, src.relname AS table_name
        FROM pg_constraint c
        JOIN pg_class src ON src.oid = c.conrelid
        JOIN pg_class dst ON dst.oid = c.confrelid
        JOIN pg_namespace n ON n.oid = src.relnamespace
        WHERE c.contype = 'f'
          AND n.nspname = 'public'
          AND dst.relname = 'products'
          AND src.relname IN (
              'draft_purchase_goods_lines',
              'posted_purchase_goods_lines',
              'cancelled_purchase_goods_lines'
          )
    LOOP
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I', r.table_name, r.constraint_name);
    END LOOP;

    FOR r IN
        SELECT c.conname AS constraint_name, src.relname AS table_name
        FROM pg_constraint c
        JOIN pg_class src ON src.oid = c.conrelid
        JOIN pg_class dst ON dst.oid = c.confrelid
        JOIN pg_namespace n ON n.oid = src.relnamespace
        WHERE c.contype = 'f'
          AND n.nspname = 'public'
          AND dst.relname = 'services'
          AND src.relname IN (
              'draft_purchase_service_lines',
              'posted_purchase_service_lines',
              'cancelled_purchase_service_lines'
          )
    LOOP
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I', r.table_name, r.constraint_name);
    END LOOP;
END $$;

ALTER TABLE draft_purchase_goods_lines
    DROP CONSTRAINT IF EXISTS draft_purchase_goods_lines_purchase_product_id_fkey;
ALTER TABLE draft_purchase_goods_lines
    ADD CONSTRAINT draft_purchase_goods_lines_purchase_product_id_fkey
    FOREIGN KEY (product_id) REFERENCES purchase_products(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;

ALTER TABLE posted_purchase_goods_lines
    DROP CONSTRAINT IF EXISTS posted_purchase_goods_lines_purchase_product_id_fkey;
ALTER TABLE posted_purchase_goods_lines
    ADD CONSTRAINT posted_purchase_goods_lines_purchase_product_id_fkey
    FOREIGN KEY (product_id) REFERENCES purchase_products(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;

ALTER TABLE cancelled_purchase_goods_lines
    DROP CONSTRAINT IF EXISTS cancelled_purchase_goods_lines_purchase_product_id_fkey;
ALTER TABLE cancelled_purchase_goods_lines
    ADD CONSTRAINT cancelled_purchase_goods_lines_purchase_product_id_fkey
    FOREIGN KEY (product_id) REFERENCES purchase_products(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;

ALTER TABLE draft_purchase_service_lines
    DROP CONSTRAINT IF EXISTS draft_purchase_service_lines_purchase_service_id_fkey;
ALTER TABLE draft_purchase_service_lines
    ADD CONSTRAINT draft_purchase_service_lines_purchase_service_id_fkey
    FOREIGN KEY (service_id) REFERENCES purchase_services(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;

ALTER TABLE posted_purchase_service_lines
    DROP CONSTRAINT IF EXISTS posted_purchase_service_lines_purchase_service_id_fkey;
ALTER TABLE posted_purchase_service_lines
    ADD CONSTRAINT posted_purchase_service_lines_purchase_service_id_fkey
    FOREIGN KEY (service_id) REFERENCES purchase_services(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;

ALTER TABLE cancelled_purchase_service_lines
    DROP CONSTRAINT IF EXISTS cancelled_purchase_service_lines_purchase_service_id_fkey;
ALTER TABLE cancelled_purchase_service_lines
    ADD CONSTRAINT cancelled_purchase_service_lines_purchase_service_id_fkey
    FOREIGN KEY (service_id) REFERENCES purchase_services(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;
"#;

const DOWN: &str = r#"
ALTER TABLE draft_purchase_goods_lines
    DROP CONSTRAINT IF EXISTS draft_purchase_goods_lines_purchase_product_id_fkey;
ALTER TABLE posted_purchase_goods_lines
    DROP CONSTRAINT IF EXISTS posted_purchase_goods_lines_purchase_product_id_fkey;
ALTER TABLE cancelled_purchase_goods_lines
    DROP CONSTRAINT IF EXISTS cancelled_purchase_goods_lines_purchase_product_id_fkey;
ALTER TABLE draft_purchase_service_lines
    DROP CONSTRAINT IF EXISTS draft_purchase_service_lines_purchase_service_id_fkey;
ALTER TABLE posted_purchase_service_lines
    DROP CONSTRAINT IF EXISTS posted_purchase_service_lines_purchase_service_id_fkey;
ALTER TABLE cancelled_purchase_service_lines
    DROP CONSTRAINT IF EXISTS cancelled_purchase_service_lines_purchase_service_id_fkey;

ALTER TABLE draft_purchase_goods_lines
    ADD CONSTRAINT draft_purchase_goods_lines_product_id_fkey
    FOREIGN KEY (product_id) REFERENCES products(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE posted_purchase_goods_lines
    ADD CONSTRAINT posted_purchase_goods_lines_product_id_fkey
    FOREIGN KEY (product_id) REFERENCES products(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE cancelled_purchase_goods_lines
    ADD CONSTRAINT cancelled_purchase_goods_lines_product_id_fkey
    FOREIGN KEY (product_id) REFERENCES products(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE draft_purchase_service_lines
    ADD CONSTRAINT draft_purchase_service_lines_service_id_fkey
    FOREIGN KEY (service_id) REFERENCES services(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE posted_purchase_service_lines
    ADD CONSTRAINT posted_purchase_service_lines_service_id_fkey
    FOREIGN KEY (service_id) REFERENCES services(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE cancelled_purchase_service_lines
    ADD CONSTRAINT cancelled_purchase_service_lines_service_id_fkey
    FOREIGN KEY (service_id) REFERENCES services(id)
    ON UPDATE CASCADE ON DELETE RESTRICT;

DROP TABLE IF EXISTS purchase_product_journal_entry_item_templates;
DROP TABLE IF EXISTS purchase_product_preferences_taxes;
DROP TABLE IF EXISTS purchase_product_preferences;
DROP TABLE IF EXISTS purchase_product_taxes;
DROP TABLE IF EXISTS purchase_products;
DROP TABLE IF EXISTS purchase_service_journal_entry_item_templates;
DROP TABLE IF EXISTS purchase_service_preferences_taxes;
DROP TABLE IF EXISTS purchase_service_preferences;
DROP TABLE IF EXISTS purchase_service_taxes;
DROP TABLE IF EXISTS purchase_services;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(UP).await?;
        let backend = manager.get_database_backend();
        trigram::create_gin_index(
            conn,
            backend,
            "purchase_products_name_trgm_idx",
            "purchase_products",
            "name",
        )
        .await?;
        trigram::create_gin_index(
            conn,
            backend,
            "purchase_products_reference_trgm_idx",
            "purchase_products",
            "reference",
        )
        .await?;
        trigram::create_gin_index(
            conn,
            backend,
            "purchase_services_name_trgm_idx",
            "purchase_services",
            "name",
        )
        .await?;
        trigram::create_gin_index(
            conn,
            backend,
            "purchase_services_reference_trgm_idx",
            "purchase_services",
            "reference",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        Ok(())
    }
}
