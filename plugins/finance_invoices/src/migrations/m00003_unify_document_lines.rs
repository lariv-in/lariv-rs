//! Fold quotation service lines into `quotation_lines`.
use lariv_core::db::migration_sql::{
    self, FK_QUOTATION_LINE_PRODUCT, FK_QUOTATION_LINE_TAX, exec_sql,
};

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
BEGIN
    IF to_regclass('public.quotation_goods_lines') IS NULL THEN
        RETURN;
    END IF;

    IF to_regclass('public.quotation_service_lines') IS NOT NULL THEN
        ALTER TABLE quotation_goods_lines
            ADD COLUMN IF NOT EXISTS migrated_from_service_line_id bigint NULL;

        INSERT INTO quotation_goods_lines (
            created_at, updated_at, quotation_id, product_id,
            rate, quantity, variable_values, pre_tax_amount, remarks,
            migrated_from_service_line_id
        )
        SELECT
            created_at, updated_at, quotation_id, service_id,
            rate, quantity, variable_values, pre_tax_amount, remarks,
            id
        FROM quotation_service_lines;

        INSERT INTO quotation_goods_line_taxes (line_id, tax_id)
        SELECT g.id, t.tax_id
          FROM quotation_service_line_taxes AS t
          JOIN quotation_goods_lines AS g
            ON g.migrated_from_service_line_id = t.line_id
        ON CONFLICT DO NOTHING;

        ALTER TABLE quotation_goods_lines
            DROP COLUMN migrated_from_service_line_id;
        DROP TABLE IF EXISTS quotation_service_line_taxes CASCADE;
        DROP TABLE IF EXISTS quotation_service_lines CASCADE;
    END IF;

    UPDATE quotation_goods_lines SET product_id = 0 WHERE product_id IS NULL;
    ALTER TABLE quotation_goods_lines ALTER COLUMN product_id SET NOT NULL;

    ALTER TABLE quotation_goods_lines RENAME TO quotation_lines;
    ALTER TABLE quotation_goods_line_taxes RENAME TO quotation_line_taxes;
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, UP).await?;
        migration_sql::exec_sql(
            manager,
            r#"
DO $$
BEGIN
    ALTER TABLE IF EXISTS quotation_lines
        DROP CONSTRAINT IF EXISTS fk_quotation_goods_lines_product_id;
    ALTER TABLE IF EXISTS quotation_line_taxes
        DROP CONSTRAINT IF EXISTS fk_quotation_goods_line_taxes_tax_id;
END $$;
"#,
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[FK_QUOTATION_LINE_PRODUCT, FK_QUOTATION_LINE_TAX],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
