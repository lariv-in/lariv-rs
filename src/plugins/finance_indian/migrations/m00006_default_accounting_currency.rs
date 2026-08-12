//! Set default accounting currency to INR for databases that already ran m00002.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum AccountingPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DefaultCurrencyId,
}

#[derive(DeriveIden)]
enum Currencies {
    Table,
    Id,
    Code,
}

fn currency_id_by_code(code: i32) -> SelectStatement {
    Query::select()
        .column(Currencies::Id)
        .from(Currencies::Table)
        .and_where(Expr::col(Currencies::Code).eq(code))
        .limit(1)
        .to_owned()
}

fn subquery_expr(sel: SelectStatement) -> SimpleExpr {
    SimpleExpr::SubQuery(None, Box::new(sel.into_sub_query_statement()))
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let conn = manager.get_connection();

        let accounting_prefs = Query::insert()
            .into_table(AccountingPreferences::Table)
            .columns([
                AccountingPreferences::Id,
                AccountingPreferences::CreatedAt,
                AccountingPreferences::UpdatedAt,
                AccountingPreferences::DefaultCurrencyId,
            ])
            .values_panic([
                1.into(),
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
                subquery_expr(currency_id_by_code(356)).into(),
            ])
            .on_conflict(
                OnConflict::column(AccountingPreferences::Id)
                    .update_columns([
                        AccountingPreferences::DefaultCurrencyId,
                        AccountingPreferences::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .to_owned();
        conn.execute(backend.build(&accounting_prefs)).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let conn = manager.get_connection();
        let update = Query::update()
            .table(AccountingPreferences::Table)
            .value(
                AccountingPreferences::DefaultCurrencyId,
                Expr::val(None::<i64>),
            )
            .and_where(Expr::col(AccountingPreferences::Id).eq(1))
            .to_owned();
        conn.execute(backend.build(&update)).await?;
        Ok(())
    }
}
