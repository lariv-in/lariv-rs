//! Invoice GL preference columns are retired after the journal templates are seeded.
//!
//! Product, service, and tax templates copy `account_receivable_id`,
//! `account_revenue_id`, `account_tax_payable_id`, and `journal_id` in their
//! own plugins, which migrate first. `m00042_payment_journal_entry_item_templates`
//! runs before this migration and copies the receivable account onto the
//! payment template.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            r#"ALTER TABLE invoice_preferences
                DROP COLUMN IF EXISTS account_receivable_id,
                DROP COLUMN IF EXISTS account_revenue_id,
                DROP COLUMN IF EXISTS account_tax_payable_id,
                DROP COLUMN IF EXISTS journal_id"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE posted_invoices
                DROP COLUMN IF EXISTS account_revenue_id,
                DROP COLUMN IF EXISTS account_tax_payable_id"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE cancelled_invoices
                DROP COLUMN IF EXISTS account_revenue_id,
                DROP COLUMN IF EXISTS account_tax_payable_id"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            r#"ALTER TABLE invoice_preferences
                ADD COLUMN IF NOT EXISTS account_receivable_id bigint NULL,
                ADD COLUMN IF NOT EXISTS account_revenue_id bigint NULL,
                ADD COLUMN IF NOT EXISTS account_tax_payable_id bigint NULL,
                ADD COLUMN IF NOT EXISTS journal_id bigint NULL"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE posted_invoices
                ADD COLUMN IF NOT EXISTS account_revenue_id bigint NOT NULL DEFAULT 0,
                ADD COLUMN IF NOT EXISTS account_tax_payable_id bigint NOT NULL DEFAULT 0"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE cancelled_invoices
                ADD COLUMN IF NOT EXISTS account_revenue_id bigint NOT NULL DEFAULT 0,
                ADD COLUMN IF NOT EXISTS account_tax_payable_id bigint NOT NULL DEFAULT 0"#,
        )
        .await?;
        Ok(())
    }
}
