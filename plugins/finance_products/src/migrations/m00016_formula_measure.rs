//! `formula_measure_kind` and accepted units on product variable definitions.

use sea_orm_migration::prelude::*;

use super::formula_measure_sql::{ENSURE_FORMULA_MEASURE, MIGRATE_PRODUCT_SCHEMAS};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(ENSURE_FORMULA_MEASURE).await?;
        conn.execute_unprepared(MIGRATE_PRODUCT_SCHEMAS).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "DROP FUNCTION IF EXISTS lariv_migrate_formula_values(jsonb, jsonb, jsonb)",
        )
        .await?;
        conn.execute_unprepared("DROP FUNCTION IF EXISTS lariv_migrate_formula_schema(jsonb)")
            .await?;
        conn.execute_unprepared("DROP FUNCTION IF EXISTS lariv_formula_accepted_unit(text)")
            .await?;
        conn.execute_unprepared("DROP TYPE IF EXISTS formula_measure")
            .await?;
        conn.execute_unprepared("DROP TYPE IF EXISTS formula_measure_kind")
            .await?;
        Ok(())
    }
}
