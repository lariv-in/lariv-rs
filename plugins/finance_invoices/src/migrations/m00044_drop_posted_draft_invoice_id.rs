//! Posted invoices no longer keep a link to the draft they were posted from.
//! Drafts are deleted on post; this migration purges historical posted drafts.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"CREATE TEMP TABLE IF NOT EXISTS _posted_drafts AS
               SELECT draft_invoice_id AS id FROM posted_invoices
               WHERE draft_invoice_id IS NOT NULL"#,
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS fk_posted_invoices_draft_invoice_id",
        )
        .await?;
        conn.execute_unprepared("DROP INDEX IF EXISTS uix_posted_invoices_draft_invoice_id")
            .await?;
        conn.execute_unprepared(
            "ALTER TABLE posted_invoices DROP COLUMN IF EXISTS draft_invoice_id",
        )
        .await?;
        conn.execute_unprepared(
            "DELETE FROM draft_invoices WHERE id IN (SELECT id FROM _posted_drafts)",
        )
        .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS _posted_drafts")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        // Historical draft rows are gone; restore a nullable column without backfill.
        conn.execute_unprepared(
            "ALTER TABLE posted_invoices ADD COLUMN IF NOT EXISTS draft_invoice_id bigint",
        )
        .await?;
        Ok(())
    }
}
