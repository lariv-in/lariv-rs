use lariv_core::db::migration_sql::is_postgres;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Products {
    Table,
    Reference,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            manager
                .alter_table(
                    Table::alter()
                        .table(Products::Table)
                        .modify_column(ColumnDef::new(Products::Reference).text().null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let update = Query::update()
            .table(Products::Table)
            .value(Products::Reference, "")
            .and_where(Expr::col(Products::Reference).is_null())
            .to_owned();
        manager.get_connection().execute(&update).await?;

        if is_postgres(manager) {
            manager
                .alter_table(
                    Table::alter()
                        .table(Products::Table)
                        .modify_column(ColumnDef::new(Products::Reference).text().not_null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
