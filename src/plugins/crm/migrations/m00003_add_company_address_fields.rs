use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ADD_ADDRESS_LINE_1: &str =
    "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS address_line_1 TEXT";
const ADD_ADDRESS_LINE_2: &str =
    "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS address_line_2 TEXT";
const ADD_CITY: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS city TEXT";
const ADD_PINCODE: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS pincode TEXT";
const ADD_STATE: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS state TEXT";
const ADD_WEBSITE: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS website TEXT";

const DROP_ADDRESS_LINE_1: &str =
    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS address_line_1";
const DROP_ADDRESS_LINE_2: &str =
    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS address_line_2";
const DROP_CITY: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS city";
const DROP_PINCODE: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS pincode";
const DROP_STATE: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS state";
const DROP_WEBSITE: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS website";

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
        execute(manager, ADD_ADDRESS_LINE_1).await?;
        execute(manager, ADD_ADDRESS_LINE_2).await?;
        execute(manager, ADD_CITY).await?;
        execute(manager, ADD_PINCODE).await?;
        execute(manager, ADD_STATE).await?;
        execute(manager, ADD_WEBSITE).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, DROP_ADDRESS_LINE_1).await?;
        execute(manager, DROP_ADDRESS_LINE_2).await?;
        execute(manager, DROP_CITY).await?;
        execute(manager, DROP_PINCODE).await?;
        execute(manager, DROP_STATE).await?;
        execute(manager, DROP_WEBSITE).await
    }
}
