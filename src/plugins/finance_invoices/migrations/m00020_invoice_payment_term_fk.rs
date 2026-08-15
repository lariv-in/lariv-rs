//! Invert payment-term ownership: invoices point at terms.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DraftInvoices {
    Table,
    DraftPaymentTermId,
}

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    PostedPaymentTermId,
}

#[derive(DeriveIden)]
enum CancelledInvoices {
    Table,
    PostedPaymentTermId,
}

#[derive(DeriveIden)]
enum DraftPaymentTerms {
    Table,
    Id,
    DraftInvoiceId,
}

#[derive(DeriveIden)]
enum PostedPaymentTerms {
    Table,
    Id,
    PostedInvoiceId,
    CancelledInvoiceId,
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
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(
                        ColumnDef::new(DraftInvoices::DraftPaymentTermId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .add_column(
                        ColumnDef::new(PostedInvoices::PostedPaymentTermId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .add_column(
                        ColumnDef::new(CancelledInvoices::PostedPaymentTermId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        execute(
            manager,
            "UPDATE draft_invoices d SET draft_payment_term_id = t.id \
             FROM draft_payment_terms t WHERE t.draft_invoice_id = d.id",
        )
        .await?;
        execute(
            manager,
            "UPDATE posted_invoices p SET posted_payment_term_id = t.id \
             FROM posted_payment_terms t WHERE t.posted_invoice_id = p.id",
        )
        .await?;
        execute(
            manager,
            "UPDATE cancelled_invoices c SET posted_payment_term_id = t.id \
             FROM posted_payment_terms t WHERE t.cancelled_invoice_id = c.id",
        )
        .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("uq_draft_invoices_draft_payment_term_id")
                    .table(DraftInvoices::Table)
                    .col(DraftInvoices::DraftPaymentTermId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("uq_posted_invoices_posted_payment_term_id")
                    .table(PostedInvoices::Table)
                    .col(PostedInvoices::PostedPaymentTermId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("uq_cancelled_invoices_posted_payment_term_id")
                    .table(CancelledInvoices::Table)
                    .col(CancelledInvoices::PostedPaymentTermId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_draft_invoices_draft_payment_term_id")
                            .from_tbl(DraftInvoices::Table)
                            .from_col(DraftInvoices::DraftPaymentTermId)
                            .to_tbl(DraftPaymentTerms::Table)
                            .to_col(DraftPaymentTerms::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(PostedInvoices::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_posted_invoices_posted_payment_term_id")
                            .from_tbl(PostedInvoices::Table)
                            .from_col(PostedInvoices::PostedPaymentTermId)
                            .to_tbl(PostedPaymentTerms::Table)
                            .to_col(PostedPaymentTerms::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CancelledInvoices::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_cancelled_invoices_posted_payment_term_id")
                            .from_tbl(CancelledInvoices::Table)
                            .from_col(CancelledInvoices::PostedPaymentTermId)
                            .to_tbl(PostedPaymentTerms::Table)
                            .to_col(PostedPaymentTerms::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_draft_payment_terms_draft_invoice_id")
                    .table(DraftPaymentTerms::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_posted_payment_terms_posted_invoice_id")
                    .table(PostedPaymentTerms::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_posted_payment_terms_cancelled_invoice_id")
                    .table(PostedPaymentTerms::Table)
                    .to_owned(),
            )
            .await?;
        execute(
            manager,
            "ALTER TABLE posted_payment_terms DROP CONSTRAINT IF EXISTS chk_posted_payment_terms_one_owner",
        )
        .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DraftPaymentTerms::Table)
                    .drop_column(DraftPaymentTerms::DraftInvoiceId)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(PostedPaymentTerms::Table)
                    .drop_column(PostedPaymentTerms::PostedInvoiceId)
                    .drop_column(PostedPaymentTerms::CancelledInvoiceId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "m00020 invoice payment term fk cannot be reversed".into(),
        ))
    }
}
