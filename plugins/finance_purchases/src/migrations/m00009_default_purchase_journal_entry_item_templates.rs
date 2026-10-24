//! Seed purchase product and service journal templates from the old purchase accounts.
//!
//! A posted purchase debited `purchase_preferences.account_expense_id` for each
//! line's pre-tax amount and credited `account_payable_id` for the same amount,
//! on `journal_id`. Tax was posted separately through `account_input_tax_id`.
//! The template lines are `pre_tax` and `-pre_tax`, so they balance the way
//! that pre-tax pair did.
//!
//! Registered before the column drop. A blank preference falls back to the
//! accounts stamped on the latest posted purchase. Databases that already
//! dropped those columns fall back to chart accounts 50101 (Merchandise,
//! Material, Parts And Supplies) and 20101 (Trade Payables).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
DECLARE
    v_journal_id bigint;
    v_payable_id bigint;
    v_expense_id bigint;
    v_posted_payable_id bigint;
    v_posted_expense_id bigint;
    v_posted_journal_id bigint;
BEGIN
    IF to_regclass('public.purchase_product_journal_entry_item_templates') IS NULL
       OR to_regclass('public.purchase_service_journal_entry_item_templates') IS NULL
       OR to_regclass('public.purchase_products') IS NULL
       OR to_regclass('public.purchase_services') IS NULL THEN
        RETURN;
    END IF;

    IF to_regclass('public.purchase_preferences') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'purchase_preferences'
              AND column_name = 'account_payable_id'
       )
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'purchase_preferences'
              AND column_name = 'account_expense_id'
       )
    THEN
        SELECT NULLIF(account_payable_id, 0),
               NULLIF(account_expense_id, 0)
          INTO v_payable_id, v_expense_id
          FROM purchase_preferences
         WHERE id = 1;

        IF EXISTS (
            SELECT 1
              FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'purchase_preferences'
               AND column_name = 'journal_id'
        ) THEN
            SELECT NULLIF(journal_id, 0)
              INTO v_journal_id
              FROM purchase_preferences
             WHERE id = 1;
        END IF;
    END IF;

    IF to_regclass('public.posted_purchases') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'posted_purchases'
              AND column_name = 'account_payable_id'
       )
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'posted_purchases'
              AND column_name = 'account_expense_id'
       )
       AND (v_payable_id IS NULL OR v_expense_id IS NULL OR v_journal_id IS NULL)
    THEN
        SELECT NULLIF(account_payable_id, 0),
               NULLIF(account_expense_id, 0),
               NULLIF(journal_id, 0)
          INTO v_posted_payable_id, v_posted_expense_id, v_posted_journal_id
          FROM posted_purchases
         WHERE account_payable_id <> 0
           AND account_expense_id <> 0
         ORDER BY id DESC
         LIMIT 1;

        v_payable_id := COALESCE(v_payable_id, v_posted_payable_id);
        v_expense_id := COALESCE(v_expense_id, v_posted_expense_id);
        v_journal_id := COALESCE(v_journal_id, v_posted_journal_id);
    END IF;

    IF v_payable_id IS NULL AND to_regclass('public.accounts') IS NOT NULL THEN
        SELECT id
          INTO v_payable_id
          FROM accounts
         WHERE code = 20101
           AND is_group = false
         ORDER BY id
         LIMIT 1;
    END IF;

    IF v_expense_id IS NULL AND to_regclass('public.accounts') IS NOT NULL THEN
        SELECT id
          INTO v_expense_id
          FROM accounts
         WHERE code = 50101
           AND is_group = false
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

    IF v_journal_id IS NULL OR v_payable_id IS NULL OR v_expense_id IS NULL THEN
        RETURN;
    END IF;

    INSERT INTO purchase_product_journal_entry_item_templates
        (created_at, updated_at, product_id, journal_id, account_id, formula)
    SELECT NOW(), NOW(), p.id, v_journal_id, line.account_id, line.formula
      FROM purchase_products AS p
     CROSS JOIN (VALUES
         (1, v_expense_id, 'pre_tax'),
         (2, v_payable_id, '-pre_tax')
     ) AS line(ord, account_id, formula)
     WHERE NOT EXISTS (
         SELECT 1
           FROM purchase_product_journal_entry_item_templates AS existing
          WHERE existing.product_id = p.id
     )
     ORDER BY p.id, line.ord;

    INSERT INTO purchase_service_journal_entry_item_templates
        (created_at, updated_at, service_id, journal_id, account_id, formula)
    SELECT NOW(), NOW(), s.id, v_journal_id, line.account_id, line.formula
      FROM purchase_services AS s
     CROSS JOIN (VALUES
         (1, v_expense_id, 'pre_tax'),
         (2, v_payable_id, '-pre_tax')
     ) AS line(ord, account_id, formula)
     WHERE NOT EXISTS (
         SELECT 1
           FROM purchase_service_journal_entry_item_templates AS existing
          WHERE existing.service_id = s.id
     )
     ORDER BY s.id, line.ord;
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
