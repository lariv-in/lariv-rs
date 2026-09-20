use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ADD_PROBATIONS_UPDATED_AT: &str =
    "ALTER TABLE hr_probations ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ";
const ADD_EMPLOYEES_UPDATED_AT: &str =
    "ALTER TABLE hr_employees ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ";

const DROP_PROBATIONS_UPDATED_AT: &str =
    "ALTER TABLE hr_probations DROP COLUMN IF EXISTS updated_at";
const DROP_EMPLOYEES_UPDATED_AT: &str = "ALTER TABLE hr_employees DROP COLUMN IF EXISTS updated_at";

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

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, ADD_PROBATIONS_UPDATED_AT).await?;
        execute(manager, ADD_EMPLOYEES_UPDATED_AT).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, DROP_EMPLOYEES_UPDATED_AT).await?;
        execute(manager, DROP_PROBATIONS_UPDATED_AT).await
    }
}
