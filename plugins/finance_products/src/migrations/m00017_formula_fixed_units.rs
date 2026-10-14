//! Quantity unit `Number` and decimal unit `Seconds`.

use sea_orm_migration::prelude::*;

use super::formula_measure_sql::{REFRESH_FORMULA_FIXED_UNITS, REFRESH_PRODUCT_SCHEMA_UNITS};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(REFRESH_FORMULA_FIXED_UNITS).await?;
        conn.execute_unprepared(REFRESH_PRODUCT_SCHEMA_UNITS).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
