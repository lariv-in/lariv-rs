use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DraftInvoices {
    Table,
    AccountReceivableId,
    AccountRevenueId,
    JournalId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite cannot drop a column that an inline foreign key still names.
        exec_sql(
            manager,
            "ALTER TABLE draft_invoices DROP COLUMN account_receivable_id",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE draft_invoices DROP COLUMN account_revenue_id",
        )
        .await?;
        exec_sql(manager, "ALTER TABLE draft_invoices DROP COLUMN journal_id").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(
                        ColumnDef::new(DraftInvoices::AccountReceivableId)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(
                        ColumnDef::new(DraftInvoices::AccountRevenueId)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DraftInvoices::Table)
                    .add_column(
                        ColumnDef::new(DraftInvoices::JournalId)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
