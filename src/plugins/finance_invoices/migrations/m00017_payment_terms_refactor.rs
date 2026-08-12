use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DraftPaymentTerms {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DraftInvoiceId,
}

#[derive(DeriveIden)]
enum DraftPaymentTermLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DraftPaymentTermId,
    LineOrder,
    DateKind,
    DueDatetime,
    DueDuration,
    AmountKind,
    Amount,
    AmountPercentage,
}

#[derive(DeriveIden)]
enum PostedPaymentTerms {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    PostedInvoiceId,
    CancelledInvoiceId,
}

#[derive(DeriveIden)]
enum PostedPaymentTermLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    PostedPaymentTermId,
    LineOrder,
    DueDatetime,
    Amount,
}

#[derive(DeriveIden)]
enum DraftInvoices {
    Table,
    PaymentTermType,
    PaymentTermId,
}

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    PaymentTermType,
    PaymentTermId,
}

#[derive(DeriveIden)]
enum CancelledInvoices {
    Table,
    PaymentTermType,
    PaymentTermId,
}

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_unprepared(sql)
        .await
        .map(|_| ())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DraftPaymentTerms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DraftPaymentTerms::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTerms::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTerms::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTerms::DraftInvoiceId)
                            .big_integer()
                            .not_null()
                            .unique_key(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_payment_terms_draft_invoice_id")
                            .from(DraftPaymentTerms::Table, DraftPaymentTerms::DraftInvoiceId)
                            .to(DraftInvoices::Table, Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DraftPaymentTermLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::DraftPaymentTermId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::LineOrder)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::DateKind)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::DueDatetime)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::DueDuration).big_integer(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::AmountKind)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::Amount)
                            .decimal_len(19, 6),
                    )
                    .col(
                        ColumnDef::new(DraftPaymentTermLines::AmountPercentage)
                            .decimal_len(19, 6),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_draft_payment_term_lines_term_id")
                            .from(
                                DraftPaymentTermLines::Table,
                                DraftPaymentTermLines::DraftPaymentTermId,
                            )
                            .to(DraftPaymentTerms::Table, DraftPaymentTerms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PostedPaymentTerms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PostedPaymentTerms::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTerms::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTerms::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTerms::PostedInvoiceId)
                            .big_integer()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTerms::CancelledInvoiceId)
                            .big_integer()
                            .unique_key(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_payment_terms_posted_invoice_id")
                            .from(PostedPaymentTerms::Table, PostedPaymentTerms::PostedInvoiceId)
                            .to(PostedInvoices::Table, Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_payment_terms_cancelled_invoice_id")
                            .from(
                                PostedPaymentTerms::Table,
                                PostedPaymentTerms::CancelledInvoiceId,
                            )
                            .to(CancelledInvoices::Table, Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        execute(
            manager,
            "ALTER TABLE posted_payment_terms ADD CONSTRAINT chk_posted_payment_terms_one_owner \
             CHECK (\
               (CASE WHEN posted_invoice_id IS NOT NULL THEN 1 ELSE 0 END) + \
               (CASE WHEN cancelled_invoice_id IS NOT NULL THEN 1 ELSE 0 END) \
               <= 1\
             )",
        )
        .await?;

        manager
            .create_table(
                Table::create()
                    .table(PostedPaymentTermLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PostedPaymentTermLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTermLines::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTermLines::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTermLines::PostedPaymentTermId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTermLines::LineOrder)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTermLines::DueDatetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PostedPaymentTermLines::Amount)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posted_payment_term_lines_term_id")
                            .from(
                                PostedPaymentTermLines::Table,
                                PostedPaymentTermLines::PostedPaymentTermId,
                            )
                            .to(PostedPaymentTerms::Table, PostedPaymentTerms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Migrate legacy data via Rust helper (uses old tables before drop).
        crate::plugins::finance_invoices::logic::draft_payment_term::migrate_legacy_payment_terms(manager.get_connection())
            .await
            .map_err(DbErr::Custom)?;

        // Drop FK constraints referencing payment_terms
        execute(
            manager,
            "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS fk_draft_invoices_payment_term_id",
        )
        .await?;
        execute(
            manager,
            "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS fk_posted_invoices_payment_term_id",
        )
        .await?;
        execute(
            manager,
            "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS fk_cancelled_invoices_payment_term_id",
        )
        .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .drop_column(DraftInvoices::PaymentTermType)
                    .drop_column(DraftInvoices::PaymentTermId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .drop_column(PostedInvoices::PaymentTermType)
                    .drop_column(PostedInvoices::PaymentTermId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .drop_column(CancelledInvoices::PaymentTermType)
                    .drop_column(CancelledInvoices::PaymentTermId)
                    .to_owned(),
            )
            .await?;

        execute(manager, "DROP TABLE IF EXISTS payment_terms CASCADE").await?;
        execute(manager, "DROP TABLE IF EXISTS payment_term_due_dates CASCADE").await?;
        execute(manager, "DROP TABLE IF EXISTS payment_term_relatives CASCADE").await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "m00017 payment terms refactor cannot be reversed".into(),
        ))
    }
}
