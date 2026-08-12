use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Currencies {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Code,
    Name,
    Symbol,
    MinorUnit,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Currencies::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Currencies::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Currencies::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Currencies::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Currencies::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Currencies::Code).integer().not_null().unique_key())
                    .col(ColumnDef::new(Currencies::Name).text().not_null())
                    .col(
                        ColumnDef::new(Currencies::Symbol)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Currencies::MinorUnit).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_currencies_deleted_at")
                    .table(Currencies::Table)
                    .col(Currencies::DeletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Currencies::Table).to_owned())
            .await
    }
}
