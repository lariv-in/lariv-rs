use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Code,
    IsGroup,
    BalanceType,
    ParentId,
}

#[derive(DeriveIden)]
enum Taxes {
    Table,
    CreatedAt,
    UpdatedAt,
    Name,
    Percentage,
    TaxType,
    AccountId,
}

#[derive(DeriveIden)]
enum TaxKind {
    #[sea_orm(iden = "tax_kind")]
    Enum,
}

#[derive(DeriveIden)]
enum BalanceTypeEnum {
    #[sea_orm(iden = "balance_type")]
    Enum,
}

const GST_CHILD_ACCOUNTS: &[(&str, i32)] = &[
    ("CGST output payable", 20501),
    ("SGST and UTGST output payable", 20502),
    ("IGST output payable", 20503),
];

const GST_TAXES: &[(&str, &str, i32)] = &[
    ("CGST 2.5%", "2.5", 20501),
    ("SGST 2.5%", "2.5", 20502),
    ("IGST 5%", "5", 20503),
    ("CGST 6%", "6", 20501),
    ("SGST 6%", "6", 20502),
    ("IGST 12%", "12", 20503),
    ("CGST 9%", "9", 20501),
    ("SGST 9%", "9", 20502),
    ("IGST 18%", "18", 20503),
    ("CGST 14%", "14", 20501),
    ("SGST 14%", "14", 20502),
    ("IGST 28%", "28", 20503),
    ("CGST 1.5%", "1.5", 20501),
    ("SGST 1.5%", "1.5", 20502),
    ("IGST 3%", "3", 20503),
    ("CGST 0.125%", "0.125", 20501),
    ("SGST 0.125%", "0.125", 20502),
    ("IGST 0.25%", "0.25", 20503),
    ("CGST 0.75%", "0.75", 20501),
    ("SGST 0.75%", "0.75", 20502),
    ("IGST 1.5%", "1.5", 20503),
];

const GST_TAX_NAMES: &[&str] = &[
    "CGST 2.5%",
    "SGST 2.5%",
    "IGST 5%",
    "CGST 6%",
    "SGST 6%",
    "IGST 12%",
    "CGST 9%",
    "SGST 9%",
    "IGST 18%",
    "CGST 14%",
    "SGST 14%",
    "IGST 28%",
    "CGST 1.5%",
    "SGST 1.5%",
    "IGST 3%",
    "CGST 0.125%",
    "SGST 0.125%",
    "IGST 0.25%",
    "CGST 0.75%",
    "SGST 0.75%",
    "IGST 1.5%",
];

fn account_id_by_code(code: i32) -> SelectStatement {
    Query::select()
        .column(Accounts::Id)
        .from(Accounts::Table)
        .and_where(Expr::col(Accounts::Code).eq(code))
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

        let group_insert = Query::insert()
            .into_table(Accounts::Table)
            .columns([
                Accounts::CreatedAt,
                Accounts::UpdatedAt,
                Accounts::Name,
                Accounts::Code,
                Accounts::IsGroup,
                Accounts::BalanceType,
                Accounts::ParentId,
            ])
            .values_panic([
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
                "India GST output payable".into(),
                20500.into(),
                true.into(),
                Expr::val("Credit").cast_as(BalanceTypeEnum::Enum).into(),
                subquery_expr(account_id_by_code(20000)).into(),
            ])
            .to_owned();
        conn.execute(backend.build(&group_insert)).await?;

        for &(name, code) in GST_CHILD_ACCOUNTS {
            let insert = Query::insert()
                .into_table(Accounts::Table)
                .columns([
                    Accounts::CreatedAt,
                    Accounts::UpdatedAt,
                    Accounts::Name,
                    Accounts::Code,
                    Accounts::IsGroup,
                    Accounts::BalanceType,
                    Accounts::ParentId,
                ])
                .values_panic([
                    Expr::current_timestamp().into(),
                    Expr::current_timestamp().into(),
                    name.into(),
                    code.into(),
                    false.into(),
                    Expr::val("Credit").cast_as(BalanceTypeEnum::Enum).into(),
                    subquery_expr(account_id_by_code(20500)).into(),
                ])
                .to_owned();
            conn.execute(backend.build(&insert)).await?;
        }

        for &(name, pct, acct_code) in GST_TAXES {
            let insert = Query::insert()
                .into_table(Taxes::Table)
                .columns([
                    Taxes::CreatedAt,
                    Taxes::UpdatedAt,
                    Taxes::Name,
                    Taxes::Percentage,
                    Taxes::TaxType,
                    Taxes::AccountId,
                ])
                .values_panic([
                    Expr::current_timestamp().into(),
                    Expr::current_timestamp().into(),
                    name.into(),
                    Expr::val(pct).cast_as(Alias::new("numeric")).into(),
                    Expr::val("levied").cast_as(TaxKind::Enum).into(),
                    subquery_expr(account_id_by_code(acct_code)).into(),
                ])
                .to_owned();
            conn.execute(backend.build(&insert)).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let conn = manager.get_connection();

        let delete_taxes = Query::delete()
            .from_table(Taxes::Table)
            .cond_where(Expr::col(Taxes::Name).is_in(GST_TAX_NAMES.iter().copied()))
            .to_owned();
        conn.execute(backend.build(&delete_taxes)).await?;

        let delete_accounts = Query::delete()
            .from_table(Accounts::Table)
            .cond_where(Expr::col(Accounts::Code).is_in([20501, 20502, 20503, 20500]))
            .to_owned();
        conn.execute(backend.build(&delete_accounts)).await?;

        Ok(())
    }
}
