//! Optional company tenancy on audit rows.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Logs {
    Table,
    CompanyId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Logs::Table)
                    .add_column(ColumnDef::new(Logs::CompanyId).big_integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_logs_company_id")
                    .table(Logs::Table)
                    .col(Logs::CompanyId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_logs_company_id").table(Logs::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Logs::Table)
                    .drop_column(Logs::CompanyId)
                    .to_owned(),
            )
            .await
    }
}
