use sea_orm_migration::prelude::*;

use super::preferences_table::{MdBookPreferences, create_mdbook_preferences};

/// Databases that already applied the text-path columns are rebuilt with
/// `source_vnode_id` and `output_vnode_id` foreign keys. The singleton row is
/// dropped; the next load creates the default directories and inserts it again.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_table("mdbook_preferences").await?
            || !manager
                .has_column("mdbook_preferences", "source_dir")
                .await?
        {
            return Ok(());
        }
        manager
            .drop_table(Table::drop().table(MdBookPreferences::Table).to_owned())
            .await?;
        create_mdbook_preferences(manager).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
