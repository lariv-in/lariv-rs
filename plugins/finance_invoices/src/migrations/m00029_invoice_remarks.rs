//! Optional remarks on draft, posted, and cancelled invoices.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DraftInvoices {
    Table,
    Remarks,
}

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    Remarks,
}

#[derive(DeriveIden)]
enum CancelledInvoices {
    Table,
    Remarks,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(ColumnDef::new(DraftInvoices::Remarks).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .add_column(ColumnDef::new(PostedInvoices::Remarks).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .add_column(ColumnDef::new(CancelledInvoices::Remarks).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .drop_column(CancelledInvoices::Remarks)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .drop_column(PostedInvoices::Remarks)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .drop_column(DraftInvoices::Remarks)
                    .to_owned(),
            )
            .await
    }
}
