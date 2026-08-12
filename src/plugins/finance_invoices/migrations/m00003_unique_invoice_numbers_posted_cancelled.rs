use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PostedInvoices {
    Table,
    Number,
    DeletedAt,
}

#[derive(DeriveIden)]
enum CancelledInvoices {
    Table,
    Number,
    DeletedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("uix_cancelled_invoices_number_live")
                    .table(CancelledInvoices::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("uix_posted_invoices_number_live")
                    .table(PostedInvoices::Table)
                    .to_owned(),
            )
            .await
    }
}
