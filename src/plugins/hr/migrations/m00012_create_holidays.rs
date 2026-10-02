use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrHolidays {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Title,
    Description,
    Date,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrHolidays::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrHolidays::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrHolidays::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrHolidays::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrHolidays::Title).text().not_null())
                    .col(ColumnDef::new(HrHolidays::Description).text().not_null())
                    .col(ColumnDef::new(HrHolidays::Date).date().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_holidays_date")
                    .table(HrHolidays::Table)
                    .col(HrHolidays::Date)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrHolidays::Table).to_owned())
            .await
    }
}
