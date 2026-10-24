//! Move purchases that already point at `crm_customers` onto `purchase_vendors`.
//!
//! Databases that applied the earlier customer foreign key keep `customer_id`.
//! Each referenced customer is copied, with the same id, into `purchase_vendors`,
//! and the purchase column is renamed to `vendor_id`. Installs that already
//! stored `vendor_id` leave this migration as a no-op.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE IF NOT EXISTS purchase_vendors (
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
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_purchase_vendors_company
    ON purchase_vendors (company_id) WHERE NOT is_individual;
CREATE UNIQUE INDEX IF NOT EXISTS ux_purchase_vendors_individual
    ON purchase_vendors (individual_id) WHERE is_individual;

DO $$
BEGIN
    IF to_regclass('public.draft_purchases') IS NULL THEN
        RETURN;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'draft_purchases'
           AND column_name = 'vendor_id'
    ) THEN
        RETURN;
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'draft_purchases'
           AND column_name = 'customer_id'
    ) THEN
        RETURN;
    END IF;

    INSERT INTO purchase_vendors (
        id, created_at, updated_at, is_individual, company_id, individual_id
    )
    SELECT c.id, c.created_at, c.updated_at, c.is_individual, c.company_id, c.individual_id
      FROM crm_customers AS c
     WHERE c.id IN (
        SELECT customer_id FROM draft_purchases
        UNION
        SELECT customer_id FROM posted_purchases
        UNION
        SELECT customer_id FROM cancelled_purchases
     )
    ON CONFLICT (id) DO NOTHING;

    PERFORM setval(
        pg_get_serial_sequence('purchase_vendors', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM purchase_vendors), 1), 1),
        EXISTS (SELECT 1 FROM purchase_vendors)
    );

    ALTER TABLE draft_purchases DROP CONSTRAINT IF EXISTS fk_draft_purchases_customer_id;
    ALTER TABLE posted_purchases DROP CONSTRAINT IF EXISTS fk_posted_purchases_customer_id;
    ALTER TABLE cancelled_purchases DROP CONSTRAINT IF EXISTS fk_cancelled_purchases_customer_id;

    ALTER TABLE draft_purchases RENAME COLUMN customer_id TO vendor_id;
    ALTER TABLE posted_purchases RENAME COLUMN customer_id TO vendor_id;
    ALTER TABLE cancelled_purchases RENAME COLUMN customer_id TO vendor_id;

    ALTER TABLE draft_purchases DROP CONSTRAINT IF EXISTS fk_draft_purchases_vendor_id;
    ALTER TABLE draft_purchases
        ADD CONSTRAINT fk_draft_purchases_vendor_id
        FOREIGN KEY (vendor_id) REFERENCES purchase_vendors(id)
        ON DELETE RESTRICT ON UPDATE CASCADE;
    ALTER TABLE posted_purchases DROP CONSTRAINT IF EXISTS fk_posted_purchases_vendor_id;
    ALTER TABLE posted_purchases
        ADD CONSTRAINT fk_posted_purchases_vendor_id
        FOREIGN KEY (vendor_id) REFERENCES purchase_vendors(id)
        ON DELETE RESTRICT ON UPDATE CASCADE;
    ALTER TABLE cancelled_purchases DROP CONSTRAINT IF EXISTS fk_cancelled_purchases_vendor_id;
    ALTER TABLE cancelled_purchases
        ADD CONSTRAINT fk_cancelled_purchases_vendor_id
        FOREIGN KEY (vendor_id) REFERENCES purchase_vendors(id)
        ON DELETE RESTRICT ON UPDATE CASCADE;

    DROP INDEX IF EXISTS idx_draft_purchases_customer_id;
    DROP INDEX IF EXISTS idx_posted_purchases_customer_id;
    DROP INDEX IF EXISTS idx_cancelled_purchases_customer_id;
    CREATE INDEX IF NOT EXISTS idx_draft_purchases_vendor_id ON draft_purchases (vendor_id);
    CREATE INDEX IF NOT EXISTS idx_posted_purchases_vendor_id ON posted_purchases (vendor_id);
    CREATE INDEX IF NOT EXISTS idx_cancelled_purchases_vendor_id ON cancelled_purchases (vendor_id);
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "purchase vendor migration is not reversible".into(),
        ))
    }
}
