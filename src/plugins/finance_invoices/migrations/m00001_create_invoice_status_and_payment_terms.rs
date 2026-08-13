use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PaymentTermDueDates {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Datetime,
}

#[derive(DeriveIden)]
enum PaymentTermRelatives {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentTermDueDates::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PaymentTermDueDates::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PaymentTermDueDates::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentTermDueDates::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentTermDueDates::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(PaymentTermDueDates::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_payment_term_due_dates_deleted_at")
                    .table(PaymentTermDueDates::Table)
                    .col(PaymentTermDueDates::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PaymentTermRelatives::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PaymentTermRelatives::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PaymentTermRelatives::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentTermRelatives::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentTermRelatives::DeletedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_payment_term_relatives_deleted_at")
                    .table(PaymentTermRelatives::Table)
                    .col(PaymentTermRelatives::DeletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PaymentTermRelatives::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PaymentTermDueDates::Table).to_owned())
            .await
    }
}
