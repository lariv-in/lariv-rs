//! Seed default service journal templates from the old GL preferences.
//!
//! Service preferences carry their own cost accounts. When those are unset the
//! migration falls back to the product-side accounts, which is what the legacy
//! service rows used before services were split out of products. Because the
//! product migration already retired the product preference columns, that
//! fallback also reads the product templates seeded immediately before this
//! migration.
//!
//! Formulas are stored plainly: numeric constants are wrapped in `decimal(...)`
//! when a formula is evaluated, so a credit line is just `-pre_tax`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
DECLARE
    v_journal_id bigint;
    v_receivable_id bigint;
    v_revenue_id bigint;
    v_cogs_id bigint;
    v_credit_id bigint;
BEGIN
    IF to_regclass('public.service_journal_entry_item_templates') IS NULL
       OR to_regclass('public.service_preferences') IS NULL
       OR to_regclass('public.services') IS NULL THEN
        RETURN;
    END IF;

    IF to_regclass('public.invoice_preferences') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'invoice_preferences'
              AND column_name = 'account_receivable_id'
       )
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'invoice_preferences'
              AND column_name = 'account_revenue_id'
       )
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'invoice_preferences'
              AND column_name = 'journal_id'
       )
    THEN
        SELECT NULLIF(account_receivable_id, 0),
               NULLIF(account_revenue_id, 0),
               NULLIF(journal_id, 0)
          INTO v_receivable_id, v_revenue_id, v_journal_id
          FROM invoice_preferences
         WHERE id = 1;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'service_preferences'
           AND column_name = 'cost_of_sales_account_id'
    )
    AND EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'service_preferences'
           AND column_name = 'cost_credit_account_id'
    )
    THEN
        SELECT NULLIF(cost_of_sales_account_id, 0),
               NULLIF(cost_credit_account_id, 0)
          INTO v_cogs_id, v_credit_id
          FROM service_preferences
         WHERE id = 1;
    END IF;

    IF v_cogs_id IS NULL
       AND to_regclass('public.product_preferences') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'product_preferences'
              AND column_name = 'cost_of_sales_account_id'
       )
    THEN
        SELECT NULLIF(cost_of_sales_account_id, 0)
          INTO v_cogs_id
          FROM product_preferences
         WHERE id = 1;
    END IF;

    IF v_credit_id IS NULL
       AND to_regclass('public.product_preferences') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'product_preferences'
              AND column_name = 'inventory_account_id'
       )
    THEN
        SELECT NULLIF(inventory_account_id, 0)
          INTO v_credit_id
          FROM product_preferences
         WHERE id = 1;
    END IF;

    IF v_cogs_id IS NULL
       AND to_regclass('public.product_journal_entry_item_templates') IS NOT NULL
    THEN
        SELECT account_id
          INTO v_cogs_id
          FROM product_journal_entry_item_templates
         WHERE formula = 'quantity * base_cost'
         ORDER BY id
         LIMIT 1;
    END IF;

    IF v_credit_id IS NULL
       AND to_regclass('public.product_journal_entry_item_templates') IS NOT NULL
    THEN
        SELECT account_id
          INTO v_credit_id
          FROM product_journal_entry_item_templates
         WHERE formula IN ('-quantity * base_cost', 'decimal("0") - quantity * base_cost')
         ORDER BY id
         LIMIT 1;
    END IF;

    IF v_journal_id IS NULL AND to_regclass('public.journals') IS NOT NULL THEN
        SELECT id
          INTO v_journal_id
          FROM journals
         WHERE is_active = true
         ORDER BY id
         LIMIT 1;
    END IF;

    IF v_journal_id IS NULL
       OR v_receivable_id IS NULL
       OR v_revenue_id IS NULL
       OR v_cogs_id IS NULL
       OR v_credit_id IS NULL THEN
        RETURN;
    END IF;

    INSERT INTO service_journal_entry_item_templates
        (created_at, updated_at, service_id, journal_id, account_id, formula)
    SELECT NOW(), NOW(), s.id, v_journal_id, line.account_id, line.formula
      FROM services AS s
     CROSS JOIN (VALUES
         (1, v_receivable_id, 'pre_tax'),
         (2, v_revenue_id, '-pre_tax'),
         (3, v_cogs_id, 'quantity * base_cost'),
         (4, v_credit_id, '-quantity * base_cost')
     ) AS line(ord, account_id, formula)
     WHERE NOT EXISTS (
         SELECT 1
           FROM service_journal_entry_item_templates AS existing
          WHERE existing.service_id = s.id
     )
     ORDER BY s.id, line.ord;
END $$;

ALTER TABLE service_preferences
    DROP COLUMN IF EXISTS cost_of_sales_account_id,
    DROP COLUMN IF EXISTS cost_credit_account_id;
"#;

const DOWN: &str = r#"
ALTER TABLE service_preferences
    ADD COLUMN IF NOT EXISTS cost_of_sales_account_id bigint NULL,
    ADD COLUMN IF NOT EXISTS cost_credit_account_id bigint NULL;

ALTER TABLE service_preferences
    DROP CONSTRAINT IF EXISTS service_preferences_cost_of_sales_account_id_fkey,
    DROP CONSTRAINT IF EXISTS service_preferences_cost_credit_account_id_fkey;

ALTER TABLE service_preferences
    ADD CONSTRAINT service_preferences_cost_of_sales_account_id_fkey
        FOREIGN KEY (cost_of_sales_account_id) REFERENCES accounts(id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    ADD CONSTRAINT service_preferences_cost_credit_account_id_fkey
        FOREIGN KEY (cost_credit_account_id) REFERENCES accounts(id)
        ON UPDATE CASCADE ON DELETE RESTRICT;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        Ok(())
    }
}
