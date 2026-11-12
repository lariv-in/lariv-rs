//! Point converted and failed leads at `customers`.
//!
//! A customer is one party, a company or an individual. Converted rows drop
//! the paired company and contact columns and keep that customer.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

const UP_POSTGRES: &str = r#"
DO $$
BEGIN
    IF to_regclass('public.crm_converted_leads') IS NOT NULL
       AND NOT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'crm_converted_leads'
              AND column_name = 'customer_id'
       )
    THEN
        ALTER TABLE crm_converted_leads ADD COLUMN customer_id bigint;

        IF to_regclass('public.crm_leads') IS NOT NULL
           AND EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'crm_leads'
                  AND column_name = 'customer_id'
           )
        THEN
            UPDATE crm_converted_leads AS c
            SET customer_id = l.customer_id
            FROM crm_leads AS l
            WHERE c.lead_id = l.id
              AND l.customer_id IS NOT NULL
              AND c.customer_id IS NULL;
        END IF;

        DELETE FROM crm_converted_leads WHERE customer_id IS NULL;

        ALTER TABLE crm_converted_leads ALTER COLUMN customer_id SET NOT NULL;
        ALTER TABLE crm_converted_leads
            ADD CONSTRAINT fk_crm_converted_leads_customer_id
            FOREIGN KEY (customer_id) REFERENCES customers(id)
            ON DELETE RESTRICT ON UPDATE CASCADE;
        CREATE INDEX IF NOT EXISTS idx_crm_converted_leads_customer_id
            ON crm_converted_leads (customer_id);

        ALTER TABLE crm_converted_leads DROP CONSTRAINT IF EXISTS fk_crm_converted_leads_company_id;
        ALTER TABLE crm_converted_leads DROP CONSTRAINT IF EXISTS fk_crm_converted_leads_contact_id;
        ALTER TABLE crm_converted_leads DROP COLUMN IF EXISTS company_id;
        ALTER TABLE crm_converted_leads DROP COLUMN IF EXISTS contact_id;
    END IF;

    IF to_regclass('public.crm_failed_leads') IS NOT NULL
       AND NOT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'crm_failed_leads'
              AND column_name = 'customer_id'
       )
    THEN
        ALTER TABLE crm_failed_leads ADD COLUMN customer_id bigint;

        IF to_regclass('public.crm_leads') IS NOT NULL
           AND EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'crm_leads'
                  AND column_name = 'customer_id'
           )
        THEN
            UPDATE crm_failed_leads AS f
            SET customer_id = l.customer_id
            FROM crm_leads AS l
            WHERE f.lead_id = l.id
              AND l.customer_id IS NOT NULL
              AND f.customer_id IS NULL;
        END IF;

        DELETE FROM crm_failed_leads WHERE customer_id IS NULL;

        ALTER TABLE crm_failed_leads ALTER COLUMN customer_id SET NOT NULL;
        ALTER TABLE crm_failed_leads
            ADD CONSTRAINT fk_crm_failed_leads_customer_id
            FOREIGN KEY (customer_id) REFERENCES customers(id)
            ON DELETE RESTRICT ON UPDATE CASCADE;
        CREATE INDEX IF NOT EXISTS idx_crm_failed_leads_customer_id
            ON crm_failed_leads (customer_id);
    END IF;
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
            "ALTER TABLE crm_converted_leads ADD COLUMN customer_id integer NOT NULL DEFAULT 0",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE crm_converted_leads DROP COLUMN company_id",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE crm_converted_leads DROP COLUMN contact_id",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE crm_failed_leads ADD COLUMN customer_id integer NOT NULL DEFAULT 0",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            exec_sql(
                manager,
                r#"
                ALTER TABLE crm_converted_leads DROP CONSTRAINT IF EXISTS fk_crm_converted_leads_customer_id;
                ALTER TABLE crm_converted_leads DROP COLUMN IF EXISTS customer_id;
                ALTER TABLE crm_converted_leads ADD COLUMN IF NOT EXISTS company_id bigint;
                ALTER TABLE crm_converted_leads ADD COLUMN IF NOT EXISTS contact_id bigint;
                ALTER TABLE crm_failed_leads DROP CONSTRAINT IF EXISTS fk_crm_failed_leads_customer_id;
                ALTER TABLE crm_failed_leads DROP COLUMN IF EXISTS customer_id;
                "#,
            )
            .await?;
            return Ok(());
        }
        exec_sql(
            manager,
            "ALTER TABLE crm_converted_leads DROP COLUMN customer_id",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE crm_converted_leads ADD COLUMN company_id integer NOT NULL DEFAULT 0",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE crm_converted_leads ADD COLUMN contact_id integer NOT NULL DEFAULT 0",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE crm_failed_leads DROP COLUMN customer_id",
        )
        .await?;
        Ok(())
    }
}
