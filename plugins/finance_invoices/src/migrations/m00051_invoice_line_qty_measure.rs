//! Quantity on invoice lines is a formula measure: kind, magnitude, and unit.
//!
//! Existing rows stay a count (`quantity` / `Number`). Quotation lines share
//! the draft line editor, so they get the same columns.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DraftInvoiceLines {
    Table,
    QtyKind,
    QtyUnit,
}

#[derive(DeriveIden)]
enum PostedInvoiceLines {
    Table,
}

#[derive(DeriveIden)]
enum CancelledInvoiceLines {
    Table,
}

#[derive(DeriveIden)]
enum QuotationInvoiceLines {
    Table,
}

fn add_qty_measure(table: impl IntoIden) -> TableAlterStatement {
    Table::alter()
        .table(table)
        .add_column(
            ColumnDef::new(DraftInvoiceLines::QtyKind)
                .text()
                .not_null()
                .default("quantity")
                .to_owned(),
        )
        .add_column(
            ColumnDef::new(DraftInvoiceLines::QtyUnit)
                .text()
                .not_null()
                .default("Number")
                .to_owned(),
        )
        .to_owned()
}

fn drop_qty_measure(table: impl IntoIden) -> TableAlterStatement {
    Table::alter()
        .table(table)
        .drop_column(DraftInvoiceLines::QtyKind)
        .drop_column(DraftInvoiceLines::QtyUnit)
        .to_owned()
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(add_qty_measure(DraftInvoiceLines::Table))
            .await?;
        manager
            .alter_table(add_qty_measure(PostedInvoiceLines::Table))
            .await?;
        manager
            .alter_table(add_qty_measure(CancelledInvoiceLines::Table))
            .await?;
        manager
            .alter_table(add_qty_measure(QuotationInvoiceLines::Table))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(drop_qty_measure(QuotationInvoiceLines::Table))
            .await?;
        manager
            .alter_table(drop_qty_measure(CancelledInvoiceLines::Table))
            .await?;
        manager
            .alter_table(drop_qty_measure(PostedInvoiceLines::Table))
            .await?;
        manager
            .alter_table(drop_qty_measure(DraftInvoiceLines::Table))
            .await
    }
}
