//! Product variable schema and price formula.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS variables text NOT NULL DEFAULT '{}'",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS price_formula text NOT NULL DEFAULT ''",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "ALTER TABLE products DROP COLUMN IF EXISTS price_formula",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE products DROP COLUMN IF EXISTS variables",
        )
        .await?;
        Ok(())
    }
}
