//! Put `crm_customers` back after `m00010` drops an empty copy.
//!
//! `m00010` removes `crm_customers` while the legacy address-shaped `customers`
//! table is still present. CRM's `m00022_create_customers` used to recreate it.
//! A deployment that installs customer without CRM still needs the party table
//! before invoices point `customer_id` at it. When `customers` is already that
//! party table, this leaves it in place. `m00014` drops `state`.

use crate::foreign_keys::{FK_CRM_CUSTOMERS_COMPANY, FK_CRM_CUSTOMERS_INDIVIDUAL};
use lariv_core::db::migration_sql::{self, exec_sql};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $recreate$
BEGIN
    IF to_regclass('public.crm_customers') IS NOT NULL THEN
        ALTER TABLE crm_customers ADD COLUMN IF NOT EXISTS state text;
        RETURN;
    END IF;

    IF to_regclass('public.customers') IS NOT NULL
       AND EXISTS (
            SELECT 1
              FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'customers'
               AND column_name = 'is_individual'
       ) THEN
        ALTER TABLE customers ADD COLUMN IF NOT EXISTS state text;
        RETURN;
    END IF;

    CREATE TABLE crm_customers (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        is_individual boolean NOT NULL,
        company_id bigint,
        individual_id bigint,
        state text,
        CONSTRAINT ck_crm_customers_party CHECK (
            (is_individual AND individual_id IS NOT NULL AND company_id IS NULL)
            OR (NOT is_individual AND company_id IS NOT NULL AND individual_id IS NULL)
        )
    );
    CREATE UNIQUE INDEX IF NOT EXISTS ux_crm_customers_company
        ON crm_customers (company_id) WHERE NOT is_individual;
    CREATE UNIQUE INDEX IF NOT EXISTS ux_crm_customers_individual
        ON crm_customers (individual_id) WHERE is_individual;
END
$recreate$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            recreate_sqlite(manager).await?;
        } else {
            exec_sql(manager, UP).await?;
        }
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[FK_CRM_CUSTOMERS_COMPANY, FK_CRM_CUSTOMERS_INDIVIDUAL],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn recreate_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if table_exists(manager, "crm_customers").await? {
        if column_exists(manager, "crm_customers", "state").await? {
            return Ok(());
        }
        return exec_sql(manager, "ALTER TABLE crm_customers ADD COLUMN state text").await;
    }
    if table_exists(manager, "customers").await?
        && column_exists(manager, "customers", "is_individual").await?
    {
        if column_exists(manager, "customers", "state").await? {
            return Ok(());
        }
        return exec_sql(manager, "ALTER TABLE customers ADD COLUMN state text").await;
    }
    exec_sql(
        manager,
        r#"CREATE TABLE crm_customers (
            id integer PRIMARY KEY AUTOINCREMENT,
            created_at text,
            updated_at text,
            is_individual integer NOT NULL,
            company_id integer,
            individual_id integer,
            state text,
            CONSTRAINT ck_crm_customers_party CHECK (
                (is_individual AND individual_id IS NOT NULL AND company_id IS NULL)
                OR (NOT is_individual AND company_id IS NOT NULL AND individual_id IS NULL)
            )
        )"#,
    )
    .await?;
    exec_sql(
        manager,
        "CREATE UNIQUE INDEX IF NOT EXISTS ux_crm_customers_company
            ON crm_customers (company_id) WHERE NOT is_individual",
    )
    .await?;
    exec_sql(
        manager,
        "CREATE UNIQUE INDEX IF NOT EXISTS ux_crm_customers_individual
            ON crm_customers (individual_id) WHERE is_individual",
    )
    .await
}

async fn table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
        ))
        .await?;
    Ok(row.is_some())
}

async fn column_exists(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = '{column}'"),
        ))
        .await?;
    Ok(row.is_some())
}
