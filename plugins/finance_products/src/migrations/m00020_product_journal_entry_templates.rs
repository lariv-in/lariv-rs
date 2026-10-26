//! Per-product journal entry templates.

use lariv_core::db::migration_sql::{
    self, FK_PRODUCT_JOURNAL_ENTRY_TEMPLATES_JOURNAL, FK_PRODUCT_JOURNAL_ITEM_ACCOUNT,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS product_journal_entry_templates (
                id bigserial PRIMARY KEY,
                created_at timestamptz NULL,
                updated_at timestamptz NULL,
                product_id bigint NOT NULL UNIQUE
                    REFERENCES products(id) ON UPDATE CASCADE ON DELETE CASCADE,
                journal_id bigint NOT NULL
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_product_journal_entry_templates_journal_id ON product_journal_entry_templates (journal_id)",
        )
        .await?;
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS product_journal_entry_item_templates (
                id bigserial PRIMARY KEY,
                created_at timestamptz NULL,
                updated_at timestamptz NULL,
                product_journal_entry_template_id bigint NOT NULL
                    REFERENCES product_journal_entry_templates(id) ON UPDATE CASCADE ON DELETE CASCADE,
                account_id bigint NOT NULL,
                formula text NOT NULL,
                receivable boolean NOT NULL DEFAULT false
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_product_journal_entry_item_templates_template_id ON product_journal_entry_item_templates (product_journal_entry_template_id)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_product_journal_entry_item_templates_account_id ON product_journal_entry_item_templates (account_id)",
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[
                FK_PRODUCT_JOURNAL_ENTRY_TEMPLATES_JOURNAL,
                FK_PRODUCT_JOURNAL_ITEM_ACCOUNT,
            ],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS product_journal_entry_item_templates")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS product_journal_entry_templates")
            .await?;
        Ok(())
    }
}
