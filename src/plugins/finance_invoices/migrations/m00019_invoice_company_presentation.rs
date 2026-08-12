//! Company presentation fields for invoice PDF templates.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    CompanyAddress,
    CompanyPhone,
    CompanyGstin,
    PlaceOfSupply,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .add_column(
                        ColumnDef::new(InvoicePreferences::CompanyAddress)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(InvoicePreferences::CompanyPhone)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(InvoicePreferences::CompanyGstin)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(InvoicePreferences::PlaceOfSupply)
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
                    .drop_column(InvoicePreferences::CompanyAddress)
                    .drop_column(InvoicePreferences::CompanyPhone)
                    .drop_column(InvoicePreferences::CompanyGstin)
                    .drop_column(InvoicePreferences::PlaceOfSupply)
                    .to_owned(),
            )
            .await
    }
}
