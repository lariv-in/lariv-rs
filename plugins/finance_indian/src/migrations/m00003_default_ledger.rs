use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Journals {
    Table,
    CreatedAt,
    UpdatedAt,
    Name,
    IsActive,
    CurrencyId,
}

#[derive(DeriveIden)]
enum Currencies {
    Table,
    Id,
    Code,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        let journal_insert = Query::insert()
            .into_table(Journals::Table)
            .columns([
                Journals::CreatedAt,
                Journals::UpdatedAt,
                Journals::Name,
                Journals::IsActive,
                Journals::CurrencyId,
            ])
            .select_from(
                Query::select()
                    .expr(Expr::current_timestamp())
                    .expr(Expr::current_timestamp())
                    .expr(Expr::val("General"))
                    .expr(Expr::val(true))
                    .expr(Expr::col((Currencies::Table, Currencies::Id)))
                    .from(Currencies::Table)
                    .and_where(Expr::col((Currencies::Table, Currencies::Code)).eq(356))
                    .and_where(
                        Expr::exists(
                            Query::select()
                                .expr(Expr::val(1))
                                .from(Journals::Table)
                                .to_owned(),
                        )
                        .not(),
                    )
                    .to_owned(),
            )
            .unwrap()
            .to_owned();
        conn.execute(&journal_insert).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
