use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Taxes {
    Table,
    CreatedAt,
    UpdatedAt,
    Name,
    Percentage,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let insert = Query::insert()
            .into_table(Taxes::Table)
            .columns([
                Taxes::CreatedAt,
                Taxes::UpdatedAt,
                Taxes::Name,
                Taxes::Percentage,
            ])
            .values_panic([
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
                "Service Tax".into(),
                18.into(),
            ])
            .to_owned();
        manager
            .get_connection()
            .execute(backend.build(&insert))
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let delete = Query::delete()
            .from_table(Taxes::Table)
            .cond_where(
                Condition::all()
                    .add(Expr::col(Taxes::Name).eq("Service Tax"))
                    .add(Expr::col(Taxes::Percentage).eq(18)),
            )
            .to_owned();
        manager
            .get_connection()
            .execute(backend.build(&delete))
            .await
            .map(|_| ())
    }
}
