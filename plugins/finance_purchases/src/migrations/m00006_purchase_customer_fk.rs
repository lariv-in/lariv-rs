//! Point purchases at `purchase_vendors` and drop the company/individual columns.
//!
//! One vendor row is inserted for each distinct company or individual already
//! billed on a draft, posted, or cancelled purchase. A vendor is one company
//! or one contact, the same shape as a customer, and purchases reference it
//! instead of `crm_customers`.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS purchase_vendors (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        is_individual boolean NOT NULL,
        company_id bigint,
        individual_id bigint,
        CONSTRAINT fk_purchase_vendors_company_id
            FOREIGN KEY (company_id) REFERENCES crm_companies(id)
            ON DELETE RESTRICT ON UPDATE CASCADE,
        CONSTRAINT fk_purchase_vendors_individual_id
            FOREIGN KEY (individual_id) REFERENCES crm_contacts(id)
            ON DELETE RESTRICT ON UPDATE CASCADE,
        CONSTRAINT ck_purchase_vendors_party CHECK (
            (is_individual AND individual_id IS NOT NULL AND company_id IS NULL)
            OR (NOT is_individual AND company_id IS NOT NULL AND individual_id IS NULL)
        )
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS ux_purchase_vendors_company
        ON purchase_vendors (company_id) WHERE NOT is_individual",
    "CREATE UNIQUE INDEX IF NOT EXISTS ux_purchase_vendors_individual
        ON purchase_vendors (individual_id) WHERE is_individual",
    "INSERT INTO purchase_vendors (is_individual, company_id, individual_id, created_at, updated_at)
     SELECT FALSE, party.company_id, NULL, NOW(), NOW()
     FROM (
        SELECT vendor_company_id AS company_id FROM draft_purchases
        WHERE NOT vendor_is_individual AND vendor_company_id IS NOT NULL
        UNION
        SELECT vendor_company_id FROM posted_purchases
        WHERE NOT vendor_is_individual AND vendor_company_id IS NOT NULL
        UNION
        SELECT vendor_company_id FROM cancelled_purchases
        WHERE NOT vendor_is_individual AND vendor_company_id IS NOT NULL
     ) AS party
     WHERE NOT EXISTS (
        SELECT 1 FROM purchase_vendors AS existing
        WHERE NOT existing.is_individual AND existing.company_id = party.company_id
     )",
    "INSERT INTO purchase_vendors (is_individual, company_id, individual_id, created_at, updated_at)
     SELECT TRUE, NULL, party.individual_id, NOW(), NOW()
     FROM (
        SELECT vendor_contact_id AS individual_id FROM draft_purchases
        WHERE vendor_is_individual AND vendor_contact_id IS NOT NULL
        UNION
        SELECT vendor_contact_id FROM posted_purchases
        WHERE vendor_is_individual AND vendor_contact_id IS NOT NULL
        UNION
        SELECT vendor_contact_id FROM cancelled_purchases
        WHERE vendor_is_individual AND vendor_contact_id IS NOT NULL
     ) AS party
     WHERE NOT EXISTS (
        SELECT 1 FROM purchase_vendors AS existing
        WHERE existing.is_individual AND existing.individual_id = party.individual_id
     )",
    "ALTER TABLE draft_purchases ADD COLUMN IF NOT EXISTS vendor_id bigint",
    "ALTER TABLE posted_purchases ADD COLUMN IF NOT EXISTS vendor_id bigint",
    "ALTER TABLE cancelled_purchases ADD COLUMN IF NOT EXISTS vendor_id bigint",
    "UPDATE draft_purchases AS d
     SET vendor_id = v.id
     FROM purchase_vendors AS v
     WHERE NOT d.vendor_is_individual
       AND NOT v.is_individual
       AND v.company_id = d.vendor_company_id",
    "UPDATE draft_purchases AS d
     SET vendor_id = v.id
     FROM purchase_vendors AS v
     WHERE d.vendor_is_individual
       AND v.is_individual
       AND v.individual_id = d.vendor_contact_id",
    "UPDATE posted_purchases AS d
     SET vendor_id = v.id
     FROM purchase_vendors AS v
     WHERE NOT d.vendor_is_individual
       AND NOT v.is_individual
       AND v.company_id = d.vendor_company_id",
    "UPDATE posted_purchases AS d
     SET vendor_id = v.id
     FROM purchase_vendors AS v
     WHERE d.vendor_is_individual
       AND v.is_individual
       AND v.individual_id = d.vendor_contact_id",
    "UPDATE cancelled_purchases AS d
     SET vendor_id = v.id
     FROM purchase_vendors AS v
     WHERE NOT d.vendor_is_individual
       AND NOT v.is_individual
       AND v.company_id = d.vendor_company_id",
    "UPDATE cancelled_purchases AS d
     SET vendor_id = v.id
     FROM purchase_vendors AS v
     WHERE d.vendor_is_individual
       AND v.is_individual
       AND v.individual_id = d.vendor_contact_id",
    "DO $$ BEGIN
        IF EXISTS (SELECT 1 FROM draft_purchases WHERE vendor_id IS NULL)
           OR EXISTS (SELECT 1 FROM posted_purchases WHERE vendor_id IS NULL)
           OR EXISTS (SELECT 1 FROM cancelled_purchases WHERE vendor_id IS NULL)
        THEN
            RAISE EXCEPTION 'purchase is missing a migrated vendor';
        END IF;
     END $$",
    "ALTER TABLE draft_purchases ALTER COLUMN vendor_id SET NOT NULL",
    "ALTER TABLE posted_purchases ALTER COLUMN vendor_id SET NOT NULL",
    "ALTER TABLE cancelled_purchases ALTER COLUMN vendor_id SET NOT NULL",
    "ALTER TABLE draft_purchases DROP CONSTRAINT IF EXISTS fk_draft_purchases_vendor_id",
    "ALTER TABLE draft_purchases ADD CONSTRAINT fk_draft_purchases_vendor_id
        FOREIGN KEY (vendor_id) REFERENCES purchase_vendors(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE posted_purchases DROP CONSTRAINT IF EXISTS fk_posted_purchases_vendor_id",
    "ALTER TABLE posted_purchases ADD CONSTRAINT fk_posted_purchases_vendor_id
        FOREIGN KEY (vendor_id) REFERENCES purchase_vendors(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE cancelled_purchases DROP CONSTRAINT IF EXISTS fk_cancelled_purchases_vendor_id",
    "ALTER TABLE cancelled_purchases ADD CONSTRAINT fk_cancelled_purchases_vendor_id
        FOREIGN KEY (vendor_id) REFERENCES purchase_vendors(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE draft_purchases DROP COLUMN IF EXISTS vendor_is_individual",
    "ALTER TABLE draft_purchases DROP COLUMN IF EXISTS vendor_contact_id",
    "ALTER TABLE draft_purchases DROP COLUMN IF EXISTS vendor_company_id",
    "ALTER TABLE posted_purchases DROP COLUMN IF EXISTS vendor_is_individual",
    "ALTER TABLE posted_purchases DROP COLUMN IF EXISTS vendor_contact_id",
    "ALTER TABLE posted_purchases DROP COLUMN IF EXISTS vendor_company_id",
    "ALTER TABLE cancelled_purchases DROP COLUMN IF EXISTS vendor_is_individual",
    "ALTER TABLE cancelled_purchases DROP COLUMN IF EXISTS vendor_contact_id",
    "ALTER TABLE cancelled_purchases DROP COLUMN IF EXISTS vendor_company_id",
    "CREATE INDEX IF NOT EXISTS idx_draft_purchases_vendor_id ON draft_purchases (vendor_id)",
    "CREATE INDEX IF NOT EXISTS idx_posted_purchases_vendor_id ON posted_purchases (vendor_id)",
    "CREATE INDEX IF NOT EXISTS idx_cancelled_purchases_vendor_id ON cancelled_purchases (vendor_id)",
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
            "purchase vendor migration is not reversible".into(),
        ))
    }
}
