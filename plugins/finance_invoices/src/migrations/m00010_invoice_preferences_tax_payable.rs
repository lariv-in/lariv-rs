use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    AccountTaxPayableId,
}

#[derive(DeriveIden)]
enum DraftInvoices {
    Table,
    AccountTaxPayableId,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .add_column(
                        ColumnDef::new(InvoicePreferences::AccountTaxPayableId).big_integer(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_invoice_preferences_account_tax_payable_id")
                    .table(InvoicePreferences::Table)
                    .col(InvoicePreferences::AccountTaxPayableId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .drop_column(DraftInvoices::AccountTaxPayableId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(
                        ColumnDef::new(DraftInvoices::AccountTaxPayableId)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_draft_invoices_account_tax_payable_id")
                            .from_tbl(DraftInvoices::Table)
                            .from_col(DraftInvoices::AccountTaxPayableId)
                            .to_tbl(Accounts::Table)
                            .to_col(Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .modify_column(
                        ColumnDef::new(DraftInvoices::AccountTaxPayableId)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_invoice_preferences_account_tax_payable_id")
                    .table(InvoicePreferences::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .drop_column(InvoicePreferences::AccountTaxPayableId)
                    .to_owned(),
            )
            .await
    }
}
