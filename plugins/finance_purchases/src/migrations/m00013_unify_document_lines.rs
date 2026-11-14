//! Fold purchase service lines into a single `*_purchase_lines` family.
use crate::foreign_keys::{
    FK_CANCELLED_PURCHASE_LINE_PRODUCT, FK_DRAFT_PURCHASE_LINE_PRODUCT,
    FK_POSTED_PURCHASE_LINE_PRODUCT,
};
use lariv_core::db::migration_sql::{self, exec_sql};

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
BEGIN
    -- ----- draft -----
    IF to_regclass('public.draft_purchase_goods_lines') IS NOT NULL THEN
        IF to_regclass('public.draft_purchase_service_lines') IS NOT NULL THEN
            ALTER TABLE draft_purchase_goods_lines
                ADD COLUMN IF NOT EXISTS migrated_from_service_line_id bigint NULL;

            INSERT INTO draft_purchase_goods_lines (
                created_at, updated_at, draft_purchase_id, product_id,
                rate, quantity, variable_values, pre_tax_amount, remarks,
                migrated_from_service_line_id
            )
            SELECT
                created_at, updated_at, draft_purchase_id, service_id,
                rate, quantity, variable_values, pre_tax_amount, remarks,
                id
            FROM draft_purchase_service_lines;

            INSERT INTO draft_purchase_goods_line_taxes (
                draft_purchase_goods_line_id, tax_id
            )
            SELECT g.id, t.tax_id
              FROM draft_purchase_service_line_taxes AS t
              JOIN draft_purchase_goods_lines AS g
                ON g.migrated_from_service_line_id = t.draft_purchase_service_line_id
            ON CONFLICT DO NOTHING;

            ALTER TABLE draft_purchase_goods_lines
                DROP COLUMN migrated_from_service_line_id;
            DROP TABLE IF EXISTS draft_purchase_service_line_taxes CASCADE;
            DROP TABLE IF EXISTS draft_purchase_service_lines CASCADE;
        END IF;

        UPDATE draft_purchase_goods_lines SET product_id = 0 WHERE product_id IS NULL;
        ALTER TABLE draft_purchase_goods_lines ALTER COLUMN product_id SET NOT NULL;

        ALTER TABLE draft_purchase_goods_lines RENAME TO draft_purchase_lines;
        ALTER TABLE draft_purchase_goods_line_taxes RENAME TO draft_purchase_line_taxes;
        ALTER TABLE draft_purchase_line_taxes
            RENAME COLUMN draft_purchase_goods_line_id TO draft_purchase_line_id;
    END IF;

    -- ----- posted -----
    IF to_regclass('public.posted_purchase_goods_lines') IS NOT NULL THEN
        IF to_regclass('public.posted_purchase_service_lines') IS NOT NULL THEN
            ALTER TABLE posted_purchase_goods_lines
                ADD COLUMN IF NOT EXISTS migrated_from_service_line_id bigint NULL;

            INSERT INTO posted_purchase_goods_lines (
                created_at, updated_at, posted_purchase_id, product_id,
                rate, quantity, variable_values, pre_tax_amount, remarks,
                journal_entry_item_id, migrated_from_service_line_id
            )
            SELECT
                created_at, updated_at, posted_purchase_id, service_id,
                rate, quantity, variable_values, pre_tax_amount, remarks,
                journal_entry_item_id, id
            FROM posted_purchase_service_lines;

            INSERT INTO posted_purchase_goods_line_taxes (
                posted_purchase_goods_line_id, tax_id
            )
            SELECT g.id, t.tax_id
              FROM posted_purchase_service_line_taxes AS t
              JOIN posted_purchase_goods_lines AS g
                ON g.migrated_from_service_line_id = t.posted_purchase_service_line_id
            ON CONFLICT DO NOTHING;

            ALTER TABLE posted_purchase_goods_lines
                DROP COLUMN migrated_from_service_line_id;
            DROP TABLE IF EXISTS posted_purchase_service_line_taxes CASCADE;
            DROP TABLE IF EXISTS posted_purchase_service_lines CASCADE;
        END IF;

        ALTER TABLE posted_purchase_goods_lines
            ALTER COLUMN product_id DROP NOT NULL;

        ALTER TABLE posted_purchase_goods_lines RENAME TO posted_purchase_lines;
        ALTER TABLE posted_purchase_goods_line_taxes RENAME TO posted_purchase_line_taxes;
        ALTER TABLE posted_purchase_line_taxes
            RENAME COLUMN posted_purchase_goods_line_id TO posted_purchase_line_id;
    END IF;

    -- ----- cancelled -----
    IF to_regclass('public.cancelled_purchase_goods_lines') IS NOT NULL THEN
        IF to_regclass('public.cancelled_purchase_service_lines') IS NOT NULL THEN
            ALTER TABLE cancelled_purchase_goods_lines
                ADD COLUMN IF NOT EXISTS migrated_from_service_line_id bigint NULL;

            INSERT INTO cancelled_purchase_goods_lines (
                created_at, updated_at, cancelled_purchase_id, product_id,
                rate, quantity, variable_values, pre_tax_amount, remarks,
                journal_entry_item_id, migrated_from_service_line_id
            )
            SELECT
                created_at, updated_at, cancelled_purchase_id, service_id,
                rate, quantity, variable_values, pre_tax_amount, remarks,
                journal_entry_item_id, id
            FROM cancelled_purchase_service_lines;

            INSERT INTO cancelled_purchase_goods_line_taxes (
                cancelled_purchase_goods_line_id, tax_id
            )
            SELECT g.id, t.tax_id
              FROM cancelled_purchase_service_line_taxes AS t
              JOIN cancelled_purchase_goods_lines AS g
                ON g.migrated_from_service_line_id = t.cancelled_purchase_service_line_id
            ON CONFLICT DO NOTHING;

            ALTER TABLE cancelled_purchase_goods_lines
                DROP COLUMN migrated_from_service_line_id;
            DROP TABLE IF EXISTS cancelled_purchase_service_line_taxes CASCADE;
            DROP TABLE IF EXISTS cancelled_purchase_service_lines CASCADE;
        END IF;

        ALTER TABLE cancelled_purchase_goods_lines
            ALTER COLUMN product_id DROP NOT NULL;

        ALTER TABLE cancelled_purchase_goods_lines RENAME TO cancelled_purchase_lines;
        ALTER TABLE cancelled_purchase_goods_line_taxes RENAME TO cancelled_purchase_line_taxes;
        ALTER TABLE cancelled_purchase_line_taxes
            RENAME COLUMN cancelled_purchase_goods_line_id TO cancelled_purchase_line_id;
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
    ALTER TABLE IF EXISTS draft_purchase_lines
        DROP CONSTRAINT IF EXISTS fk_draft_purchase_goods_lines_product_id;
    ALTER TABLE IF EXISTS posted_purchase_lines
        DROP CONSTRAINT IF EXISTS fk_posted_purchase_goods_lines_product_id;
    ALTER TABLE IF EXISTS cancelled_purchase_lines
        DROP CONSTRAINT IF EXISTS fk_cancelled_purchase_goods_lines_product_id;
END $$;
"#,
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[
                FK_DRAFT_PURCHASE_LINE_PRODUCT,
                FK_POSTED_PURCHASE_LINE_PRODUCT,
                FK_CANCELLED_PURCHASE_LINE_PRODUCT,
            ],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
