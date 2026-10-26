//! Per-tax journal entry templates.
//!
//! The tax's ledger account stays until the default lines are copied from it.

use lariv_core::db::migration_sql::{
    self, FK_TAX_JOURNAL_ITEM_ACCOUNT, FK_TAX_JOURNAL_ITEM_JOURNAL,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS tax_journal_entry_item_templates (
                id bigserial PRIMARY KEY,
                created_at timestamptz NULL,
                updated_at timestamptz NULL,
                tax_id bigint NOT NULL
                    REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE CASCADE,
                journal_id bigint NOT NULL,
                account_id bigint NOT NULL,
                formula text NOT NULL,
                receivable boolean NOT NULL DEFAULT false
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_tax_journal_entry_item_templates_tax_id ON tax_journal_entry_item_templates (tax_id)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_tax_journal_entry_item_templates_journal_id ON tax_journal_entry_item_templates (journal_id)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_tax_journal_entry_item_templates_account_id ON tax_journal_entry_item_templates (account_id)",
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[FK_TAX_JOURNAL_ITEM_JOURNAL, FK_TAX_JOURNAL_ITEM_ACCOUNT],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS tax_journal_entry_item_templates")
            .await?;
        Ok(())
    }
}
