//! `formula_measure_kind` and accepted units on product variable definitions.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

use super::formula_measure_sql::{ENSURE_FORMULA_MEASURE, MIGRATE_PRODUCT_SCHEMAS};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, ENSURE_FORMULA_MEASURE).await?;
        exec_sql(manager, MIGRATE_PRODUCT_SCHEMAS).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "DROP FUNCTION IF EXISTS lariv_migrate_formula_values(jsonb, jsonb, jsonb)",
        )
        .await?;
        exec_sql(
            manager,
            "DROP FUNCTION IF EXISTS lariv_migrate_formula_schema(jsonb)",
        )
        .await?;
        exec_sql(
            manager,
            "DROP FUNCTION IF EXISTS lariv_formula_accepted_unit(text)",
        )
        .await?;
        exec_sql(manager, "DROP TYPE IF EXISTS formula_measure").await?;
        exec_sql(manager, "DROP TYPE IF EXISTS formula_measure_kind").await?;
        Ok(())
    }
}
