//! Product journal templates are account and formula lines. No receivable flag.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "ALTER TABLE product_journal_entry_item_templates DROP COLUMN IF EXISTS receivable",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager,
                "ALTER TABLE product_journal_entry_item_templates ADD COLUMN IF NOT EXISTS receivable boolean NOT NULL DEFAULT false",
            )
            .await?;
        Ok(())
    }
}
