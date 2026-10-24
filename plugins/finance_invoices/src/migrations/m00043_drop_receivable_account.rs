//! The receivable account slot is gone. Journal templates post their own lines,
//! and the amount due is the invoice total.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE payment_journal_entry_item_templates DROP COLUMN IF EXISTS receivable",
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE posted_invoices
                DROP CONSTRAINT IF EXISTS fk_posted_invoices_account_receivable_id,
                DROP COLUMN IF EXISTS account_receivable_id"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE cancelled_invoices
                DROP CONSTRAINT IF EXISTS fk_cancelled_invoices_account_receivable_id,
                DROP COLUMN IF EXISTS account_receivable_id"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE payment_journal_entry_item_templates ADD COLUMN IF NOT EXISTS receivable boolean NOT NULL DEFAULT false",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE posted_invoices ADD COLUMN IF NOT EXISTS account_receivable_id bigint NOT NULL DEFAULT 0",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE cancelled_invoices ADD COLUMN IF NOT EXISTS account_receivable_id bigint NOT NULL DEFAULT 0",
        )
        .await?;
        Ok(())
    }
}
