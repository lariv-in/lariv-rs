use sea_orm::Statement;
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
enum BalanceTypeEnum {
    #[sea_orm(iden = "balance_type")]
    Enum,
}

const SEED_ACCOUNTS: &[(&str, i32, bool, &str)] = &[
    ("Assets", 10000, true, "Debit"),
    ("Cash And Financial Assets", 10100, true, "Debit"),
    ("Cash and Cash Equivalents", 10101, false, "Debit"),
    ("Financial Assets (Investments)", 10102, false, "Debit"),
    (
        "Restricted Cash and Financial Assets",
        10103,
        false,
        "Debit",
    ),
    (
        "Additional Financial Assets and Investments",
        10104,
        false,
        "Debit",
    ),
    ("Receivables And Contracts", 10200, true, "Debit"),
    (
        "Accounts, Notes And Loans Receivable",
        10201,
        false,
        "Debit",
    ),
    ("Contracts", 10202, false, "Debit"),
    ("Nontrade And Other Receivables", 10203, false, "Debit"),
    ("Inventory", 10300, true, "Debit"),
    ("Merchandise", 10301, false, "Debit"),
    ("Raw Material, Parts And Supplies", 10302, false, "Debit"),
    ("Work In Process", 10303, false, "Debit"),
    ("Finished Goods", 10304, false, "Debit"),
    ("Other Inventory", 10305, false, "Debit"),
    ("Accruals And Additional Assets", 10400, true, "Debit"),
    ("Prepaid Expense", 10401, false, "Debit"),
    ("Accrued Income", 10402, false, "Debit"),
    ("Additional Assets", 10403, false, "Debit"),
    ("Property, Plant And Equipment", 10500, true, "Debit"),
    ("Land And Land Improvements", 10501, false, "Debit"),
    (
        "Buildings, Structures And Improvements",
        10502,
        false,
        "Debit",
    ),
    ("Machinery And Equipment", 10503, false, "Debit"),
    ("Furniture And Fixtures", 10504, false, "Debit"),
    (
        "Right Of Use Assets (Classified As PP&E)",
        10505,
        false,
        "Debit",
    ),
    ("Other Property, Plant And Equipment", 10506, false, "Debit"),
    ("Construction In Progress", 10507, false, "Debit"),
    (
        "Property, Plant And Equipment Accumulated Depreciation And Depletion",
        10600,
        true,
        "Credit",
    ),
    ("Accumulated Depletion", 10601, false, "Credit"),
    ("Accumulated Depreciation", 10602, false, "Credit"),
    (
        "Intangible Assets (Excluding Goodwill)",
        10700,
        true,
        "Debit",
    ),
    ("Intellectual Property", 10701, false, "Debit"),
    ("Computer Software", 10702, false, "Debit"),
    ("Trade And Distribution Assets", 10703, false, "Debit"),
    ("Contracts And Rights", 10704, false, "Debit"),
    ("Right Of Use Assets", 10705, false, "Debit"),
    ("Crypto Assets", 10706, false, "Debit"),
    ("Other Intangible Assets", 10707, false, "Debit"),
    ("Acquisition In Progress", 10708, false, "Debit"),
    (
        "Intangible Assets Accumulated Amortization",
        10800,
        false,
        "Credit",
    ),
    ("Goodwill", 10900, false, "Debit"),
    ("Liabilities", 20000, true, "Credit"),
    ("Payables", 20100, true, "Credit"),
    ("Trade Payables", 20101, false, "Credit"),
    ("Dividends Payable", 20102, false, "Credit"),
    ("Interest Payable", 20103, false, "Credit"),
    ("Other Payables", 20104, false, "Credit"),
    ("Accruals And Other Liabilities", 20200, true, "Credit"),
    (
        "Accrued Expenses (Including Payroll)",
        20201,
        false,
        "Credit",
    ),
    ("Deferred Income (Unearned Revenue)", 20202, false, "Credit"),
    ("Accrued Taxes (Other Than Payroll)", 20203, false, "Credit"),
    ("Other (Non-Financial) Liabilities", 20204, false, "Credit"),
    ("Financial Liabilities", 20300, true, "Credit"),
    ("Notes Payable", 20301, false, "Credit"),
    ("Loans Payable", 20302, false, "Credit"),
    ("Bonds (Debentures)", 20303, false, "Credit"),
    ("Other Debts And Borrowings", 20304, false, "Credit"),
    ("Lease Obligations", 20305, false, "Credit"),
    ("Derivative Financial Liabilities", 20306, false, "Credit"),
    ("Other Financial Liabilities", 20307, false, "Credit"),
    ("Provisions (Contingencies)", 20400, true, "Credit"),
    ("Customer Related Provisions", 20401, false, "Credit"),
    ("Ligation And Regulatory Provisions", 20402, false, "Credit"),
    ("Other Provisions", 20403, false, "Credit"),
    ("Equity", 30000, true, "Credit"),
    (
        "Owners Equity (Attributable To Owners Of Parent)",
        30100,
        true,
        "Credit",
    ),
    ("Equity At par (Issued Capital)", 30101, false, "Credit"),
    ("Additional Paid-in Capital", 30102, false, "Credit"),
    ("Retained Earnings", 30200, true, "Debit"),
    ("Appropriated", 30201, false, "Credit"),
    ("Unappropriated", 30202, false, "Credit"),
    ("Deficit", 30203, false, "Debit"),
    ("In Suspense Zero", 30204, false, "Debit"),
    ("Accumulated OCI", 30300, true, "Debit"),
    ("Exchange Differences On Translation", 30301, false, "Debit"),
    ("Cash Flow Hedges", 30302, false, "Debit"),
    (
        "Gains And Losses On Remeasuring Available-For-Sale Investments",
        30303,
        false,
        "Debit",
    ),
    (
        "Remeasurements Of Defined Benefit Plans",
        30304,
        false,
        "Debit",
    ),
    ("Revaluation Surplus (IFRS only)", 30305, false, "Credit"),
    ("Other Equity Items", 30400, true, "Debit"),
    ("ESOP Related Items", 30401, false, "Debit"),
    ("Subscribed Stock Receivables", 30402, false, "Debit"),
    ("Treasury Stock (Not Extinguished)", 30403, false, "Debit"),
    ("Miscellaneous Equity", 30404, false, "Credit"),
    ("Noncontrolling (Minority) Interest", 30500, false, "Credit"),
    ("Revenue", 40000, true, "Credit"),
    ("Recognized Point Of Time", 40100, true, "Credit"),
    ("Goods", 40101, false, "Credit"),
    ("Services", 40102, false, "Credit"),
    ("Recognized Over Time", 40200, true, "Credit"),
    ("Products", 40201, false, "Credit"),
    ("Services", 40202, false, "Credit"),
    ("Adjustments", 40300, true, "Debit"),
    ("Variable Consideration", 40301, false, "Debit"),
    (
        "Consideration Paid (Payable) To Customers",
        40302,
        false,
        "Debit",
    ),
    ("Other Adjustments", 40303, false, "Debit"),
    ("Expenses", 50000, true, "Debit"),
    ("Expenses Classified By Nature", 50100, true, "Debit"),
    (
        "Merchandise, Material, Parts And Supplies",
        50101,
        false,
        "Debit",
    ),
    ("Employee Benefits", 50102, false, "Debit"),
    ("Services", 50103, false, "Debit"),
    (
        "Rent, Depreciation, Amortization And Depletion",
        50104,
        false,
        "Debit",
    ),
    (
        "Increase (Decrease) In Inventories Of Finished Goods And Work In Progress",
        50105,
        false,
        "Debit",
    ),
    (
        "Other Work Performed By Entity And Capitalized",
        50106,
        false,
        "Credit",
    ),
    ("Expenses Classified By Function", 50200, true, "Debit"),
    ("Cost Of Sales", 50201, false, "Debit"),
    ("Selling, General And Administrative", 50202, false, "Debit"),
    (
        "Credit Loss (Reversal) On Receivables",
        50203,
        false,
        "Debit",
    ),
    (
        "Other (Non-Operating) Income And Expenses",
        60000,
        true,
        "Debit",
    ),
    ("Other Revenue And Expenses", 60100, true, "Debit"),
    ("Other Revenue", 60101, false, "Credit"),
    ("Other Expenses", 60102, false, "Debit"),
    ("Gains And Losses", 60200, true, "Debit"),
    (
        "Foreign Currency Transaction Gain (Loss)",
        60201,
        false,
        "Debit",
    ),
    ("Gain (Loss) On Investments", 60202, false, "Debit"),
    ("Gain (Loss) On Derivatives", 60203, false, "Debit"),
    ("Crypto Asset Gain (Loss)", 60204, false, "Debit"),
    ("Gain (Loss) On Disposal Of Assets", 60205, false, "Debit"),
    ("Debt Related Gain (Loss)", 60206, false, "Debit"),
    ("Impairment Loss", 60207, false, "Debit"),
    ("Other Gains And Losses", 60208, false, "Debit"),
    (
        "Taxes (Other Than Income And Payroll) And Fees",
        60300,
        true,
        "Debit",
    ),
    ("Real Estate Taxes And Insurance", 60301, false, "Debit"),
    ("Highway (Road) Taxes And Tolls", 60302, false, "Debit"),
    ("Direct Tax And License Fees", 60303, false, "Debit"),
    ("Excise And Sales Taxes", 60304, false, "Debit"),
    (
        "Customs Fees And Duties (Not Classified As Sales Or Excise)",
        60305,
        false,
        "Debit",
    ),
    ("Non-Deductible VAT (GST)", 60306, false, "Debit"),
    ("General Insurance Expense", 60307, false, "Debit"),
    (
        "Administrative Fees (Revenue Stamps)",
        60308,
        false,
        "Debit",
    ),
    ("Fines And Penalties", 60309, false, "Debit"),
    ("Miscellaneous Taxes", 60310, false, "Debit"),
    ("Other Taxes And Fees", 60311, false, "Debit"),
    ("Income Tax Expense (Benefit)", 60400, false, "Debit"),
    (
        "Intercompany And Related Party Accounts",
        70000,
        true,
        "Debit",
    ),
    (
        "Intercompany And Related Party Assets",
        70100,
        true,
        "Debit",
    ),
    (
        "Intercompany Balances (Eliminated In Consolidation)",
        70101,
        false,
        "Debit",
    ),
    (
        "Related Party Balances (Reported Or Disclosed)",
        70102,
        false,
        "Debit",
    ),
    ("Intercompany Investments", 70103, false, "Debit"),
    (
        "Intercompany And Related Party Liabilities",
        70200,
        true,
        "Credit",
    ),
    (
        "Intercompany Balances (Eliminated In Consolidation)",
        70201,
        false,
        "Credit",
    ),
    (
        "Related Party Balances (Reported Or Disclosed)",
        70202,
        false,
        "Credit",
    ),
    (
        "Intercompany And Related Party Income And Expense",
        70300,
        true,
        "Debit",
    ),
    (
        "Intercompany And Related Party Income",
        70301,
        false,
        "Credit",
    ),
    (
        "Intercompany And Related Party Expenses",
        70302,
        false,
        "Debit",
    ),
    (
        "Income (Loss) From Equity Method Investments",
        70303,
        false,
        "Debit",
    ),
];

