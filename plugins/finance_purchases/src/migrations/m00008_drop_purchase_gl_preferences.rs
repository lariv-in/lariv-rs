//! Purchase GL account columns are retired after the journal templates are seeded.
//!
//! `m00009_default_purchase_journal_entry_item_templates` runs first so it can
//! still read `account_payable_id` and `account_expense_id`.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            r#"ALTER TABLE purchase_preferences
                DROP COLUMN IF EXISTS account_payable_id CASCADE,
                DROP COLUMN IF EXISTS account_expense_id CASCADE,
                DROP COLUMN IF EXISTS account_input_tax_id CASCADE,
                DROP COLUMN IF EXISTS journal_id CASCADE"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE posted_purchases
                DROP COLUMN IF EXISTS account_payable_id CASCADE,
                DROP COLUMN IF EXISTS account_expense_id CASCADE,
                DROP COLUMN IF EXISTS account_input_tax_id CASCADE"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE cancelled_purchases
                DROP COLUMN IF EXISTS account_payable_id CASCADE,
                DROP COLUMN IF EXISTS account_expense_id CASCADE,
                DROP COLUMN IF EXISTS account_input_tax_id CASCADE"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            r#"ALTER TABLE purchase_preferences
                ADD COLUMN IF NOT EXISTS account_payable_id bigint NULL,
                ADD COLUMN IF NOT EXISTS account_expense_id bigint NULL,
                ADD COLUMN IF NOT EXISTS account_input_tax_id bigint NULL,
                ADD COLUMN IF NOT EXISTS journal_id bigint NULL"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE posted_purchases
                ADD COLUMN IF NOT EXISTS account_payable_id bigint NOT NULL DEFAULT 0,
                ADD COLUMN IF NOT EXISTS account_expense_id bigint NOT NULL DEFAULT 0,
                ADD COLUMN IF NOT EXISTS account_input_tax_id bigint NOT NULL DEFAULT 0"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE cancelled_purchases
                ADD COLUMN IF NOT EXISTS account_payable_id bigint NOT NULL DEFAULT 0,
                ADD COLUMN IF NOT EXISTS account_expense_id bigint NOT NULL DEFAULT 0,
                ADD COLUMN IF NOT EXISTS account_input_tax_id bigint NOT NULL DEFAULT 0"#,
        )
        .await?;
        Ok(())
    }
}
