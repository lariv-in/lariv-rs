use crate::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;


const UP_POSTGRES: &[&str] = &[
    "ALTER TABLE crm_converted_leads DROP CONSTRAINT IF EXISTS fk_crm_converted_leads_customer_id",
    "ALTER TABLE crm_converted_leads DROP COLUMN IF EXISTS customer_id",
    "ALTER TABLE crm_companies DROP CONSTRAINT IF EXISTS fk_crm_companies_customer_id",
    "DROP INDEX IF EXISTS uix_crm_companies_customer_id",
    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS customer_id",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                for sql in UP_POSTGRES {
                    exec_sql(manager, sql).await?;
                }
                Ok(())
            }
            sea_orm::DatabaseBackend::Sqlite => {
                exec_sql(manager,
                    "DROP INDEX IF EXISTS uix_crm_companies_customer_id",
                )
                .await?;
                exec_sql(manager,
                    "ALTER TABLE crm_converted_leads DROP COLUMN IF EXISTS customer_id",
                )
                .await?;
                exec_sql(manager,
                    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS customer_id",
                )
                .await
            }
            _ => Ok(()),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
