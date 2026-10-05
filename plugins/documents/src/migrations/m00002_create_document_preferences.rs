use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DocumentPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    SigningAuthorityName,
    ValidityDuration,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DocumentPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DocumentPreferences::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DocumentPreferences::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DocumentPreferences::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(DocumentPreferences::SigningAuthorityName)
                            .text()
                            .not_null()
                            .default("Lariv"),
                    )
                    .col(
                        ColumnDef::new(DocumentPreferences::ValidityDuration)
                            .big_integer()
                            .not_null()
                            .default(31_536_000_000_000_000_i64),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DocumentPreferences::Table).to_owned())
            .await
    }
}
