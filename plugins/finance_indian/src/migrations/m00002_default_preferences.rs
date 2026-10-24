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

#[derive(DeriveIden)]
enum ProductPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PaymentPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AccountingPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DefaultCurrencyId,
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

        let product_prefs = Query::insert()
            .into_table(ProductPreferences::Table)
            .columns([
                ProductPreferences::Id,
                ProductPreferences::CreatedAt,
                ProductPreferences::UpdatedAt,
            ])
            .values_panic([
                1.into(),
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
            ])
            .on_conflict(
                OnConflict::column(ProductPreferences::Id)
                    .update_columns([ProductPreferences::UpdatedAt])
                    .to_owned(),
            )
            .to_owned();
        conn.execute(&product_prefs).await?;

        let invoice_prefs = Query::insert()
            .into_table(InvoicePreferences::Table)
            .columns([
                InvoicePreferences::Id,
                InvoicePreferences::CreatedAt,
                InvoicePreferences::UpdatedAt,
            ])
            .values_panic([
                1.into(),
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
            ])
            .on_conflict(
                OnConflict::column(InvoicePreferences::Id)
                    .update_columns([InvoicePreferences::UpdatedAt])
                    .to_owned(),
            )
            .to_owned();
        conn.execute(&invoice_prefs).await?;

        let payment_prefs = Query::insert()
            .into_table(PaymentPreferences::Table)
            .columns([
                PaymentPreferences::Id,
                PaymentPreferences::CreatedAt,
                PaymentPreferences::UpdatedAt,
            ])
            .values_panic([
                1.into(),
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
            ])
            .on_conflict(
                OnConflict::column(PaymentPreferences::Id)
                    .update_columns([PaymentPreferences::UpdatedAt])
                    .to_owned(),
            )
            .to_owned();
        conn.execute(&payment_prefs).await?;

        // INR (ISO 4217 numeric code 356)
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
        conn.execute(&accounting_prefs).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
