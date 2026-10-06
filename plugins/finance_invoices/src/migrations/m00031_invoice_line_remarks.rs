//! Optional remarks on draft, posted, and cancelled invoice product lines.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DraftInvoiceLines {
    Table,
    Remarks,
}

#[derive(DeriveIden)]
enum PostedInvoiceLines {
    Table,
    Remarks,
}

#[derive(DeriveIden)]
enum CancelledInvoiceLines {
    Table,
    Remarks,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoiceLines::Table)
                    .add_column(ColumnDef::new(DraftInvoiceLines::Remarks).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoiceLines::Table)
                    .add_column(ColumnDef::new(PostedInvoiceLines::Remarks).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoiceLines::Table)
                    .add_column(ColumnDef::new(CancelledInvoiceLines::Remarks).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoiceLines::Table)
                    .drop_column(CancelledInvoiceLines::Remarks)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoiceLines::Table)
                    .drop_column(PostedInvoiceLines::Remarks)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoiceLines::Table)
                    .drop_column(DraftInvoiceLines::Remarks)
                    .to_owned(),
            )
            .await
    }
}
