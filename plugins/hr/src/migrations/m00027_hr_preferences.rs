use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    IdCardFormat,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrPreferences::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrPreferences::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrPreferences::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrPreferences::IdCardFormat).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrPreferences::Table).to_owned())
            .await
    }
}
