use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PaidInvoices {
    Table,
    DeletedAt,
    PostedInvoiceId,
}

#[derive(DeriveIden)]
enum CancelledInvoices {
    Table,
    DeletedAt,
    PostedInvoiceId,
    Number,
}

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    DeletedAt,
    DraftInvoiceId,
    Number,
}

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_connection().get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Purge soft-deleted rows; delete children that reference soft-deleted parents first.
        execute(
            manager,
            r#"DELETE FROM paid_invoices
               WHERE deleted_at IS NOT NULL
                  OR posted_invoice_id IN (SELECT id FROM posted_invoices WHERE deleted_at IS NOT NULL)
                  OR payment_id IN (SELECT id FROM payments WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        execute(
            manager,
            r#"DELETE FROM partially_paid_invoices
               WHERE deleted_at IS NOT NULL
                  OR posted_invoice_id IN (SELECT id FROM posted_invoices WHERE deleted_at IS NOT NULL)
                  OR payment_id IN (SELECT id FROM payments WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        execute(
            manager,
            r#"DELETE FROM payments
               WHERE deleted_at IS NOT NULL
                  OR payment_batch_id IN (SELECT id FROM payment_batches WHERE deleted_at IS NOT NULL)
                  OR posted_invoice_id IN (SELECT id FROM posted_invoices WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        execute(
            manager,
            "DELETE FROM payment_batches WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            r#"DELETE FROM cancelled_invoice_lines
               WHERE deleted_at IS NOT NULL
                  OR cancelled_invoice_id IN (
                    SELECT id FROM cancelled_invoices WHERE deleted_at IS NOT NULL
                  )"#,
        )
        .await?;
        execute(
            manager,
            r#"DELETE FROM cancelled_invoices
               WHERE deleted_at IS NOT NULL
                  OR posted_invoice_id IN (SELECT id FROM posted_invoices WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        execute(
            manager,
            r#"DELETE FROM posted_invoice_lines
               WHERE deleted_at IS NOT NULL
                  OR posted_invoice_id IN (SELECT id FROM posted_invoices WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        execute(
            manager,
            r#"DELETE FROM posted_invoices
               WHERE deleted_at IS NOT NULL
                  OR draft_invoice_id IN (SELECT id FROM draft_invoices WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        execute(
            manager,
            r#"DELETE FROM draft_invoice_lines
               WHERE deleted_at IS NOT NULL
                  OR draft_invoice_id IN (SELECT id FROM draft_invoices WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        execute(
            manager,
            "DELETE FROM draft_invoices WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            "DELETE FROM payment_terms WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            "DELETE FROM payment_term_due_dates WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            "DELETE FROM payment_term_relatives WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            "DELETE FROM invoice_preferences WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            "DELETE FROM payment_preferences WHERE deleted_at IS NOT NULL",
        )
        .await?;

        // Drop partial uniques that depend on deleted_at before dropping the column.
        for (name, table) in [
            ("uix_posted_invoices_draft_invoice_id", "posted_invoices"),
            (
                "uix_cancelled_invoices_posted_invoice_id",
                "cancelled_invoices",
            ),
            ("uix_posted_invoices_number_live", "posted_invoices"),
            ("uix_cancelled_invoices_number_live", "cancelled_invoices"),
            ("uix_paid_invoices_posted_invoice_active", "paid_invoices"),
        ] {
            manager
                .drop_index(Index::drop().name(name).table(Alias::new(table)).to_owned())
                .await?;
        }

        for (index, table) in [
            ("idx_paid_invoices_deleted_at", "paid_invoices"),
            (
                "idx_partially_paid_invoices_deleted_at",
                "partially_paid_invoices",
            ),
            ("idx_payments_deleted_at", "payments"),
            (
                "idx_cancelled_invoice_lines_deleted_at",
                "cancelled_invoice_lines",
            ),
            ("idx_cancelled_invoices_deleted_at", "cancelled_invoices"),
            (
                "idx_posted_invoice_lines_deleted_at",
                "posted_invoice_lines",
            ),
            ("idx_posted_invoices_deleted_at", "posted_invoices"),
            ("idx_draft_invoice_lines_deleted_at", "draft_invoice_lines"),
            ("idx_draft_invoices_deleted_at", "draft_invoices"),
            ("idx_payment_terms_deleted_at", "payment_terms"),
            (
                "idx_payment_term_due_dates_deleted_at",
                "payment_term_due_dates",
            ),
            (
                "idx_payment_term_relatives_deleted_at",
                "payment_term_relatives",
            ),
            ("idx_invoice_preferences_deleted_at", "invoice_preferences"),
            ("idx_payment_preferences_deleted_at", "payment_preferences"),
        ] {
            manager
                .drop_index(
                    Index::drop()
                        .name(index)
                        .table(Alias::new(table))
                        .to_owned(),
                )
                .await?;
        }

        for table in [
            "paid_invoices",
            "partially_paid_invoices",
            "payments",
            "payment_batches",
            "cancelled_invoice_lines",
            "cancelled_invoices",
            "posted_invoice_lines",
            "posted_invoices",
            "draft_invoice_lines",
            "draft_invoices",
            "payment_terms",
            "payment_term_due_dates",
            "payment_term_relatives",
            "invoice_preferences",
            "payment_preferences",
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new(table))
                        .drop_column(Alias::new("deleted_at"))
                        .to_owned(),
                )
                .await?;
        }

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_posted_invoices_draft_invoice_id")
                    .table(PostedInvoices::Table)
                    .col(PostedInvoices::DraftInvoiceId)
                    .unique()
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
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_posted_invoices_number")
                    .table(PostedInvoices::Table)
                    .col(PostedInvoices::Number)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_cancelled_invoices_number")
                    .table(CancelledInvoices::Table)
                    .col(CancelledInvoices::Number)
                    .unique()
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
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (name, table) in [
            ("uix_paid_invoices_posted_invoice_active", "paid_invoices"),
            ("uix_cancelled_invoices_number", "cancelled_invoices"),
            ("uix_posted_invoices_number", "posted_invoices"),
            (
                "uix_cancelled_invoices_posted_invoice_id",
                "cancelled_invoices",
            ),
            ("uix_posted_invoices_draft_invoice_id", "posted_invoices"),
        ] {
            manager
                .drop_index(Index::drop().name(name).table(Alias::new(table)).to_owned())
                .await?;
        }

        for table in [
            "payment_preferences",
            "invoice_preferences",
            "payment_term_relatives",
            "payment_term_due_dates",
            "payment_terms",
            "draft_invoices",
            "draft_invoice_lines",
            "posted_invoices",
            "posted_invoice_lines",
            "cancelled_invoices",
            "cancelled_invoice_lines",
            "payment_batches",
            "payments",
            "partially_paid_invoices",
            "paid_invoices",
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new(table))
                        .add_column(
                            ColumnDef::new(Alias::new("deleted_at")).timestamp_with_time_zone(),
                        )
                        .to_owned(),
                )
                .await?;
        }

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
                    .name("uix_posted_invoices_number_live")
                    .table(PostedInvoices::Table)
                    .col(PostedInvoices::Number)
                    .unique()
                    .and_where(Expr::col(PostedInvoices::DeletedAt).is_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_cancelled_invoices_number_live")
                    .table(CancelledInvoices::Table)
                    .col(CancelledInvoices::Number)
                    .unique()
                    .and_where(Expr::col(CancelledInvoices::DeletedAt).is_null())
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
            .await?;

        Ok(())
    }
}
