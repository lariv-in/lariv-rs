//! Payment accounts live on the shared payment journal template.
//!
//! A receipt debited `payment_preferences.payment_account_id` for the cash
//! received and credited receivable for the same amount, on the invoice
//! journal. Withholding tax templates credit receivable for the withheld
//! amount, so the payment lines are `amount - withholding` and
//! `withholding - amount`. They sum to zero whether or not tax is withheld.
//! Those two lines are copied here before the account columns go away. Registered before
//! `m00041_drop_invoice_gl_preferences` so `account_receivable_id` and
//! `journal_id` can still be read. When a preference is blank, the latest
//! posted invoice or payment supplies the same account.

use lariv_core::db::migration_sql::{
    self, FK_PAYMENT_JOURNAL_ITEM_ACCOUNT, FK_PAYMENT_JOURNAL_ITEM_JOURNAL,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const SEED: &str = r#"
DO $$
DECLARE
    v_journal_id bigint;
    v_payment_account_id bigint;
    v_receivable_id bigint;
BEGIN
    IF to_regclass('public.payment_journal_entry_item_templates') IS NULL
       OR to_regclass('public.payment_preferences') IS NULL THEN
        RETURN;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM payment_preferences WHERE id = 1) THEN
        RETURN;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM payment_journal_entry_item_templates
         WHERE payment_preferences_id = 1
    ) THEN
        RETURN;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'payment_preferences'
           AND column_name = 'payment_account_id'
    ) THEN
        SELECT NULLIF(payment_account_id, 0)
          INTO v_payment_account_id
          FROM payment_preferences
         WHERE id = 1;
    END IF;

    IF v_payment_account_id IS NULL
       AND to_regclass('public.payments') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'payments'
              AND column_name = 'account_id'
       )
    THEN
        SELECT NULLIF(account_id, 0)
          INTO v_payment_account_id
          FROM payments
         WHERE account_id <> 0
         ORDER BY id DESC
         LIMIT 1;
    END IF;

    IF to_regclass('public.invoice_preferences') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'invoice_preferences'
              AND column_name = 'account_receivable_id'
       )
    THEN
        SELECT NULLIF(account_receivable_id, 0)
          INTO v_receivable_id
          FROM invoice_preferences
         WHERE id = 1;
    END IF;

    IF v_receivable_id IS NULL
       AND to_regclass('public.posted_invoices') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'posted_invoices'
              AND column_name = 'account_receivable_id'
       )
    THEN
        SELECT NULLIF(account_receivable_id, 0)
          INTO v_receivable_id
          FROM posted_invoices
         WHERE account_receivable_id <> 0
         ORDER BY id DESC
         LIMIT 1;
    END IF;

    IF to_regclass('public.invoice_preferences') IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'invoice_preferences'
              AND column_name = 'journal_id'
       )
    THEN
        SELECT NULLIF(journal_id, 0)
          INTO v_journal_id
          FROM invoice_preferences
         WHERE id = 1;
    END IF;

    IF v_journal_id IS NULL
       AND to_regclass('public.posted_invoices') IS NOT NULL
    THEN
        SELECT NULLIF(journal_id, 0)
          INTO v_journal_id
          FROM posted_invoices
         WHERE journal_id <> 0
         ORDER BY id DESC
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

    IF v_journal_id IS NULL OR v_payment_account_id IS NULL OR v_receivable_id IS NULL THEN
        RETURN;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM journals WHERE id = v_journal_id) THEN
        RETURN;
    END IF;
    IF to_regclass('public.accounts') IS NULL
       OR NOT EXISTS (
           SELECT 1
             FROM accounts
            WHERE id = v_payment_account_id
              AND is_group = false
       )
       OR NOT EXISTS (
           SELECT 1
             FROM accounts
            WHERE id = v_receivable_id
              AND is_group = false
       )
    THEN
        RETURN;
    END IF;

    INSERT INTO payment_journal_entry_item_templates
        (created_at, updated_at, payment_preferences_id, journal_id, account_id, formula)
    VALUES
        (NOW(), NOW(), 1, v_journal_id, v_payment_account_id, 'amount - withholding'),
        (NOW(), NOW(), 1, v_journal_id, v_receivable_id, 'withholding - amount');
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS payment_journal_entry_item_templates (
                id bigserial PRIMARY KEY,
                created_at timestamptz NULL,
                updated_at timestamptz NULL,
                payment_preferences_id bigint NOT NULL
                    REFERENCES payment_preferences(id) ON UPDATE CASCADE ON DELETE CASCADE,
                journal_id bigint NOT NULL,
                account_id bigint NOT NULL,
                formula text NOT NULL,
                receivable boolean NOT NULL DEFAULT false
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_payment_journal_entry_item_templates_prefs_id ON payment_journal_entry_item_templates (payment_preferences_id)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_payment_journal_entry_item_templates_journal_id ON payment_journal_entry_item_templates (journal_id)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_payment_journal_entry_item_templates_account_id ON payment_journal_entry_item_templates (account_id)",
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[
                FK_PAYMENT_JOURNAL_ITEM_JOURNAL,
                FK_PAYMENT_JOURNAL_ITEM_ACCOUNT,
            ],
        )
        .await?;
        conn.execute_unprepared(SEED).await?;
        conn.execute_unprepared(
            "ALTER TABLE payment_preferences DROP COLUMN IF EXISTS payment_account_id",
        )
        .await?;
        conn.execute_unprepared("ALTER TABLE payments DROP COLUMN IF EXISTS account_id")
            .await?;
        conn.execute_unprepared("ALTER TABLE payment_batches DROP COLUMN IF EXISTS account_id")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS payment_journal_entry_item_templates")
            .await?;
        conn.execute_unprepared(
            "ALTER TABLE payment_preferences ADD COLUMN IF NOT EXISTS payment_account_id bigint NULL",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE payments ADD COLUMN IF NOT EXISTS account_id bigint NOT NULL DEFAULT 0",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE payment_batches ADD COLUMN IF NOT EXISTS account_id bigint NOT NULL DEFAULT 0",
        )
        .await?;
        Ok(())
    }
}
