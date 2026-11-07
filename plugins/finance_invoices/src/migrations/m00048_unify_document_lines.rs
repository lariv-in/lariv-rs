//! Fold service lines into a single `*_invoice_lines` family.
use lariv_core::db::migration_sql::{
    self, FK_CANCELLED_INVOICE_LINE_PRODUCT, FK_DRAFT_INVOICE_LINE_PRODUCT,
    FK_POSTED_INVOICE_LINE_PRODUCT, exec_sql,
};

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
BEGIN
    -- ----- draft -----
    IF to_regclass('public.draft_invoice_goods_lines') IS NOT NULL THEN
        ALTER TABLE draft_invoice_goods_lines
            ADD COLUMN IF NOT EXISTS sac_code bigint NULL;
        ALTER TABLE draft_invoice_goods_lines
            ALTER COLUMN hsn_code DROP NOT NULL;

        IF to_regclass('public.draft_invoice_service_lines') IS NOT NULL THEN
            ALTER TABLE draft_invoice_goods_lines
                ADD COLUMN IF NOT EXISTS migrated_from_service_line_id bigint NULL;

            INSERT INTO draft_invoice_goods_lines (
                created_at, updated_at, draft_invoice_id, product_id,
                product_name, product_remarks, hsn_code, sac_code,
                variable_types, sales_price_formula, rate, quantity,
                variable_values, pre_tax_amount, remarks,
                migrated_from_service_line_id
            )
            SELECT
                created_at, updated_at, draft_invoice_id, service_id,
                service_name, service_remarks, NULL, sac_code,
                variable_types, sales_price_formula, rate, quantity,
                variable_values, pre_tax_amount, remarks,
                id
            FROM draft_invoice_service_lines;

            INSERT INTO draft_invoice_goods_line_taxes (
                draft_invoice_goods_line_id, tax_id
            )
            SELECT g.id, t.tax_id
              FROM draft_invoice_service_line_taxes AS t
              JOIN draft_invoice_goods_lines AS g
                ON g.migrated_from_service_line_id = t.draft_invoice_service_line_id
            ON CONFLICT DO NOTHING;

            ALTER TABLE draft_invoice_goods_lines
                DROP COLUMN migrated_from_service_line_id;
            DROP TABLE IF EXISTS draft_invoice_service_line_taxes CASCADE;
            DROP TABLE IF EXISTS draft_invoice_service_lines CASCADE;
        END IF;

        UPDATE draft_invoice_goods_lines SET product_id = 0 WHERE product_id IS NULL;
        ALTER TABLE draft_invoice_goods_lines ALTER COLUMN product_id SET NOT NULL;

        ALTER TABLE draft_invoice_goods_lines RENAME TO draft_invoice_lines;
        ALTER TABLE draft_invoice_goods_line_taxes RENAME TO draft_invoice_line_taxes;
        ALTER TABLE draft_invoice_line_taxes
            RENAME COLUMN draft_invoice_goods_line_id TO draft_invoice_line_id;
    END IF;

    -- ----- posted -----
    IF to_regclass('public.posted_invoice_goods_lines') IS NOT NULL THEN
        ALTER TABLE posted_invoice_goods_lines
            ADD COLUMN IF NOT EXISTS sac_code bigint NULL;
        ALTER TABLE posted_invoice_goods_lines
            ALTER COLUMN hsn_code DROP NOT NULL;

        IF to_regclass('public.posted_invoice_service_lines') IS NOT NULL THEN
            ALTER TABLE posted_invoice_goods_lines
                ADD COLUMN IF NOT EXISTS migrated_from_service_line_id bigint NULL;

            INSERT INTO posted_invoice_goods_lines (
                created_at, updated_at, posted_invoice_id, product_id,
                product_name, product_remarks, hsn_code, sac_code,
                variable_types, sales_price_formula, rate, quantity,
                variable_values, pre_tax_amount, remarks, journal_entry_item_id,
                migrated_from_service_line_id
            )
            SELECT
                created_at, updated_at, posted_invoice_id, service_id,
                service_name, service_remarks, NULL, sac_code,
                variable_types, sales_price_formula, rate, quantity,
                variable_values, pre_tax_amount, remarks, journal_entry_item_id,
                id
            FROM posted_invoice_service_lines;

            INSERT INTO posted_invoice_goods_line_taxes (
                posted_invoice_goods_line_id, tax_id
            )
            SELECT g.id, t.tax_id
              FROM posted_invoice_service_line_taxes AS t
              JOIN posted_invoice_goods_lines AS g
                ON g.migrated_from_service_line_id = t.posted_invoice_service_line_id
            ON CONFLICT DO NOTHING;

            ALTER TABLE posted_invoice_goods_lines
                DROP COLUMN migrated_from_service_line_id;
            DROP TABLE IF EXISTS posted_invoice_service_line_taxes CASCADE;
            DROP TABLE IF EXISTS posted_invoice_service_lines CASCADE;
        END IF;

        ALTER TABLE posted_invoice_goods_lines
            ALTER COLUMN product_id DROP NOT NULL;

        ALTER TABLE posted_invoice_goods_lines RENAME TO posted_invoice_lines;
        ALTER TABLE posted_invoice_goods_line_taxes RENAME TO posted_invoice_line_taxes;
        ALTER TABLE posted_invoice_line_taxes
            RENAME COLUMN posted_invoice_goods_line_id TO posted_invoice_line_id;
    END IF;

    -- ----- cancelled -----
    IF to_regclass('public.cancelled_invoice_goods_lines') IS NOT NULL THEN
        ALTER TABLE cancelled_invoice_goods_lines
            ADD COLUMN IF NOT EXISTS sac_code bigint NULL;
        ALTER TABLE cancelled_invoice_goods_lines
            ALTER COLUMN hsn_code DROP NOT NULL;

        IF to_regclass('public.cancelled_invoice_service_lines') IS NOT NULL THEN
            ALTER TABLE cancelled_invoice_goods_lines
                ADD COLUMN IF NOT EXISTS migrated_from_service_line_id bigint NULL;

            INSERT INTO cancelled_invoice_goods_lines (
                created_at, updated_at, cancelled_invoice_id, product_id,
                product_name, product_remarks, hsn_code, sac_code,
                variable_types, sales_price_formula, rate, quantity,
                variable_values, pre_tax_amount, remarks, journal_entry_item_id,
                migrated_from_service_line_id
            )
            SELECT
                created_at, updated_at, cancelled_invoice_id, service_id,
                service_name, service_remarks, NULL, sac_code,
                variable_types, sales_price_formula, rate, quantity,
                variable_values, pre_tax_amount, remarks, journal_entry_item_id,
                id
            FROM cancelled_invoice_service_lines;

            INSERT INTO cancelled_invoice_goods_line_taxes (
                cancelled_invoice_goods_line_id, tax_id
            )
            SELECT g.id, t.tax_id
              FROM cancelled_invoice_service_line_taxes AS t
              JOIN cancelled_invoice_goods_lines AS g
                ON g.migrated_from_service_line_id = t.cancelled_invoice_service_line_id
            ON CONFLICT DO NOTHING;

            ALTER TABLE cancelled_invoice_goods_lines
                DROP COLUMN migrated_from_service_line_id;
            DROP TABLE IF EXISTS cancelled_invoice_service_line_taxes CASCADE;
            DROP TABLE IF EXISTS cancelled_invoice_service_lines CASCADE;
        END IF;

        ALTER TABLE cancelled_invoice_goods_lines
            ALTER COLUMN product_id DROP NOT NULL;

        ALTER TABLE cancelled_invoice_goods_lines RENAME TO cancelled_invoice_lines;
        ALTER TABLE cancelled_invoice_goods_line_taxes RENAME TO cancelled_invoice_line_taxes;
        ALTER TABLE cancelled_invoice_line_taxes
            RENAME COLUMN cancelled_invoice_goods_line_id TO cancelled_invoice_line_id;
    END IF;
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, UP).await?;
        // Drop pre-rename goods constraint names so unified names/actions apply.
        migration_sql::exec_sql(
            manager,
            r#"
DO $$
BEGIN
    ALTER TABLE IF EXISTS draft_invoice_lines
        DROP CONSTRAINT IF EXISTS fk_draft_invoice_goods_lines_product_id;
    ALTER TABLE IF EXISTS posted_invoice_lines
        DROP CONSTRAINT IF EXISTS fk_posted_invoice_goods_lines_product_id;
    ALTER TABLE IF EXISTS cancelled_invoice_lines
        DROP CONSTRAINT IF EXISTS fk_cancelled_invoice_goods_lines_product_id;
END $$;
"#,
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[
                FK_DRAFT_INVOICE_LINE_PRODUCT,
                FK_POSTED_INVOICE_LINE_PRODUCT,
                FK_CANCELLED_INVOICE_LINE_PRODUCT,
            ],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
