use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum FiscalYears {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Code,
    Name,
    StartsAt,
    EndsAt,
    IsActive,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FiscalYears::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FiscalYears::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FiscalYears::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FiscalYears::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FiscalYears::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FiscalYears::Code).text().not_null())
                    .col(ColumnDef::new(FiscalYears::Name).text().not_null())
                    .col(
                        ColumnDef::new(FiscalYears::StartsAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FiscalYears::EndsAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FiscalYears::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_fiscal_years_code")
                    .table(FiscalYears::Table)
                    .col(FiscalYears::Code)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_fiscal_years_deleted_at")
                    .table(FiscalYears::Table)
                    .col(FiscalYears::DeletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FiscalYears::Table).to_owned())
            .await
    }
}
