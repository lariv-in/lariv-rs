use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Journals {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    IsActive,
    CurrencyId,
    JournalType,
}

#[derive(DeriveIden)]
enum JournalType {
    #[sea_orm(iden = "journal_type")]
    Enum,
}

#[derive(DeriveIden)]
enum Currencies {
    Table,
    Id,
    Code,
}

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    Id,
    JournalId,
    UpdatedAt,
}

fn subquery_expr(sel: SelectStatement) -> SimpleExpr {
    SimpleExpr::SubQuery(None, Box::new(sel.into_sub_query_statement()))
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let conn = manager.get_connection();

        let journal_insert = Query::insert()
            .into_table(Journals::Table)
            .columns([
                Journals::CreatedAt,
                Journals::UpdatedAt,
                Journals::Name,
                Journals::IsActive,
                Journals::CurrencyId,
                Journals::JournalType,
            ])
            .select_from(
                Query::select()
                    .expr(Expr::current_timestamp())
                    .expr(Expr::current_timestamp())
                    .expr(Expr::val("General"))
                    .expr(Expr::val(true))
                    .expr(Expr::col((Currencies::Table, Currencies::Id)))
                    .expr(Expr::val("Debit").cast_as(JournalType::Enum))
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
        conn.execute(backend.build(&journal_insert)).await?;

        let journal_id = Query::select()
            .column(Journals::Id)
            .from(Journals::Table)
            .order_by(Journals::Id, Order::Asc)
            .limit(1)
            .to_owned();

        let update = Query::update()
            .table(InvoicePreferences::Table)
            .values([
                (InvoicePreferences::JournalId, subquery_expr(journal_id)),
                (
                    InvoicePreferences::UpdatedAt,
                    Expr::current_timestamp().into(),
                ),
            ])
            .and_where(Expr::col(InvoicePreferences::Id).eq(1))
            .and_where(Expr::col(InvoicePreferences::JournalId).is_null())
            .to_owned();
        conn.execute(backend.build(&update)).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let update = Query::update()
            .table(InvoicePreferences::Table)
            .value(InvoicePreferences::JournalId, Expr::val(None::<i64>))
            .and_where(Expr::col(InvoicePreferences::Id).eq(1))
            .to_owned();
        manager
            .get_connection()
            .execute(backend.build(&update))
            .await?;
        Ok(())
    }
}
