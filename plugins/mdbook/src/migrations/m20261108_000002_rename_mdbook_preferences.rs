use sea_orm_migration::prelude::*;

/// Databases that already applied `m20261108_000001` created `md_book_preferences`.
/// The entity reads `mdbook_preferences`.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_table("md_book_preferences").await?
            && !manager.has_table("mdbook_preferences").await?
        {
            manager
                .rename_table(
                    Table::rename()
                        .table(
                            Alias::new("md_book_preferences"),
                            Alias::new("mdbook_preferences"),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_table("mdbook_preferences").await?
            && !manager.has_table("md_book_preferences").await?
        {
            manager
                .rename_table(
                    Table::rename()
                        .table(
                            Alias::new("mdbook_preferences"),
                            Alias::new("md_book_preferences"),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
