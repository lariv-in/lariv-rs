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
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .modify_column(ColumnDef::new(Products::Reference).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let update = Query::update()
            .table(Products::Table)
            .value(Products::Reference, "")
            .and_where(Expr::col(Products::Reference).is_null())
            .to_owned();
        manager
            .get_connection()
            .execute(backend.build(&update))
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .modify_column(ColumnDef::new(Products::Reference).text().not_null())
                    .to_owned(),
            )
            .await
    }
}
