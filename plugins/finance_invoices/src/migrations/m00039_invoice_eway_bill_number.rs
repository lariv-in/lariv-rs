//! Optional E-Way bill number on draft, posted, and cancelled invoices.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DraftInvoices {
    Table,
    EwayBillNumber,
}

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    EwayBillNumber,
}

#[derive(DeriveIden)]
enum CancelledInvoices {
    Table,
    EwayBillNumber,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(ColumnDef::new(DraftInvoices::EwayBillNumber).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .add_column(ColumnDef::new(PostedInvoices::EwayBillNumber).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .add_column(
                        ColumnDef::new(CancelledInvoices::EwayBillNumber)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .drop_column(CancelledInvoices::EwayBillNumber)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .drop_column(PostedInvoices::EwayBillNumber)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .drop_column(DraftInvoices::EwayBillNumber)
                    .to_owned(),
            )
            .await
    }
}
