//! Per-product journal entry templates.
use crate::foreign_keys::{
    FK_PRODUCT_JOURNAL_ENTRY_TEMPLATES_JOURNAL, FK_PRODUCT_JOURNAL_ITEM_ACCOUNT,
};
use lariv_core::db::migration_sql::{self, exec_sql};

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
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
        exec_sql(manager,
            "CREATE INDEX IF NOT EXISTS idx_product_journal_entry_templates_journal_id ON product_journal_entry_templates (journal_id)",
        )
        .await?;
        exec_sql(manager,
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
        exec_sql(manager,
            "CREATE INDEX IF NOT EXISTS idx_product_journal_entry_item_templates_template_id ON product_journal_entry_item_templates (product_journal_entry_template_id)",
        )
        .await?;
        exec_sql(manager,
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
        exec_sql(
            manager,
            "DROP TABLE IF EXISTS product_journal_entry_item_templates",
        )
        .await?;
        exec_sql(
            manager,
            "DROP TABLE IF EXISTS product_journal_entry_templates",
        )
        .await?;
        Ok(())
    }
}
