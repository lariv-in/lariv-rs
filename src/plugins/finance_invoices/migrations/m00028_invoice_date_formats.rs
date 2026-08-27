//! Chrono format strings for invoice PDF date / datetime display.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    InvoiceDateFormat,
    InvoiceDatetimeFormat,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .add_column(
                        ColumnDef::new(InvoicePreferences::InvoiceDateFormat)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(InvoicePreferences::InvoiceDatetimeFormat)
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
                    .table(InvoicePreferences::Table)
                    .drop_column(InvoicePreferences::InvoiceDatetimeFormat)
                    .drop_column(InvoicePreferences::InvoiceDateFormat)
                    .to_owned(),
            )
            .await
    }
}
