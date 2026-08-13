use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PaymentTerms {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Type,
    BackingId,
}

#[derive(DeriveIden)]
enum DraftInvoices {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Number,
    AccountReceivableId,
    AccountRevenueId,
    AccountTaxPayableId,
    JournalId,
    Datetime,
    CustomerId,
    PaymentTermType,
    PaymentTermId,
}

#[derive(DeriveIden)]
enum DraftInvoiceTaxes {
    Table,
    DraftInvoiceId,
    TaxId,
}

#[derive(DeriveIden)]
enum DraftInvoiceLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    DraftInvoiceId,
    ProductId,
    Rate,
    Quantity,
}

#[derive(DeriveIden)]
enum DraftInvoiceLineTaxes {
    Table,
    DraftInvoiceLineId,
    TaxId,
}

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    DraftInvoiceId,
    PostedAt,
    Number,
    AccountReceivableId,
    AccountRevenueId,
    AccountTaxPayableId,
    JournalId,
    Datetime,
    CustomerId,
    PaymentTermType,
    PaymentTermId,
    JournalEntryId,
}

#[derive(DeriveIden)]
enum PostedInvoiceTaxes {
    Table,
    PostedInvoiceId,
    TaxId,
}

#[derive(DeriveIden)]
enum PostedInvoiceLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    PostedInvoiceId,
    ProductId,
    Rate,
    Quantity,
    JournalEntryItemId,
}

#[derive(DeriveIden)]
enum PostedInvoiceLineTaxes {
    Table,
    PostedInvoiceLineId,
    TaxId,
}

#[derive(DeriveIden)]
enum CancelledInvoices {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    PostedInvoiceId,
    PostedAt,
    CancelledAt,
    Number,
    AccountReceivableId,
    AccountRevenueId,
    AccountTaxPayableId,
    JournalId,
    Datetime,
    CustomerId,
    PaymentTermType,
    PaymentTermId,
    CreditNoteId,
}

#[derive(DeriveIden)]
enum CancelledInvoiceTaxes {
    Table,
    CancelledInvoiceId,
    TaxId,
}

#[derive(DeriveIden)]
enum CancelledInvoiceLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    CancelledInvoiceId,
    ProductId,
    Rate,
    Quantity,
    JournalEntryItemId,
}

