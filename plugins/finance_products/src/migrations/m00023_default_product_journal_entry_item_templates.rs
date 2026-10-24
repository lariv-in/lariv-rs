//! Seed default product journal templates from the old GL preferences.
//!
//! Each product posts its own sale: receivable debits `unit_rate` and revenue
//! credits it back. Cost of sales and inventory are not copied onto products.
//! Invoice posting expands every product template, so those lines would be
//! booked once per product. The same two sales lines are stored on product
//! preferences so a new product opens with that default. Once the accounts
//! are copied, the preference columns are retired.
//!
//! Formulas are stored plainly: numeric constants are wrapped in `decimal(...)`
//! when a formula is evaluated, so a credit line is just `-unit_rate`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE IF NOT EXISTS product_preference_journal_items (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    product_preferences_id bigint NOT NULL
        REFERENCES product_preferences(id) ON UPDATE CASCADE ON DELETE CASCADE,
    journal_id bigint NOT NULL
        REFERENCES journals(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    account_id bigint NOT NULL
        REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    formula text NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_product_preference_journal_items_prefs_id
    ON product_preference_journal_items (product_preferences_id);
CREATE INDEX IF NOT EXISTS idx_product_preference_journal_items_journal_id
    ON product_preference_journal_items (journal_id);
CREATE INDEX IF NOT EXISTS idx_product_preference_journal_items_account_id
    ON product_preference_journal_items (account_id);

DO $$
DECLARE
    v_journal_id bigint;
    v_receivable_id bigint;
    v_revenue_id bigint;
BEGIN
    IF to_regclass('public.product_journal_entry_item_templates') IS NULL
       OR to_regclass('public.product_preferences') IS NULL
       OR to_regclass('public.products') IS NULL THEN
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
       OR v_revenue_id IS NULL THEN
        RETURN;
    END IF;

    INSERT INTO product_journal_entry_item_templates
        (created_at, updated_at, product_id, journal_id, account_id, formula)
    SELECT NOW(), NOW(), p.id, v_journal_id, line.account_id, line.formula
      FROM products AS p
     CROSS JOIN (VALUES
         (1, v_receivable_id, 'unit_rate'),
         (2, v_revenue_id, '-unit_rate')
     ) AS line(ord, account_id, formula)
     WHERE NOT EXISTS (
         SELECT 1
           FROM product_journal_entry_item_templates AS existing
          WHERE existing.product_id = p.id
     )
     ORDER BY p.id, line.ord;

    INSERT INTO product_preferences (id, created_at, updated_at)
    VALUES (1, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    IF pg_get_serial_sequence('product_preferences', 'id') IS NOT NULL THEN
        PERFORM setval(
            pg_get_serial_sequence('product_preferences', 'id'),
            GREATEST(COALESCE((SELECT MAX(id) FROM product_preferences), 1), 1),
            TRUE
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM product_preference_journal_items
         WHERE product_preferences_id = 1
    ) THEN
        INSERT INTO product_preference_journal_items
            (created_at, updated_at, product_preferences_id, journal_id, account_id, formula)
        VALUES
            (NOW(), NOW(), 1, v_journal_id, v_receivable_id, 'unit_rate'),
            (NOW(), NOW(), 1, v_journal_id, v_revenue_id, '-unit_rate');
    END IF;
END $$;

ALTER TABLE product_preferences
    DROP COLUMN IF EXISTS inventory_account_id,
    DROP COLUMN IF EXISTS cost_of_sales_account_id;
"#;

const DOWN: &str = r#"
DROP TABLE IF EXISTS product_preference_journal_items;

ALTER TABLE product_preferences
    ADD COLUMN IF NOT EXISTS inventory_account_id bigint NULL,
    ADD COLUMN IF NOT EXISTS cost_of_sales_account_id bigint NULL;

ALTER TABLE product_preferences
    DROP CONSTRAINT IF EXISTS fk_product_preferences_inventory_account_id,
    DROP CONSTRAINT IF EXISTS fk_product_preferences_cost_of_sales_account_id;

ALTER TABLE product_preferences
    ADD CONSTRAINT fk_product_preferences_inventory_account_id
        FOREIGN KEY (inventory_account_id) REFERENCES accounts(id)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    ADD CONSTRAINT fk_product_preferences_cost_of_sales_account_id
        FOREIGN KEY (cost_of_sales_account_id) REFERENCES accounts(id)
        ON UPDATE CASCADE ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_product_preferences_inventory_account_id
    ON product_preferences (inventory_account_id);
CREATE INDEX IF NOT EXISTS idx_product_preferences_cost_of_sales_account_id
    ON product_preferences (cost_of_sales_account_id);
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
