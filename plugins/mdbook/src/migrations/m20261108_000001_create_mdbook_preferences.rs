use sea_orm_migration::prelude::*;

use super::preferences_table::{MdBookPreferences, create_mdbook_preferences};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_mdbook_preferences(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MdBookPreferences::Table).to_owned())
            .await
    }
}
