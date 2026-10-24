//! Tax journal templates are account and formula lines. No receivable flag.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE tax_journal_entry_item_templates DROP COLUMN IF EXISTS receivable",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE tax_journal_entry_item_templates ADD COLUMN IF NOT EXISTS receivable boolean NOT NULL DEFAULT false",
            )
            .await?;
        Ok(())
    }
}
