use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebsitePreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WebsitePreferences::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WebsitePreferences::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(WebsitePreferences::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(WebsitePreferences::CustomThemeCssVnodeId).big_integer(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WebsitePreferences::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WebsitePreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    CustomThemeCssVnodeId,
}
