use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    AccountReceivableId,
    AccountRevenueId,
    JournalId,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Journals {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(InvoicePreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InvoicePreferences::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(InvoicePreferences::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(InvoicePreferences::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(InvoicePreferences::DeletedAt).timestamp_with_time_zone(),
                    )
                    .col(ColumnDef::new(InvoicePreferences::AccountReceivableId).big_integer())
                    .col(ColumnDef::new(InvoicePreferences::AccountRevenueId).big_integer())
                    .col(ColumnDef::new(InvoicePreferences::JournalId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invoice_preferences_account_receivable_id")
                            .from(InvoicePreferences::Table, InvoicePreferences::AccountReceivableId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invoice_preferences_account_revenue_id")
                            .from(InvoicePreferences::Table, InvoicePreferences::AccountRevenueId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invoice_preferences_journal_id")
                            .from(InvoicePreferences::Table, InvoicePreferences::JournalId)
                            .to(Journals::Table, Journals::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_invoice_preferences_deleted_at")
                    .table(InvoicePreferences::Table)
                    .col(InvoicePreferences::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_invoice_preferences_account_receivable_id")
                    .table(InvoicePreferences::Table)
                    .col(InvoicePreferences::AccountReceivableId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_invoice_preferences_account_revenue_id")
                    .table(InvoicePreferences::Table)
                    .col(InvoicePreferences::AccountRevenueId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_invoice_preferences_journal_id")
                    .table(InvoicePreferences::Table)
                    .col(InvoicePreferences::JournalId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(InvoicePreferences::Table).to_owned())
            .await
    }
}
