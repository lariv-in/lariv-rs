//! A customer points at one company or one individual contact.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS crm_customers (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        is_individual boolean NOT NULL,
        company_id bigint,
        individual_id bigint,
        CONSTRAINT fk_crm_customers_company_id
            FOREIGN KEY (company_id) REFERENCES crm_companies(id)
            ON DELETE RESTRICT ON UPDATE CASCADE,
        CONSTRAINT fk_crm_customers_individual_id
            FOREIGN KEY (individual_id) REFERENCES crm_contacts(id)
            ON DELETE RESTRICT ON UPDATE CASCADE,
        CONSTRAINT ck_crm_customers_party CHECK (
            (is_individual AND individual_id IS NOT NULL AND company_id IS NULL)
            OR (NOT is_individual AND company_id IS NOT NULL AND individual_id IS NULL)
        )
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS ux_crm_customers_company
        ON crm_customers (company_id) WHERE NOT is_individual",
    "CREATE UNIQUE INDEX IF NOT EXISTS ux_crm_customers_individual
        ON crm_customers (individual_id) WHERE is_individual",
];

const DOWN: &[&str] = &["DROP TABLE IF EXISTS crm_customers CASCADE"];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in UP {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in DOWN {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }
}
