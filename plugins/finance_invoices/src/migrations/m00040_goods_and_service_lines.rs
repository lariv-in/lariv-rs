//! Rename invoice lines to goods lines and add service lines.

use lariv_plugin_finance_products::migrations::product_catalog_cleanup::CLEANUP_PRODUCT_CATALOG;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
BEGIN
    IF to_regclass('public.draft_invoice_lines') IS NOT NULL THEN
        ALTER TABLE draft_invoice_lines RENAME TO draft_invoice_goods_lines;
    END IF;
    IF to_regclass('public.posted_invoice_lines') IS NOT NULL THEN
        ALTER TABLE posted_invoice_lines RENAME TO posted_invoice_goods_lines;
    END IF;
    IF to_regclass('public.cancelled_invoice_lines') IS NOT NULL THEN
        ALTER TABLE cancelled_invoice_lines RENAME TO cancelled_invoice_goods_lines;
    END IF;
    IF to_regclass('public.draft_invoice_line_taxes') IS NOT NULL THEN
        ALTER TABLE draft_invoice_line_taxes RENAME TO draft_invoice_goods_line_taxes;
    END IF;
    IF to_regclass('public.posted_invoice_line_taxes') IS NOT NULL THEN
        ALTER TABLE posted_invoice_line_taxes RENAME TO posted_invoice_goods_line_taxes;
    END IF;
    IF to_regclass('public.cancelled_invoice_line_taxes') IS NOT NULL THEN
        ALTER TABLE cancelled_invoice_line_taxes RENAME TO cancelled_invoice_goods_line_taxes;
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'draft_invoice_goods_line_taxes'
          AND column_name = 'draft_invoice_line_id'
    ) THEN
        ALTER TABLE draft_invoice_goods_line_taxes
            RENAME COLUMN draft_invoice_line_id TO draft_invoice_goods_line_id;
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'posted_invoice_goods_line_taxes'
          AND column_name = 'posted_invoice_line_id'
    ) THEN
        ALTER TABLE posted_invoice_goods_line_taxes
            RENAME COLUMN posted_invoice_line_id TO posted_invoice_goods_line_id;
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'cancelled_invoice_goods_line_taxes'
          AND column_name = 'cancelled_invoice_line_id'
    ) THEN
        ALTER TABLE cancelled_invoice_goods_line_taxes
            RENAME COLUMN cancelled_invoice_line_id TO cancelled_invoice_goods_line_id;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS draft_invoice_service_lines (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    draft_invoice_id bigint NOT NULL REFERENCES draft_invoices(id) ON UPDATE CASCADE ON DELETE CASCADE,
    service_id bigint NOT NULL REFERENCES services(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    rate numeric(19, 6) NOT NULL,
    quantity numeric(19, 6) NOT NULL,
    variable_values text NOT NULL DEFAULT '{}',
    pre_tax_amount numeric(19, 6) NOT NULL DEFAULT 0,
    remarks text NULL
);
CREATE TABLE IF NOT EXISTS posted_invoice_service_lines (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    posted_invoice_id bigint NOT NULL REFERENCES posted_invoices(id) ON UPDATE CASCADE ON DELETE CASCADE,
    service_id bigint NOT NULL REFERENCES services(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    rate numeric(19, 6) NOT NULL,
    quantity numeric(19, 6) NOT NULL,
    variable_values text NOT NULL DEFAULT '{}',
    pre_tax_amount numeric(19, 6) NOT NULL DEFAULT 0,
    remarks text NULL,
    journal_entry_item_id bigint NOT NULL REFERENCES journal_entry_items(id) ON UPDATE CASCADE ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS cancelled_invoice_service_lines (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    cancelled_invoice_id bigint NOT NULL REFERENCES cancelled_invoices(id) ON UPDATE CASCADE ON DELETE CASCADE,
    service_id bigint NOT NULL REFERENCES services(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    rate numeric(19, 6) NOT NULL,
    quantity numeric(19, 6) NOT NULL,
    variable_values text NOT NULL DEFAULT '{}',
    pre_tax_amount numeric(19, 6) NOT NULL DEFAULT 0,
    remarks text NULL,
    journal_entry_item_id bigint NOT NULL REFERENCES journal_entry_items(id) ON UPDATE CASCADE ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS draft_invoice_service_line_taxes (
    draft_invoice_service_line_id bigint NOT NULL REFERENCES draft_invoice_service_lines(id) ON UPDATE CASCADE ON DELETE CASCADE,
    tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    PRIMARY KEY (draft_invoice_service_line_id, tax_id)
);
CREATE TABLE IF NOT EXISTS posted_invoice_service_line_taxes (
    posted_invoice_service_line_id bigint NOT NULL REFERENCES posted_invoice_service_lines(id) ON UPDATE CASCADE ON DELETE CASCADE,
    tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    PRIMARY KEY (posted_invoice_service_line_id, tax_id)
);
CREATE TABLE IF NOT EXISTS cancelled_invoice_service_line_taxes (
    cancelled_invoice_service_line_id bigint NOT NULL REFERENCES cancelled_invoice_service_lines(id) ON UPDATE CASCADE ON DELETE CASCADE,
    tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    PRIMARY KEY (cancelled_invoice_service_line_id, tax_id)
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'services' AND column_name = 'legacy_product_id'
    ) THEN
        RETURN;
    END IF;

    ALTER TABLE draft_invoice_service_lines ADD COLUMN IF NOT EXISTS migrated_from_line_id bigint;
    INSERT INTO draft_invoice_service_lines (
        created_at, updated_at, draft_invoice_id, service_id, rate, quantity,
        variable_values, pre_tax_amount, remarks, migrated_from_line_id
    )
    SELECT line.created_at, line.updated_at, line.draft_invoice_id, service.id,
           line.rate, line.quantity, line.variable_values, line.pre_tax_amount, line.remarks, line.id
    FROM draft_invoice_goods_lines AS line
    JOIN services AS service ON service.legacy_product_id = line.product_id
    WHERE NOT EXISTS (
        SELECT 1 FROM draft_invoice_service_lines existing
        WHERE existing.migrated_from_line_id = line.id
    );
    INSERT INTO draft_invoice_service_line_taxes (draft_invoice_service_line_id, tax_id)
    SELECT moved.id, tax.tax_id
    FROM draft_invoice_service_lines AS moved
    JOIN draft_invoice_goods_line_taxes AS tax
      ON tax.draft_invoice_goods_line_id = moved.migrated_from_line_id
    ON CONFLICT DO NOTHING;
    DELETE FROM draft_invoice_goods_line_taxes
    WHERE draft_invoice_goods_line_id IN (
        SELECT migrated_from_line_id FROM draft_invoice_service_lines WHERE migrated_from_line_id IS NOT NULL
    );
    DELETE FROM draft_invoice_goods_lines
    WHERE id IN (
        SELECT migrated_from_line_id FROM draft_invoice_service_lines WHERE migrated_from_line_id IS NOT NULL
    );
    ALTER TABLE draft_invoice_service_lines DROP COLUMN IF EXISTS migrated_from_line_id;

    ALTER TABLE posted_invoice_service_lines ADD COLUMN IF NOT EXISTS migrated_from_line_id bigint;
    INSERT INTO posted_invoice_service_lines (
        created_at, updated_at, posted_invoice_id, service_id, rate, quantity,
        variable_values, pre_tax_amount, remarks, journal_entry_item_id, migrated_from_line_id
    )
    SELECT line.created_at, line.updated_at, line.posted_invoice_id, service.id,
           line.rate, line.quantity, line.variable_values, line.pre_tax_amount, line.remarks,
           line.journal_entry_item_id, line.id
    FROM posted_invoice_goods_lines AS line
    JOIN services AS service ON service.legacy_product_id = line.product_id
    WHERE NOT EXISTS (
        SELECT 1 FROM posted_invoice_service_lines existing
        WHERE existing.migrated_from_line_id = line.id
    );
    INSERT INTO posted_invoice_service_line_taxes (posted_invoice_service_line_id, tax_id)
    SELECT moved.id, tax.tax_id
    FROM posted_invoice_service_lines AS moved
    JOIN posted_invoice_goods_line_taxes AS tax
      ON tax.posted_invoice_goods_line_id = moved.migrated_from_line_id
    ON CONFLICT DO NOTHING;
    DELETE FROM posted_invoice_goods_line_taxes
    WHERE posted_invoice_goods_line_id IN (
        SELECT migrated_from_line_id FROM posted_invoice_service_lines WHERE migrated_from_line_id IS NOT NULL
    );
    DELETE FROM posted_invoice_goods_lines
    WHERE id IN (
        SELECT migrated_from_line_id FROM posted_invoice_service_lines WHERE migrated_from_line_id IS NOT NULL
    );
    ALTER TABLE posted_invoice_service_lines DROP COLUMN IF EXISTS migrated_from_line_id;

    ALTER TABLE cancelled_invoice_service_lines ADD COLUMN IF NOT EXISTS migrated_from_line_id bigint;
    INSERT INTO cancelled_invoice_service_lines (
        created_at, updated_at, cancelled_invoice_id, service_id, rate, quantity,
        variable_values, pre_tax_amount, remarks, journal_entry_item_id, migrated_from_line_id
    )
    SELECT line.created_at, line.updated_at, line.cancelled_invoice_id, service.id,
           line.rate, line.quantity, line.variable_values, line.pre_tax_amount, line.remarks,
           line.journal_entry_item_id, line.id
    FROM cancelled_invoice_goods_lines AS line
    JOIN services AS service ON service.legacy_product_id = line.product_id
    WHERE NOT EXISTS (
        SELECT 1 FROM cancelled_invoice_service_lines existing
        WHERE existing.migrated_from_line_id = line.id
    );
    INSERT INTO cancelled_invoice_service_line_taxes (cancelled_invoice_service_line_id, tax_id)
    SELECT moved.id, tax.tax_id
    FROM cancelled_invoice_service_lines AS moved
    JOIN cancelled_invoice_goods_line_taxes AS tax
      ON tax.cancelled_invoice_goods_line_id = moved.migrated_from_line_id
    ON CONFLICT DO NOTHING;
    DELETE FROM cancelled_invoice_goods_line_taxes
    WHERE cancelled_invoice_goods_line_id IN (
        SELECT migrated_from_line_id FROM cancelled_invoice_service_lines WHERE migrated_from_line_id IS NOT NULL
    );
    DELETE FROM cancelled_invoice_goods_lines
    WHERE id IN (
        SELECT migrated_from_line_id FROM cancelled_invoice_service_lines WHERE migrated_from_line_id IS NOT NULL
    );
    ALTER TABLE cancelled_invoice_service_lines DROP COLUMN IF EXISTS migrated_from_line_id;
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(UP).await?;
        conn.execute_unprepared(CLEANUP_PRODUCT_CATALOG).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
