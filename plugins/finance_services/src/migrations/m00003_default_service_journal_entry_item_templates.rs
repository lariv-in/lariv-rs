//! Seed default service journal templates from the old invoice GL preferences.
//!
//! Each service posts its own sale: receivable debits `unit_rate` and revenue
//! credits it back. Cost of sales is not copied onto services. Invoice posting
//! expands every service template, so those lines would be booked once per
//! service. The same two sales lines are stored on service preferences so a
//! new service opens with that default. Formulas are stored plainly: numeric
//! constants are wrapped in `decimal(...)` when a formula is evaluated, so a
//! credit line is just `-unit_rate`.

use lariv_core::db::migration_sql::{
    self, FK_SERVICE_PREFERENCE_JOURNAL_ACCOUNT, FK_SERVICE_PREFERENCE_JOURNAL_JOURNAL,
    FK_SERVICE_PREFERENCES_COST_ACCOUNT, FK_SERVICE_PREFERENCES_CREDIT_ACCOUNT,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE IF NOT EXISTS service_preference_journal_items (
    id bigserial PRIMARY KEY,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    service_preferences_id bigint NOT NULL
        REFERENCES service_preferences(id) ON UPDATE CASCADE ON DELETE CASCADE,
    journal_id bigint NOT NULL,
    account_id bigint NOT NULL,
    formula text NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_service_preference_journal_items_prefs_id
    ON service_preference_journal_items (service_preferences_id);
CREATE INDEX IF NOT EXISTS idx_service_preference_journal_items_journal_id
    ON service_preference_journal_items (journal_id);
CREATE INDEX IF NOT EXISTS idx_service_preference_journal_items_account_id
    ON service_preference_journal_items (account_id);

DO $$
DECLARE
    v_journal_id bigint;
    v_receivable_id bigint;
    v_revenue_id bigint;
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

    INSERT INTO service_journal_entry_item_templates
        (created_at, updated_at, service_id, journal_id, account_id, formula)
    SELECT NOW(), NOW(), s.id, v_journal_id, line.account_id, line.formula
      FROM services AS s
     CROSS JOIN (VALUES
         (1, v_receivable_id, 'unit_rate'),
         (2, v_revenue_id, '-unit_rate')
     ) AS line(ord, account_id, formula)
     WHERE NOT EXISTS (
         SELECT 1
           FROM service_journal_entry_item_templates AS existing
          WHERE existing.service_id = s.id
     )
     ORDER BY s.id, line.ord;

    INSERT INTO service_preferences (id, created_at, updated_at)
    VALUES (1, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    IF pg_get_serial_sequence('service_preferences', 'id') IS NOT NULL THEN
        PERFORM setval(
            pg_get_serial_sequence('service_preferences', 'id'),
            GREATEST(COALESCE((SELECT MAX(id) FROM service_preferences), 1), 1),
            TRUE
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM service_preference_journal_items
         WHERE service_preferences_id = 1
    ) THEN
        INSERT INTO service_preference_journal_items
            (created_at, updated_at, service_preferences_id, journal_id, account_id, formula)
        VALUES
            (NOW(), NOW(), 1, v_journal_id, v_receivable_id, 'unit_rate'),
            (NOW(), NOW(), 1, v_journal_id, v_revenue_id, '-unit_rate');
    END IF;
END $$;

ALTER TABLE service_preferences
    DROP COLUMN IF EXISTS cost_of_sales_account_id,
    DROP COLUMN IF EXISTS cost_credit_account_id;
"#;

const DOWN: &str = r#"
DROP TABLE IF EXISTS service_preference_journal_items;

ALTER TABLE service_preferences
    ADD COLUMN IF NOT EXISTS cost_of_sales_account_id bigint NULL,
    ADD COLUMN IF NOT EXISTS cost_credit_account_id bigint NULL;

ALTER TABLE service_preferences
    DROP CONSTRAINT IF EXISTS service_preferences_cost_of_sales_account_id_fkey,
    DROP CONSTRAINT IF EXISTS service_preferences_cost_credit_account_id_fkey,
    DROP CONSTRAINT IF EXISTS fk_service_preferences_cost_of_sales_account_id,
    DROP CONSTRAINT IF EXISTS fk_service_preferences_cost_credit_account_id;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[
                FK_SERVICE_PREFERENCE_JOURNAL_JOURNAL,
                FK_SERVICE_PREFERENCE_JOURNAL_ACCOUNT,
            ],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[
                FK_SERVICE_PREFERENCES_COST_ACCOUNT,
                FK_SERVICE_PREFERENCES_CREDIT_ACCOUNT,
            ],
        )
        .await?;
        Ok(())
    }
}
