//! Allow an individual invoice to also store the contact's company.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const CHECK: &str = "(bill_to_individual AND customer_individual IS NOT NULL) \
     OR (NOT bill_to_individual AND customer_company IS NOT NULL AND customer_individual IS NULL)";

const UP: &[&str] = &[
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS ck_draft_invoices_bill_to",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS ck_posted_invoices_bill_to",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS ck_cancelled_invoices_bill_to",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in UP {
            exec_sql(manager, sql).await?;
        }
        for (table, name) in [
            ("draft_invoices", "ck_draft_invoices_bill_to"),
            ("posted_invoices", "ck_posted_invoices_bill_to"),
            ("cancelled_invoices", "ck_cancelled_invoices_bill_to"),
        ] {
            exec_sql(
                manager,
                &format!("ALTER TABLE {table} ADD CONSTRAINT {name} CHECK ({CHECK})"),
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "invoice individual company migration is not reversible".into(),
        ))
    }
}
