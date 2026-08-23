use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

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
        execute(
            manager,
            "DELETE FROM fiscal_years WHERE deleted_at IS NOT NULL",
        )
        .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_fiscal_years_deleted_at")
                    .table(Alias::new("fiscal_years"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("fiscal_years"))
                    .drop_column(Alias::new("deleted_at"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("fiscal_years"))
                    .add_column(ColumnDef::new(Alias::new("deleted_at")).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_fiscal_years_deleted_at")
                    .table(Alias::new("fiscal_years"))
                    .col(Alias::new("deleted_at"))
                    .to_owned(),
            )
            .await
    }
}
