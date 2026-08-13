use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_connection().get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

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
                    execute(manager, sql).await?;
                }
                Ok(())
            }
            sea_orm::DatabaseBackend::Sqlite => {
                execute(
                    manager,
                    "DROP INDEX IF EXISTS uix_crm_companies_customer_id",
                )
                .await?;
                execute(
                    manager,
                    "ALTER TABLE crm_converted_leads DROP COLUMN IF EXISTS customer_id",
                )
                .await?;
                execute(
                    manager,
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
