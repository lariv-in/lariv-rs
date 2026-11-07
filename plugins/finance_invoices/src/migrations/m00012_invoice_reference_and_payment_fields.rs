use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DraftInvoices {
    Table,
    Reference,
    PaymentReference,
    BankAccount,
}

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    Reference,
    PaymentReference,
    BankAccount,
}

#[derive(DeriveIden)]
enum CancelledInvoices {
    Table,
    Reference,
    PaymentReference,
    BankAccount,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(ColumnDef::new(DraftInvoices::Reference).text())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(ColumnDef::new(DraftInvoices::PaymentReference).text())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(ColumnDef::new(DraftInvoices::BankAccount).text())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .add_column(ColumnDef::new(PostedInvoices::Reference).text())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .add_column(ColumnDef::new(PostedInvoices::PaymentReference).text())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .add_column(ColumnDef::new(PostedInvoices::BankAccount).text())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .add_column(ColumnDef::new(CancelledInvoices::Reference).text())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .add_column(ColumnDef::new(CancelledInvoices::PaymentReference).text())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .add_column(ColumnDef::new(CancelledInvoices::BankAccount).text())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .drop_column(CancelledInvoices::BankAccount)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .drop_column(CancelledInvoices::PaymentReference)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .drop_column(CancelledInvoices::Reference)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .drop_column(PostedInvoices::BankAccount)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .drop_column(PostedInvoices::PaymentReference)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .drop_column(PostedInvoices::Reference)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .drop_column(DraftInvoices::BankAccount)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .drop_column(DraftInvoices::PaymentReference)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .drop_column(DraftInvoices::Reference)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
