//! Seed default tax journal templates from the accounts posting used before templates.
//!
//! Invoice posting debited receivable for a levied tax and credited the tax
//! payable account. A withholding tax debited that tax's own ledger account,
//! and receivable was lower by the same amount. Product templates now debit
//! receivable for `pre_tax` only, so each tax template carries its own share:
//!
//! * levied: debit receivable `base * percentage / 100`, credit the tax's
//!   account when one was stored and otherwise the invoice tax-payable account
//! * withholding: debit the tax's account `base * percentage / 100`, credit
//!   receivable `-base * percentage / 100`
//!
//! Formulas are stored plainly: numeric constants are wrapped in `decimal(...)`
//! when a formula is evaluated, so a credit line is just `-base * percentage / 100`.
//! Tax preferences keep the shared levied pair (receivable, then the invoice
//! tax-payable account) so a new tax opens with that default. Once the lines
//! are copied, `taxes.account_id` is retired.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE IF NOT EXISTS tax_preferences (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL
);

CREATE TABLE IF NOT EXISTS tax_preference_journal_items (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    tax_preferences_id bigint NOT NULL
        REFERENCES tax_preferences(id) ON UPDATE CASCADE ON DELETE CASCADE,
    journal_id bigint NOT NULL
        REFERENCES journals(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    account_id bigint NOT NULL
        REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    formula text NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tax_preference_journal_items_prefs_id
    ON tax_preference_journal_items (tax_preferences_id);
CREATE INDEX IF NOT EXISTS idx_tax_preference_journal_items_journal_id
    ON tax_preference_journal_items (journal_id);
CREATE INDEX IF NOT EXISTS idx_tax_preference_journal_items_account_id
    ON tax_preference_journal_items (account_id);

DO $$
DECLARE
    v_journal_id bigint;
    v_receivable_id bigint;
    v_tax_payable_id bigint;
    v_has_account boolean;
BEGIN
    IF to_regclass('public.tax_journal_entry_item_templates') IS NULL
       OR to_regclass('public.taxes') IS NULL THEN
        RETURN;
    END IF;

    v_has_account := EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'taxes'
           AND column_name = 'account_id'
    );

    IF to_regclass('public.invoice_preferences') IS NOT NULL THEN
        IF EXISTS (
            SELECT 1
              FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'invoice_preferences'
               AND column_name = 'account_receivable_id'
        ) THEN
            SELECT NULLIF(account_receivable_id, 0)
              INTO v_receivable_id
              FROM invoice_preferences
             WHERE id = 1;
        END IF;

        IF EXISTS (
            SELECT 1
              FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'invoice_preferences'
               AND column_name = 'account_tax_payable_id'
        ) THEN
            SELECT NULLIF(account_tax_payable_id, 0)
              INTO v_tax_payable_id
              FROM invoice_preferences
             WHERE id = 1;
        END IF;

        IF EXISTS (
            SELECT 1
              FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'invoice_preferences'
               AND column_name = 'journal_id'
        ) THEN
            SELECT NULLIF(journal_id, 0)
              INTO v_journal_id
              FROM invoice_preferences
             WHERE id = 1;
        END IF;
    END IF;

    IF v_journal_id IS NULL AND to_regclass('public.journals') IS NOT NULL THEN
        SELECT id
          INTO v_journal_id
          FROM journals
         WHERE is_active = true
         ORDER BY id
         LIMIT 1;
    END IF;

    IF v_journal_id IS NULL OR v_receivable_id IS NULL THEN
        RETURN;
    END IF;

    IF v_has_account THEN
        INSERT INTO tax_journal_entry_item_templates
            (created_at, updated_at, tax_id, journal_id, account_id, formula)
        SELECT NOW(), NOW(), t.id, v_journal_id, line.account_id, line.formula
          FROM taxes AS t
         CROSS JOIN LATERAL (VALUES
             (1, v_receivable_id, 'base * percentage / 100'),
             (2, COALESCE(NULLIF(t.account_id, 0), v_tax_payable_id), '-base * percentage / 100')
         ) AS line(ord, account_id, formula)
         WHERE t.tax_type::text = 'levied'
           AND COALESCE(NULLIF(t.account_id, 0), v_tax_payable_id) IS NOT NULL
           AND NOT EXISTS (
               SELECT 1
                 FROM tax_journal_entry_item_templates AS existing
                WHERE existing.tax_id = t.id
           )
         ORDER BY t.id, line.ord;

        INSERT INTO tax_journal_entry_item_templates
            (created_at, updated_at, tax_id, journal_id, account_id, formula)
        SELECT NOW(), NOW(), t.id, v_journal_id, line.account_id, line.formula
          FROM taxes AS t
         CROSS JOIN LATERAL (VALUES
             (1, t.account_id, 'base * percentage / 100'),
             (2, v_receivable_id, '-base * percentage / 100')
         ) AS line(ord, account_id, formula)
         WHERE t.tax_type::text = 'withholding'
           AND NULLIF(t.account_id, 0) IS NOT NULL
           AND NOT EXISTS (
               SELECT 1
                 FROM tax_journal_entry_item_templates AS existing
                WHERE existing.tax_id = t.id
           )
         ORDER BY t.id, line.ord;
    ELSIF v_tax_payable_id IS NOT NULL THEN
        INSERT INTO tax_journal_entry_item_templates
            (created_at, updated_at, tax_id, journal_id, account_id, formula)
        SELECT NOW(), NOW(), t.id, v_journal_id, line.account_id, line.formula
          FROM taxes AS t
         CROSS JOIN (VALUES
             (1, v_receivable_id, 'base * percentage / 100'),
             (2, v_tax_payable_id, '-base * percentage / 100')
         ) AS line(ord, account_id, formula)
         WHERE t.tax_type::text = 'levied'
           AND NOT EXISTS (
               SELECT 1
                 FROM tax_journal_entry_item_templates AS existing
                WHERE existing.tax_id = t.id
           )
         ORDER BY t.id, line.ord;
    END IF;

    IF v_tax_payable_id IS NOT NULL THEN
        INSERT INTO tax_preferences (id, created_at, updated_at)
        VALUES (1, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING;

        IF pg_get_serial_sequence('tax_preferences', 'id') IS NOT NULL THEN
            PERFORM setval(
                pg_get_serial_sequence('tax_preferences', 'id'),
                GREATEST(COALESCE((SELECT MAX(id) FROM tax_preferences), 1), 1),
                TRUE
            );
        END IF;

        IF NOT EXISTS (
            SELECT 1
              FROM tax_preference_journal_items
             WHERE tax_preferences_id = 1
        ) THEN
            INSERT INTO tax_preference_journal_items
                (created_at, updated_at, tax_preferences_id, journal_id, account_id, formula)
            VALUES
                (NOW(), NOW(), 1, v_journal_id, v_receivable_id, 'base * percentage / 100'),
                (NOW(), NOW(), 1, v_journal_id, v_tax_payable_id, '-base * percentage / 100');
        END IF;
    END IF;
END $$;

ALTER TABLE taxes DROP COLUMN IF EXISTS account_id;
"#;

const DOWN: &str = r#"
DROP TABLE IF EXISTS tax_preference_journal_items;
DROP TABLE IF EXISTS tax_preferences;

ALTER TABLE taxes
    ADD COLUMN IF NOT EXISTS account_id bigint NULL;

ALTER TABLE taxes
    DROP CONSTRAINT IF EXISTS fk_taxes_account_id;

ALTER TABLE taxes
    ADD CONSTRAINT fk_taxes_account_id
        FOREIGN KEY (account_id) REFERENCES accounts(id)
        ON UPDATE CASCADE ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_taxes_account_id ON taxes (account_id);
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
