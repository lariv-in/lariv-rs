use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Payments {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    PostedInvoiceId,
    Amount,
    AccountId,
    Datetime,
    JournalEntryId,
}

#[derive(DeriveIden)]
enum PartiallyPaidInvoices {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    PaymentId,
    PostedInvoiceId,
    PriorPartiallyPaidInvoiceId,
}

#[derive(DeriveIden)]
enum PaidInvoices {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    PaymentId,
    PostedInvoiceId,
    PriorPartiallyPaidInvoiceId,
}

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum JournalEntries {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Payments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Payments::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Payments::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Payments::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Payments::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Payments::PostedInvoiceId).big_integer().not_null())
                    .col(
                        ColumnDef::new(Payments::Amount)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Payments::AccountId).big_integer().not_null())
                    .col(
                        ColumnDef::new(Payments::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Payments::JournalEntryId).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payments_posted_invoice_id")
                            .from(Payments::Table, Payments::PostedInvoiceId)
                            .to(PostedInvoices::Table, PostedInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payments_account_id")
                            .from(Payments::Table, Payments::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payments_journal_entry_id")
                            .from(Payments::Table, Payments::JournalEntryId)
                            .to(JournalEntries::Table, JournalEntries::Id)
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
                    .name("idx_payments_deleted_at")
                    .table(Payments::Table)
                    .col(Payments::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_payments_posted_invoice_id")
                    .table(Payments::Table)
                    .col(Payments::PostedInvoiceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartiallyPaidInvoices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PartiallyPaidInvoices::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PartiallyPaidInvoices::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PartiallyPaidInvoices::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PartiallyPaidInvoices::DeletedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PartiallyPaidInvoices::PaymentId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PartiallyPaidInvoices::PostedInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PartiallyPaidInvoices::PriorPartiallyPaidInvoiceId)
                            .big_integer(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_partially_paid_invoices_payment_id")
                            .from(PartiallyPaidInvoices::Table, PartiallyPaidInvoices::PaymentId)
                            .to(Payments::Table, Payments::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_partially_paid_invoices_posted_invoice_id")
                            .from(
                                PartiallyPaidInvoices::Table,
                                PartiallyPaidInvoices::PostedInvoiceId,
                            )
                            .to(PostedInvoices::Table, PostedInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_partially_paid_invoices_prior_partially_paid_invoice_id")
                            .from(
                                PartiallyPaidInvoices::Table,
                                PartiallyPaidInvoices::PriorPartiallyPaidInvoiceId,
                            )
                            .to(PartiallyPaidInvoices::Table, PartiallyPaidInvoices::Id)
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
                    .name("uix_partially_paid_invoices_payment_id")
                    .table(PartiallyPaidInvoices::Table)
                    .col(PartiallyPaidInvoices::PaymentId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_partially_paid_invoices_deleted_at")
                    .table(PartiallyPaidInvoices::Table)
                    .col(PartiallyPaidInvoices::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_partially_paid_invoices_posted_invoice_id")
                    .table(PartiallyPaidInvoices::Table)
                    .col(PartiallyPaidInvoices::PostedInvoiceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PaidInvoices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PaidInvoices::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PaidInvoices::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaidInvoices::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaidInvoices::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaidInvoices::PaymentId).big_integer().not_null())
                    .col(
                        ColumnDef::new(PaidInvoices::PostedInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PaidInvoices::PriorPartiallyPaidInvoiceId).big_integer(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_paid_invoices_payment_id")
                            .from(PaidInvoices::Table, PaidInvoices::PaymentId)
                            .to(Payments::Table, Payments::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_paid_invoices_posted_invoice_id")
                            .from(PaidInvoices::Table, PaidInvoices::PostedInvoiceId)
                            .to(PostedInvoices::Table, PostedInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_paid_invoices_prior_partially_paid_invoice_id")
                            .from(
                                PaidInvoices::Table,
                                PaidInvoices::PriorPartiallyPaidInvoiceId,
                            )
                            .to(PartiallyPaidInvoices::Table, PartiallyPaidInvoices::Id)
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
                    .name("uix_paid_invoices_payment_id")
                    .table(PaidInvoices::Table)
                    .col(PaidInvoices::PaymentId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_paid_invoices_deleted_at")
                    .table(PaidInvoices::Table)
                    .col(PaidInvoices::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_paid_invoices_posted_invoice_active")
                    .table(PaidInvoices::Table)
                    .col(PaidInvoices::PostedInvoiceId)
                    .unique()
                    .and_where(Expr::col(PaidInvoices::DeletedAt).is_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PaidInvoices::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PartiallyPaidInvoices::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Payments::Table).to_owned())
            .await
    }
}
