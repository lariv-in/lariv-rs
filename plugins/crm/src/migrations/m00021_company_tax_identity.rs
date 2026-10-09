use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ADD_GSTIN: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS gstin TEXT";
const ADD_CIN: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS cin TEXT";
const ADD_PAN: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS pan TEXT";
const ADD_PHONE: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS phone TEXT";
const ADD_EMAIL: &str = "ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS email TEXT";

const DROP_GSTIN: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS gstin";
const DROP_CIN: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS cin";
const DROP_PAN: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS pan";
const DROP_PHONE: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS phone";
const DROP_EMAIL: &str = "ALTER TABLE crm_companies DROP COLUMN IF EXISTS email";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, ADD_GSTIN).await?;
        exec_sql(manager, ADD_CIN).await?;
        exec_sql(manager, ADD_PAN).await?;
        exec_sql(manager, ADD_PHONE).await?;
        exec_sql(manager, ADD_EMAIL).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, DROP_GSTIN).await?;
        exec_sql(manager, DROP_CIN).await?;
        exec_sql(manager, DROP_PAN).await?;
        exec_sql(manager, DROP_PHONE).await?;
        exec_sql(manager, DROP_EMAIL).await
    }
}
