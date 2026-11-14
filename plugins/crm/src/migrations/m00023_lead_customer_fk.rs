//! Point `crm_leads.customer_id` at the party table and drop `contact_id`.
//!
//! Each existing lead becomes an individual customer for its contact when one
//! does not already exist. On a fresh install the address-shaped `customers`
//! table is still in the way, so the party rows live in `crm_customers` until
//! finance renames that table.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

const UP_POSTGRES: &str = r#"
DO $$
DECLARE
    party text;
BEGIN
    IF to_regclass('public.crm_leads') IS NULL THEN
        RETURN;
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'crm_leads' AND column_name = 'customer_id'
    ) THEN
        RETURN;
    END IF;

    ALTER TABLE crm_leads ADD COLUMN customer_id bigint;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'customers' AND column_name = 'is_individual'
    ) THEN
        party := 'customers';
    ELSIF to_regclass('public.crm_customers') IS NOT NULL THEN
        party := 'crm_customers';
    ELSE
        party := NULL;
    END IF;

    IF party IS NOT NULL
       AND EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = 'crm_leads' AND column_name = 'contact_id'
       )
    THEN
        EXECUTE format(
            $ins$
            INSERT INTO %I (created_at, updated_at, is_individual, company_id, individual_id)
            SELECT DISTINCT now(), now(), true, NULL::bigint, l.contact_id
            FROM crm_leads l
            WHERE l.contact_id IS NOT NULL
              AND l.contact_id > 0
              AND NOT EXISTS (
                  SELECT 1 FROM %I c
                  WHERE c.is_individual AND c.individual_id = l.contact_id
              )
            $ins$,
            party, party
        );

        EXECUTE format(
            $upd$
            UPDATE crm_leads AS l
            SET customer_id = c.id
            FROM %I AS c
            WHERE c.is_individual
              AND c.individual_id = l.contact_id
              AND l.customer_id IS NULL
            $upd$,
            party
        );
    END IF;

    DELETE FROM crm_leads WHERE customer_id IS NULL;

    ALTER TABLE crm_leads ALTER COLUMN customer_id SET NOT NULL;
    IF party IS NOT NULL THEN
        EXECUTE format(
            'ALTER TABLE crm_leads
                ADD CONSTRAINT fk_crm_leads_customer_id
                FOREIGN KEY (customer_id) REFERENCES %I (id)
                ON DELETE RESTRICT ON UPDATE CASCADE',
            party
        );
    END IF;
    CREATE INDEX IF NOT EXISTS idx_crm_leads_customer_id ON crm_leads (customer_id);

    ALTER TABLE crm_leads DROP CONSTRAINT IF EXISTS fk_crm_leads_contact_id;
    ALTER TABLE crm_leads DROP COLUMN IF EXISTS contact_id;
END $$;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return exec_sql(manager, UP_POSTGRES).await;
        }
        exec_sql(
            manager,
            "ALTER TABLE crm_leads ADD COLUMN customer_id integer NOT NULL DEFAULT 0",
        )
        .await?;
        exec_sql(manager, "ALTER TABLE crm_leads DROP COLUMN contact_id").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            exec_sql(
                manager,
                r#"
                ALTER TABLE crm_leads DROP CONSTRAINT IF EXISTS fk_crm_leads_customer_id;
                ALTER TABLE crm_leads DROP COLUMN IF EXISTS customer_id;
                ALTER TABLE crm_leads ADD COLUMN IF NOT EXISTS contact_id bigint;
                "#,
            )
            .await?;
            return Ok(());
        }
        exec_sql(manager, "ALTER TABLE crm_leads DROP COLUMN customer_id").await?;
        exec_sql(
            manager,
            "ALTER TABLE crm_leads ADD COLUMN contact_id integer NOT NULL DEFAULT 0",
        )
        .await?;
        Ok(())
    }
}