#[derive(DeriveIden)]
enum CancelledInvoiceLineTaxes {
    Table,
    CancelledInvoiceLineId,
    TaxId,
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

#[derive(DeriveIden)]
enum Customers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Taxes {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum JournalEntries {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum JournalEntryItems {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum CreditNotes {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentTerms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PaymentTerms::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PaymentTerms::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentTerms::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentTerms::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentTerms::Type).text().not_null())
                    .col(
                        ColumnDef::new(PaymentTerms::BackingId)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_payment_terms_deleted_at")
                    .table(PaymentTerms::Table)
                    .col(PaymentTerms::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DraftInvoices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DraftInvoices::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DraftInvoices::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DraftInvoices::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DraftInvoices::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DraftInvoices::Number).text())
                    .col(
                        ColumnDef::new(DraftInvoices::AccountReceivableId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoices::AccountRevenueId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoices::AccountTaxPayableId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoices::JournalId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoices::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoices::CustomerId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoices::PaymentTermType)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoices::PaymentTermId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoices_account_receivable_id")
                            .from(DraftInvoices::Table, DraftInvoices::AccountReceivableId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoices_account_revenue_id")
                            .from(DraftInvoices::Table, DraftInvoices::AccountRevenueId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoices_account_tax_payable_id")
                            .from(DraftInvoices::Table, DraftInvoices::AccountTaxPayableId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoices_journal_id")
                            .from(DraftInvoices::Table, DraftInvoices::JournalId)
                            .to(Journals::Table, Journals::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoices_customer_id")
                            .from(DraftInvoices::Table, DraftInvoices::CustomerId)
                            .to(Customers::Table, Customers::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoices_payment_term_id")
                            .from(DraftInvoices::Table, DraftInvoices::PaymentTermId)
                            .to(PaymentTerms::Table, PaymentTerms::Id)
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
                    .name("idx_draft_invoices_deleted_at")
                    .table(DraftInvoices::Table)
                    .col(DraftInvoices::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_draft_invoices_customer_id")
                    .table(DraftInvoices::Table)
                    .col(DraftInvoices::CustomerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_draft_invoices_datetime")
                    .table(DraftInvoices::Table)
                    .col(DraftInvoices::Datetime)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DraftInvoiceTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DraftInvoiceTaxes::DraftInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoiceTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(DraftInvoiceTaxes::DraftInvoiceId)
                            .col(DraftInvoiceTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoice_taxes_draft_invoice_id")
                            .from(DraftInvoiceTaxes::Table, DraftInvoiceTaxes::DraftInvoiceId)
                            .to(DraftInvoices::Table, DraftInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoice_taxes_tax_id")
                            .from(DraftInvoiceTaxes::Table, DraftInvoiceTaxes::TaxId)
                            .to(Taxes::Table, Taxes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DraftInvoiceLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DraftInvoiceLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DraftInvoiceLines::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DraftInvoiceLines::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DraftInvoiceLines::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(DraftInvoiceLines::DraftInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoiceLines::ProductId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoiceLines::Rate)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoiceLines::Quantity)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoice_lines_draft_invoice_id")
                            .from(DraftInvoiceLines::Table, DraftInvoiceLines::DraftInvoiceId)
                            .to(DraftInvoices::Table, DraftInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoice_lines_product_id")
                            .from(DraftInvoiceLines::Table, DraftInvoiceLines::ProductId)
                            .to(Products::Table, Products::Id)
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
                    .name("idx_draft_invoice_lines_deleted_at")
                    .table(DraftInvoiceLines::Table)
                    .col(DraftInvoiceLines::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_draft_invoice_lines_draft_invoice_id")
                    .table(DraftInvoiceLines::Table)
                    .col(DraftInvoiceLines::DraftInvoiceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DraftInvoiceLineTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DraftInvoiceLineTaxes::DraftInvoiceLineId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftInvoiceLineTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(DraftInvoiceLineTaxes::DraftInvoiceLineId)
                            .col(DraftInvoiceLineTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoice_line_taxes_draft_invoice_line_id")
                            .from(
                                DraftInvoiceLineTaxes::Table,
                                DraftInvoiceLineTaxes::DraftInvoiceLineId,
                            )
                            .to(DraftInvoiceLines::Table, DraftInvoiceLines::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_invoice_line_taxes_tax_id")
                            .from(DraftInvoiceLineTaxes::Table, DraftInvoiceLineTaxes::TaxId)
                            .to(Taxes::Table, Taxes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PostedInvoices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PostedInvoices::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PostedInvoices::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PostedInvoices::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PostedInvoices::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(PostedInvoices::DraftInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PostedInvoices::PostedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PostedInvoices::Number).text().not_null())
                    .col(
                        ColumnDef::new(PostedInvoices::AccountReceivableId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoices::AccountRevenueId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoices::AccountTaxPayableId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoices::JournalId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoices::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoices::CustomerId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoices::PaymentTermType)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoices::PaymentTermId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoices::JournalEntryId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoices_draft_invoice_id")
                            .from(PostedInvoices::Table, PostedInvoices::DraftInvoiceId)
                            .to(DraftInvoices::Table, DraftInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoices_account_receivable_id")
                            .from(PostedInvoices::Table, PostedInvoices::AccountReceivableId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoices_account_revenue_id")
                            .from(PostedInvoices::Table, PostedInvoices::AccountRevenueId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoices_account_tax_payable_id")
                            .from(PostedInvoices::Table, PostedInvoices::AccountTaxPayableId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoices_journal_id")
                            .from(PostedInvoices::Table, PostedInvoices::JournalId)
                            .to(Journals::Table, Journals::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoices_customer_id")
                            .from(PostedInvoices::Table, PostedInvoices::CustomerId)
                            .to(Customers::Table, Customers::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoices_payment_term_id")
                            .from(PostedInvoices::Table, PostedInvoices::PaymentTermId)
                            .to(PaymentTerms::Table, PaymentTerms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoices_journal_entry_id")
                            .from(PostedInvoices::Table, PostedInvoices::JournalEntryId)
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
                    .name("uix_posted_invoices_draft_invoice_id")
                    .table(PostedInvoices::Table)
                    .col(PostedInvoices::DraftInvoiceId)
                    .unique()
                    .and_where(Expr::col(PostedInvoices::DeletedAt).is_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_posted_invoices_deleted_at")
                    .table(PostedInvoices::Table)
                    .col(PostedInvoices::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_posted_invoices_journal_entry_id")
                    .table(PostedInvoices::Table)
                    .col(PostedInvoices::JournalEntryId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PostedInvoiceTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PostedInvoiceTaxes::PostedInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoiceTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(PostedInvoiceTaxes::PostedInvoiceId)
                            .col(PostedInvoiceTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoice_taxes_posted_invoice_id")
                            .from(
                                PostedInvoiceTaxes::Table,
                                PostedInvoiceTaxes::PostedInvoiceId,
                            )
                            .to(PostedInvoices::Table, PostedInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoice_taxes_tax_id")
                            .from(PostedInvoiceTaxes::Table, PostedInvoiceTaxes::TaxId)
                            .to(Taxes::Table, Taxes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PostedInvoiceLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PostedInvoiceLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PostedInvoiceLines::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PostedInvoiceLines::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PostedInvoiceLines::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(PostedInvoiceLines::PostedInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoiceLines::ProductId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoiceLines::Rate)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoiceLines::Quantity)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoiceLines::JournalEntryItemId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoice_lines_posted_invoice_id")
                            .from(
                                PostedInvoiceLines::Table,
                                PostedInvoiceLines::PostedInvoiceId,
                            )
                            .to(PostedInvoices::Table, PostedInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoice_lines_product_id")
                            .from(PostedInvoiceLines::Table, PostedInvoiceLines::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoice_lines_journal_entry_item_id")
                            .from(
                                PostedInvoiceLines::Table,
                                PostedInvoiceLines::JournalEntryItemId,
                            )
                            .to(JournalEntryItems::Table, JournalEntryItems::Id)
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
                    .name("idx_posted_invoice_lines_deleted_at")
                    .table(PostedInvoiceLines::Table)
                    .col(PostedInvoiceLines::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_posted_invoice_lines_posted_invoice_id")
                    .table(PostedInvoiceLines::Table)
                    .col(PostedInvoiceLines::PostedInvoiceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PostedInvoiceLineTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PostedInvoiceLineTaxes::PostedInvoiceLineId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedInvoiceLineTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(PostedInvoiceLineTaxes::PostedInvoiceLineId)
                            .col(PostedInvoiceLineTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoice_line_taxes_posted_invoice_line_id")
                            .from(
                                PostedInvoiceLineTaxes::Table,
                                PostedInvoiceLineTaxes::PostedInvoiceLineId,
                            )
                            .to(PostedInvoiceLines::Table, PostedInvoiceLines::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_invoice_line_taxes_tax_id")
                            .from(PostedInvoiceLineTaxes::Table, PostedInvoiceLineTaxes::TaxId)
                            .to(Taxes::Table, Taxes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CancelledInvoices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CancelledInvoices::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CancelledInvoices::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CancelledInvoices::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CancelledInvoices::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(CancelledInvoices::PostedInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CancelledInvoices::PostedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CancelledInvoices::CancelledAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CancelledInvoices::Number).text().not_null())
                    .col(
                        ColumnDef::new(CancelledInvoices::AccountReceivableId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoices::AccountRevenueId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoices::AccountTaxPayableId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoices::JournalId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoices::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoices::CustomerId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoices::PaymentTermType)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoices::PaymentTermId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoices::CreditNoteId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoices_posted_invoice_id")
                            .from(CancelledInvoices::Table, CancelledInvoices::PostedInvoiceId)
                            .to(PostedInvoices::Table, PostedInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoices_account_receivable_id")
                            .from(
                                CancelledInvoices::Table,
                                CancelledInvoices::AccountReceivableId,
                            )
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoices_account_revenue_id")
                            .from(
                                CancelledInvoices::Table,
                                CancelledInvoices::AccountRevenueId,
                            )
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoices_account_tax_payable_id")
                            .from(
                                CancelledInvoices::Table,
                                CancelledInvoices::AccountTaxPayableId,
                            )
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoices_journal_id")
                            .from(CancelledInvoices::Table, CancelledInvoices::JournalId)
                            .to(Journals::Table, Journals::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoices_customer_id")
                            .from(CancelledInvoices::Table, CancelledInvoices::CustomerId)
                            .to(Customers::Table, Customers::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoices_payment_term_id")
                            .from(CancelledInvoices::Table, CancelledInvoices::PaymentTermId)
                            .to(PaymentTerms::Table, PaymentTerms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoices_credit_note_id")
                            .from(CancelledInvoices::Table, CancelledInvoices::CreditNoteId)
                            .to(CreditNotes::Table, CreditNotes::Id)
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
                    .name("uix_cancelled_invoices_posted_invoice_id")
                    .table(CancelledInvoices::Table)
                    .col(CancelledInvoices::PostedInvoiceId)
                    .unique()
                    .and_where(Expr::col(CancelledInvoices::DeletedAt).is_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_cancelled_invoices_deleted_at")
                    .table(CancelledInvoices::Table)
                    .col(CancelledInvoices::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_cancelled_invoices_credit_note_id")
                    .table(CancelledInvoices::Table)
                    .col(CancelledInvoices::CreditNoteId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CancelledInvoiceTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CancelledInvoiceTaxes::CancelledInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(CancelledInvoiceTaxes::CancelledInvoiceId)
                            .col(CancelledInvoiceTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoice_taxes_cancelled_invoice_id")
                            .from(
                                CancelledInvoiceTaxes::Table,
                                CancelledInvoiceTaxes::CancelledInvoiceId,
                            )
                            .to(CancelledInvoices::Table, CancelledInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoice_taxes_tax_id")
                            .from(CancelledInvoiceTaxes::Table, CancelledInvoiceTaxes::TaxId)
                            .to(Taxes::Table, Taxes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CancelledInvoiceLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::DeletedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::CancelledInvoiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::ProductId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::Rate)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::Quantity)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLines::JournalEntryItemId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoice_lines_cancelled_invoice_id")
                            .from(
                                CancelledInvoiceLines::Table,
                                CancelledInvoiceLines::CancelledInvoiceId,
                            )
                            .to(CancelledInvoices::Table, CancelledInvoices::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoice_lines_product_id")
                            .from(
                                CancelledInvoiceLines::Table,
                                CancelledInvoiceLines::ProductId,
                            )
                            .to(Products::Table, Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoice_lines_journal_entry_item_id")
                            .from(
                                CancelledInvoiceLines::Table,
                                CancelledInvoiceLines::JournalEntryItemId,
                            )
                            .to(JournalEntryItems::Table, JournalEntryItems::Id)
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
                    .name("idx_cancelled_invoice_lines_deleted_at")
                    .table(CancelledInvoiceLines::Table)
                    .col(CancelledInvoiceLines::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_cancelled_invoice_lines_cancelled_invoice_id")
                    .table(CancelledInvoiceLines::Table)
                    .col(CancelledInvoiceLines::CancelledInvoiceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CancelledInvoiceLineTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CancelledInvoiceLineTaxes::CancelledInvoiceLineId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CancelledInvoiceLineTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(CancelledInvoiceLineTaxes::CancelledInvoiceLineId)
                            .col(CancelledInvoiceLineTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoice_line_taxes_cancelled_invoice_line_id")
                            .from(
                                CancelledInvoiceLineTaxes::Table,
                                CancelledInvoiceLineTaxes::CancelledInvoiceLineId,
                            )
                            .to(CancelledInvoiceLines::Table, CancelledInvoiceLines::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancelled_invoice_line_taxes_tax_id")
                            .from(
                                CancelledInvoiceLineTaxes::Table,
                                CancelledInvoiceLineTaxes::TaxId,
                            )
                            .to(Taxes::Table, Taxes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
