//! Invoice PDF logo/signature assets stored as filesystem VNode ids.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    InvoiceLogoVnodeId,
    InvoiceSignatureVnodeId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .add_column(
                        ColumnDef::new(InvoicePreferences::InvoiceLogoVnodeId)
                            .big_integer()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(InvoicePreferences::InvoiceSignatureVnodeId)
                            .big_integer()
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
                    .drop_column(InvoicePreferences::InvoiceLogoVnodeId)
                    .drop_column(InvoicePreferences::InvoiceSignatureVnodeId)
                    .to_owned(),
            )
            .await
    }
}
