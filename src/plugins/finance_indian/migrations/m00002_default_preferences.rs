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
enum Accounts {
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
    InventoryAccountId,
    CostOfSalesAccountId,
}

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    AccountReceivableId,
    AccountRevenueId,
    AccountTaxPayableId,
    JournalId,
}

#[derive(DeriveIden)]
enum PaymentPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    PaymentAccountId,
}

#[derive(DeriveIden)]
enum AccountingPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DefaultCurrencyId,
}

fn account_id_by_code(code: i32) -> SelectStatement {
    Query::select()
        .column(Accounts::Id)
        .from(Accounts::Table)
        .and_where(Expr::col(Accounts::Code).eq(code))
        .limit(1)
        .to_owned()
}

fn default_journal_id() -> SelectStatement {
    Query::select()
        .column(Journals::Id)
        .from(Journals::Table)
        .limit(1)
        .to_owned()
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

        let product_prefs = Query::insert()
            .into_table(ProductPreferences::Table)
            .columns([
                ProductPreferences::Id,
                ProductPreferences::CreatedAt,
                ProductPreferences::UpdatedAt,
                ProductPreferences::InventoryAccountId,
                ProductPreferences::CostOfSalesAccountId,
            ])
            .values_panic([
                1.into(),
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
                subquery_expr(account_id_by_code(10301)).into(),
                subquery_expr(account_id_by_code(50201)).into(),
            ])
            .on_conflict(
                OnConflict::column(ProductPreferences::Id)
                    .update_columns([
                        ProductPreferences::InventoryAccountId,
                        ProductPreferences::CostOfSalesAccountId,
                        ProductPreferences::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .to_owned();
        conn.execute(backend.build(&product_prefs)).await?;

        let invoice_prefs = Query::insert()
            .into_table(InvoicePreferences::Table)
            .columns([
                InvoicePreferences::Id,
                InvoicePreferences::CreatedAt,
                InvoicePreferences::UpdatedAt,
                InvoicePreferences::AccountReceivableId,
                InvoicePreferences::AccountRevenueId,
                InvoicePreferences::AccountTaxPayableId,
                InvoicePreferences::JournalId,
            ])
            .values_panic([
                1.into(),
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
                subquery_expr(account_id_by_code(10201)).into(),
                subquery_expr(account_id_by_code(40101)).into(),
                subquery_expr(account_id_by_code(20203)).into(),
                subquery_expr(default_journal_id()).into(),
            ])
            .on_conflict(
                OnConflict::column(InvoicePreferences::Id)
                    .update_columns([
                        InvoicePreferences::AccountReceivableId,
                        InvoicePreferences::AccountRevenueId,
                        InvoicePreferences::AccountTaxPayableId,
                        InvoicePreferences::JournalId,
                        InvoicePreferences::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .to_owned();
        conn.execute(backend.build(&invoice_prefs)).await?;

        let payment_prefs = Query::insert()
            .into_table(PaymentPreferences::Table)
            .columns([
                PaymentPreferences::Id,
                PaymentPreferences::CreatedAt,
                PaymentPreferences::UpdatedAt,
                PaymentPreferences::PaymentAccountId,
            ])
            .values_panic([
                1.into(),
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
                subquery_expr(account_id_by_code(10101)).into(),
            ])
            .on_conflict(
                OnConflict::column(PaymentPreferences::Id)
                    .update_columns([
                        PaymentPreferences::PaymentAccountId,
                        PaymentPreferences::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .to_owned();
        conn.execute(backend.build(&payment_prefs)).await?;

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
        conn.execute(backend.build(&accounting_prefs)).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let conn = manager.get_connection();

        let update_product = Query::update()
            .table(ProductPreferences::Table)
            .value(
                ProductPreferences::InventoryAccountId,
                Expr::val(None::<i64>),
            )
            .value(
                ProductPreferences::CostOfSalesAccountId,
                Expr::val(None::<i64>),
            )
            .and_where(Expr::col(ProductPreferences::Id).eq(1))
            .to_owned();
        conn.execute(backend.build(&update_product)).await?;

        let update_invoice = Query::update()
            .table(InvoicePreferences::Table)
            .values([
                (
                    InvoicePreferences::AccountReceivableId,
                    Expr::val(None::<i64>).into(),
                ),
                (
                    InvoicePreferences::AccountRevenueId,
                    Expr::val(None::<i64>).into(),
                ),
                (
                    InvoicePreferences::AccountTaxPayableId,
                    Expr::val(None::<i64>).into(),
                ),
                (InvoicePreferences::JournalId, Expr::val(None::<i64>).into()),
            ])
            .and_where(Expr::col(InvoicePreferences::Id).eq(1))
            .to_owned();
        conn.execute(backend.build(&update_invoice)).await?;

        let update_payment = Query::update()
            .table(PaymentPreferences::Table)
            .value(PaymentPreferences::PaymentAccountId, Expr::val(None::<i64>))
            .and_where(Expr::col(PaymentPreferences::Id).eq(1))
            .to_owned();
        conn.execute(backend.build(&update_payment)).await?;

        Ok(())
    }
}