const PARENT_LINKS: &[(i32, i32)] = &[
    (10100, 10000),
    (10101, 10100),
    (10102, 10100),
    (10103, 10100),
    (10104, 10100),
    (10200, 10000),
    (10201, 10200),
    (10202, 10200),
    (10203, 10200),
    (10300, 10000),
    (10301, 10300),
    (10302, 10300),
    (10303, 10300),
    (10304, 10300),
    (10305, 10300),
    (10400, 10000),
    (10401, 10400),
    (10402, 10400),
    (10403, 10400),
    (10500, 10000),
    (10501, 10500),
    (10502, 10500),
    (10503, 10500),
    (10504, 10500),
    (10505, 10500),
    (10506, 10500),
    (10507, 10500),
    (10601, 10600),
    (10602, 10600),
    (10700, 10000),
    (10701, 10700),
    (10702, 10700),
    (10703, 10700),
    (10704, 10700),
    (10705, 10700),
    (10706, 10700),
    (10707, 10700),
    (10708, 10700),
    (10900, 10000),
    (20100, 20000),
    (20101, 20100),
    (20102, 20100),
    (20103, 20100),
    (20104, 20100),
    (20200, 20000),
    (20201, 20200),
    (20202, 20200),
    (20203, 20200),
    (20204, 20200),
    (20300, 20000),
    (20301, 20300),
    (20302, 20300),
    (20303, 20300),
    (20304, 20300),
    (20305, 20300),
    (20306, 20300),
    (20307, 20300),
    (20400, 20000),
    (20401, 20400),
    (20402, 20400),
    (20403, 20400),
    (30100, 30000),
    (30101, 30100),
    (30102, 30100),
    (30200, 30000),
    (30201, 30200),
    (30202, 30200),
    (30203, 30200),
    (30204, 30200),
    (30300, 30000),
    (30301, 30300),
    (30302, 30300),
    (30303, 30300),
    (30304, 30300),
    (30305, 30300),
    (30400, 30000),
    (30401, 30400),
    (30402, 30400),
    (30403, 30400),
    (30404, 30400),
    (30500, 30000),
    (40100, 40000),
    (40101, 40100),
    (40102, 40100),
    (40200, 40000),
    (40201, 40200),
    (40202, 40200),
    (40300, 40000),
    (40301, 40300),
    (40302, 40300),
    (40303, 40300),
    (50100, 50000),
    (50101, 50100),
    (50102, 50100),
    (50103, 50100),
    (50104, 50100),
    (50105, 50100),
    (50106, 50100),
    (50200, 50000),
    (50201, 50200),
    (50202, 50200),
    (50203, 50200),
    (60100, 60000),
    (60101, 60100),
    (60102, 60100),
    (60200, 60000),
    (60201, 60200),
    (60202, 60200),
    (60203, 60200),
    (60204, 60200),
    (60205, 60200),
    (60206, 60200),
    (60207, 60200),
    (60208, 60200),
    (60300, 60000),
    (60301, 60300),
    (60302, 60300),
    (60303, 60300),
    (60304, 60300),
    (60305, 60300),
    (60306, 60300),
    (60307, 60300),
    (60308, 60300),
    (60309, 60300),
    (60310, 60300),
    (60311, 60300),
    (60400, 60000),
    (70100, 70000),
    (70101, 70100),
    (70102, 70100),
    (70103, 70100),
    (70201, 70200),
    (70202, 70200),
    (70300, 70000),
    (70302, 70300),
    (70303, 70300),
];

