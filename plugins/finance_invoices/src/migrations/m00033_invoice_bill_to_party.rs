//! Point invoices at a contact or a company, then drop `customers`.
//!
//! The KDS delivery migration may already have matched challan customers onto
//! an existing company or contact and stamped `legacy_customer_id`. Those rows
//! are reused. Everyone else is copied from `customers`. The stamp columns are
//! dropped at the end.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "ALTER TABLE draft_invoices ADD COLUMN IF NOT EXISTS bill_to_individual boolean",
    "ALTER TABLE draft_invoices ADD COLUMN IF NOT EXISTS customer_individual bigint",
    "ALTER TABLE draft_invoices ADD COLUMN IF NOT EXISTS customer_company bigint",
    "ALTER TABLE posted_invoices ADD COLUMN IF NOT EXISTS bill_to_individual boolean",
    "ALTER TABLE posted_invoices ADD COLUMN IF NOT EXISTS customer_individual bigint",
    "ALTER TABLE posted_invoices ADD COLUMN IF NOT EXISTS customer_company bigint",
    "ALTER TABLE cancelled_invoices ADD COLUMN IF NOT EXISTS bill_to_individual boolean",
    "ALTER TABLE cancelled_invoices ADD COLUMN IF NOT EXISTS customer_individual bigint",
    "ALTER TABLE cancelled_invoices ADD COLUMN IF NOT EXISTS customer_company bigint",
    "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS legacy_customer_id bigint",
    "ALTER TABLE crm_contacts ADD COLUMN IF NOT EXISTS legacy_customer_id bigint",
    "INSERT INTO crm_companies (
        name, address_line_1, address_line_2, city, pincode, state, website,
        gstin, cin, pan, phone, email, created_at, updated_at, legacy_customer_id
     )
     SELECT
        name, address_line_1, address_line_2, city, pincode, state, website,
        gstin, cin, pan, phone, email, created_at, updated_at, id
     FROM customers AS src
     WHERE src.customer_type <> 'individual'
       AND NOT EXISTS (
            SELECT 1 FROM crm_companies AS existing
            WHERE existing.legacy_customer_id = src.id
       )",
    "INSERT INTO crm_contacts (
        name, email, phone, is_primary, company_id, created_at, updated_at, legacy_customer_id
     )
     SELECT name, email, phone, FALSE, NULL, created_at, updated_at, id
     FROM customers AS src
     WHERE src.customer_type = 'individual'
       AND NOT EXISTS (
            SELECT 1 FROM crm_contacts AS existing
            WHERE existing.legacy_customer_id = src.id
       )",
    "UPDATE draft_invoices AS d
     SET bill_to_individual = FALSE,
         customer_company = c.id,
         customer_individual = NULL
     FROM crm_companies AS c
     WHERE c.legacy_customer_id = d.customer_id",
    "UPDATE draft_invoices AS d
     SET bill_to_individual = TRUE,
         customer_individual = c.id,
         customer_company = NULL
     FROM crm_contacts AS c
     WHERE c.legacy_customer_id = d.customer_id",
    "UPDATE posted_invoices AS d
     SET bill_to_individual = FALSE,
         customer_company = c.id,
         customer_individual = NULL
     FROM crm_companies AS c
     WHERE c.legacy_customer_id = d.customer_id",
    "UPDATE posted_invoices AS d
     SET bill_to_individual = TRUE,
         customer_individual = c.id,
         customer_company = NULL
     FROM crm_contacts AS c
     WHERE c.legacy_customer_id = d.customer_id",
    "UPDATE cancelled_invoices AS d
     SET bill_to_individual = FALSE,
         customer_company = c.id,
         customer_individual = NULL
     FROM crm_companies AS c
     WHERE c.legacy_customer_id = d.customer_id",
    "UPDATE cancelled_invoices AS d
     SET bill_to_individual = TRUE,
         customer_individual = c.id,
         customer_company = NULL
     FROM crm_contacts AS c
     WHERE c.legacy_customer_id = d.customer_id",
    "DO $$ BEGIN
        IF EXISTS (SELECT 1 FROM draft_invoices WHERE bill_to_individual IS NULL)
           OR EXISTS (SELECT 1 FROM posted_invoices WHERE bill_to_individual IS NULL)
           OR EXISTS (SELECT 1 FROM cancelled_invoices WHERE bill_to_individual IS NULL)
        THEN
            RAISE EXCEPTION 'invoice is missing a migrated bill-to contact or company';
        END IF;
     END $$",
    "ALTER TABLE draft_invoices ALTER COLUMN bill_to_individual SET NOT NULL",
    "ALTER TABLE posted_invoices ALTER COLUMN bill_to_individual SET NOT NULL",
    "ALTER TABLE cancelled_invoices ALTER COLUMN bill_to_individual SET NOT NULL",
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS fk_draft_invoices_customer_id",
    "DROP INDEX IF EXISTS idx_draft_invoices_customer_id",
    "ALTER TABLE draft_invoices DROP COLUMN IF EXISTS customer_id",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS fk_posted_invoices_customer_id",
    "ALTER TABLE posted_invoices DROP COLUMN IF EXISTS customer_id",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS fk_cancelled_invoices_customer_id",
    "ALTER TABLE cancelled_invoices DROP COLUMN IF EXISTS customer_id",
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS fk_draft_invoices_customer_individual",
    "ALTER TABLE draft_invoices ADD CONSTRAINT fk_draft_invoices_customer_individual
        FOREIGN KEY (customer_individual) REFERENCES crm_contacts(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS fk_draft_invoices_customer_company",
    "ALTER TABLE draft_invoices ADD CONSTRAINT fk_draft_invoices_customer_company
        FOREIGN KEY (customer_company) REFERENCES crm_companies(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE draft_invoices DROP CONSTRAINT IF EXISTS ck_draft_invoices_bill_to",
    "ALTER TABLE draft_invoices ADD CONSTRAINT ck_draft_invoices_bill_to CHECK (
        (bill_to_individual AND customer_individual IS NOT NULL AND customer_company IS NULL)
        OR (NOT bill_to_individual AND customer_company IS NOT NULL AND customer_individual IS NULL)
     )",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS fk_posted_invoices_customer_individual",
    "ALTER TABLE posted_invoices ADD CONSTRAINT fk_posted_invoices_customer_individual
        FOREIGN KEY (customer_individual) REFERENCES crm_contacts(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS fk_posted_invoices_customer_company",
    "ALTER TABLE posted_invoices ADD CONSTRAINT fk_posted_invoices_customer_company
        FOREIGN KEY (customer_company) REFERENCES crm_companies(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE posted_invoices DROP CONSTRAINT IF EXISTS ck_posted_invoices_bill_to",
    "ALTER TABLE posted_invoices ADD CONSTRAINT ck_posted_invoices_bill_to CHECK (
        (bill_to_individual AND customer_individual IS NOT NULL AND customer_company IS NULL)
        OR (NOT bill_to_individual AND customer_company IS NOT NULL AND customer_individual IS NULL)
     )",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS fk_cancelled_invoices_customer_individual",
    "ALTER TABLE cancelled_invoices ADD CONSTRAINT fk_cancelled_invoices_customer_individual
        FOREIGN KEY (customer_individual) REFERENCES crm_contacts(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS fk_cancelled_invoices_customer_company",
    "ALTER TABLE cancelled_invoices ADD CONSTRAINT fk_cancelled_invoices_customer_company
        FOREIGN KEY (customer_company) REFERENCES crm_companies(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE cancelled_invoices DROP CONSTRAINT IF EXISTS ck_cancelled_invoices_bill_to",
    "ALTER TABLE cancelled_invoices ADD CONSTRAINT ck_cancelled_invoices_bill_to CHECK (
        (bill_to_individual AND customer_individual IS NOT NULL AND customer_company IS NULL)
        OR (NOT bill_to_individual AND customer_company IS NOT NULL AND customer_individual IS NULL)
     )",
    "CREATE INDEX IF NOT EXISTS idx_draft_invoices_customer_individual ON draft_invoices (customer_individual)",
    "CREATE INDEX IF NOT EXISTS idx_draft_invoices_customer_company ON draft_invoices (customer_company)",
    "CREATE INDEX IF NOT EXISTS idx_posted_invoices_customer_individual ON posted_invoices (customer_individual)",
    "CREATE INDEX IF NOT EXISTS idx_posted_invoices_customer_company ON posted_invoices (customer_company)",
    "CREATE INDEX IF NOT EXISTS idx_cancelled_invoices_customer_individual ON cancelled_invoices (customer_individual)",
    "CREATE INDEX IF NOT EXISTS idx_cancelled_invoices_customer_company ON cancelled_invoices (customer_company)",
    "DROP TABLE IF EXISTS customers CASCADE",
    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS legacy_customer_id",
    "ALTER TABLE crm_contacts DROP COLUMN IF EXISTS legacy_customer_id",
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
            "invoice bill-to migration is not reversible".into(),
        ))
    }
}
