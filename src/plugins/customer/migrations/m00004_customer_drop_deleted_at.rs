use crate::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager,
            "DELETE FROM customers WHERE deleted_at IS NOT NULL",
        )
        .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_customers_deleted_at")
                    .table(Alias::new("customers"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("customers"))
                    .drop_column(Alias::new("deleted_at"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("customers"))
                    .add_column(ColumnDef::new(Alias::new("deleted_at")).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_customers_deleted_at")
                    .table(Alias::new("customers"))
                    .col(Alias::new("deleted_at"))
                    .to_owned(),
            )
            .await
    }
}
