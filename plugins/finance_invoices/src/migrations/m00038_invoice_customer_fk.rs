//! Point invoices at `crm_customers` and drop the company/individual columns.
//!
//! One customer row is inserted for each distinct company or individual already
//! billed on a draft, posted, or cancelled invoice.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "INSERT INTO crm_customers (is_individual, company_id, individual_id, created_at, updated_at)
     SELECT FALSE, party.company_id, NULL, NOW(), NOW()
     FROM (
        SELECT customer_company AS company_id FROM draft_invoices
        WHERE NOT bill_to_individual AND customer_company IS NOT NULL
        UNION
        SELECT customer_company FROM posted_invoices
        WHERE NOT bill_to_individual AND customer_company IS NOT NULL
        UNION
        SELECT customer_company FROM cancelled_invoices
        WHERE NOT bill_to_individual AND customer_company IS NOT NULL
     ) AS party
     WHERE NOT EXISTS (
        SELECT 1 FROM crm_customers AS existing
        WHERE NOT existing.is_individual AND existing.company_id = party.company_id
     )",
    "INSERT INTO crm_customers (is_individual, company_id, individual_id, created_at, updated_at)
     SELECT TRUE, NULL, party.individual_id, NOW(), NOW()
     FROM (
        SELECT customer_individual AS individual_id FROM draft_invoices
        WHERE bill_to_individual AND customer_individual IS NOT NULL
        UNION
        SELECT customer_individual FROM posted_invoices
        WHERE bill_to_individual AND customer_individual IS NOT NULL
        UNION
        SELECT customer_individual FROM cancelled_invoices
        WHERE bill_to_individual AND customer_individual IS NOT NULL
     ) AS party
     WHERE NOT EXISTS (
        SELECT 1 FROM crm_customers AS existing
        WHERE existing.is_individual AND existing.individual_id = party.individual_id
     )",
    "ALTER TABLE draft_invoices ADD COLUMN IF NOT EXISTS customer_id bigint",
    "ALTER TABLE posted_invoices ADD COLUMN IF NOT EXISTS customer_id bigint",
    "ALTER TABLE cancelled_invoices ADD COLUMN IF NOT EXISTS customer_id bigint",
    "UPDATE draft_invoices AS d
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE NOT d.bill_to_individual
       AND NOT c.is_individual
       AND c.company_id = d.customer_company",
    "UPDATE draft_invoices AS d
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE d.bill_to_individual
       AND c.is_individual
       AND c.individual_id = d.customer_individual",
    "UPDATE posted_invoices AS d
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE NOT d.bill_to_individual
       AND NOT c.is_individual
       AND c.company_id = d.customer_company",
    "UPDATE posted_invoices AS d
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE d.bill_to_individual
       AND c.is_individual
       AND c.individual_id = d.customer_individual",
    "UPDATE cancelled_invoices AS d
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE NOT d.bill_to_individual
       AND NOT c.is_individual
       AND c.company_id = d.customer_company",
    "UPDATE cancelled_invoices AS d
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE d.bill_to_individual
       AND c.is_individual
       AND c.individual_id = d.customer_individual",
    "DO $$ BEGIN
        IF EXISTS (SELECT 1 FROM draft_invoices WHERE customer_id IS NULL)
           OR EXISTS (SELECT 1 FROM posted_invoices WHERE customer_id IS NULL)
           OR EXISTS (SELECT 1 FROM cancelled_invoices WHERE customer_id IS NULL)
        THEN
            RAISE EXCEPTION 'invoice is missing a migrated customer';
        END IF;
     END $$",
    "ALTER TABLE draft_invoices ALTER COLUMN customer_id SET NOT NULL",
    "ALTER TABLE posted_invoices ALTER COLUMN customer_id SET NOT NULL",
    "ALTER TABLE cancelled_invoices ALTER COLUMN customer_id SET NOT NULL",
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS fk_draft_invoices_customer_id",
    "ALTER TABLE draft_invoices ADD CONSTRAINT fk_draft_invoices_customer_id
        FOREIGN KEY (customer_id) REFERENCES crm_customers(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS fk_posted_invoices_customer_id",
    "ALTER TABLE posted_invoices ADD CONSTRAINT fk_posted_invoices_customer_id
        FOREIGN KEY (customer_id) REFERENCES crm_customers(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS fk_cancelled_invoices_customer_id",
    "ALTER TABLE cancelled_invoices ADD CONSTRAINT fk_cancelled_invoices_customer_id
        FOREIGN KEY (customer_id) REFERENCES crm_customers(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS ck_draft_invoices_bill_to",
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS fk_draft_invoices_customer_individual",
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS fk_draft_invoices_customer_company",
    "DROP INDEX IF EXISTS idx_draft_invoices_customer_individual",
    "DROP INDEX IF EXISTS idx_draft_invoices_customer_company",
    "ALTER TABLE draft_invoices DROP COLUMN IF EXISTS bill_to_individual",
    "ALTER TABLE draft_invoices DROP COLUMN IF EXISTS customer_individual",
    "ALTER TABLE draft_invoices DROP COLUMN IF EXISTS customer_company",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS ck_posted_invoices_bill_to",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS fk_posted_invoices_customer_individual",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS fk_posted_invoices_customer_company",
    "DROP INDEX IF EXISTS idx_posted_invoices_customer_individual",
    "DROP INDEX IF EXISTS idx_posted_invoices_customer_company",
    "ALTER TABLE posted_invoices DROP COLUMN IF EXISTS bill_to_individual",
    "ALTER TABLE posted_invoices DROP COLUMN IF EXISTS customer_individual",
    "ALTER TABLE posted_invoices DROP COLUMN IF EXISTS customer_company",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS ck_cancelled_invoices_bill_to",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS fk_cancelled_invoices_customer_individual",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS fk_cancelled_invoices_customer_company",
    "DROP INDEX IF EXISTS idx_cancelled_invoices_customer_individual",
    "DROP INDEX IF EXISTS idx_cancelled_invoices_customer_company",
    "ALTER TABLE cancelled_invoices DROP COLUMN IF EXISTS bill_to_individual",
    "ALTER TABLE cancelled_invoices DROP COLUMN IF EXISTS customer_individual",
    "ALTER TABLE cancelled_invoices DROP COLUMN IF EXISTS customer_company",
    "CREATE INDEX IF NOT EXISTS idx_draft_invoices_customer_id ON draft_invoices (customer_id)",
    "CREATE INDEX IF NOT EXISTS idx_posted_invoices_customer_id ON posted_invoices (customer_id)",
    "CREATE INDEX IF NOT EXISTS idx_cancelled_invoices_customer_id ON cancelled_invoices (customer_id)",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in UP {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "invoice customer migration is not reversible".into(),
        ))
    }
}