fn subquery_expr(sel: SelectStatement) -> SimpleExpr {
    SimpleExpr::SubQuery(None, Box::new(sel.into_sub_query_statement()))
}

const NORMALIZE_BALANCE_TYPE: &str = r#"
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'balance_type') THEN
    CREATE TYPE balance_type AS ENUM ('Credit', 'Debit');
  END IF;

  IF EXISTS (
    SELECT 1
    FROM pg_attribute a
    JOIN pg_class c ON a.attrelid = c.oid
    JOIN pg_type t ON a.atttypid = t.oid
    JOIN pg_namespace n ON c.relnamespace = n.oid
    WHERE n.nspname = current_schema()
      AND c.relname = 'accounts'
      AND a.attname = 'balance_type'
      AND NOT a.attisdropped
      AND t.typname = 'BalanceType'
  ) THEN
    ALTER TABLE accounts ALTER COLUMN balance_type TYPE TEXT USING balance_type::TEXT;
    DROP TYPE IF EXISTS "BalanceType";
    ALTER TABLE accounts
      ALTER COLUMN balance_type TYPE balance_type USING balance_type::balance_type;
  END IF;
END;
$$;
"#;

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
        execute(manager, NORMALIZE_BALANCE_TYPE).await?;

        let backend = manager.get_connection().get_database_backend();
        let conn = manager.get_connection();

        for &(name, code, is_group, balance_type) in SEED_ACCOUNTS {
            let insert = Query::insert()
                .into_table(Accounts::Table)
                .columns([
                    Accounts::CreatedAt,
                    Accounts::UpdatedAt,
                    Accounts::Name,
                    Accounts::Code,
                    Accounts::IsGroup,
                    Accounts::BalanceType,
                ])
                .values_panic([
                    Expr::current_timestamp().into(),
                    Expr::current_timestamp().into(),
                    name.into(),
                    code.into(),
                    is_group.into(),
                    Expr::val(balance_type)
                        .cast_as(BalanceTypeEnum::Enum)
                        .into(),
                ])
                .to_owned();
            conn.execute(backend.build(&insert)).await?;
        }

        for &(child_code, parent_code) in PARENT_LINKS {
            let parent_id = Query::select()
                .column(Accounts::Id)
                .from(Accounts::Table)
                .and_where(Expr::col(Accounts::Code).eq(parent_code))
                .limit(1)
                .to_owned();

            let update = Query::update()
                .table(Accounts::Table)
                .value(Accounts::ParentId, subquery_expr(parent_id))
                .and_where(Expr::col(Accounts::Code).eq(child_code))
                .to_owned();
            conn.execute(backend.build(&update)).await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
