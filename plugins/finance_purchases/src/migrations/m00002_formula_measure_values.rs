//! Rewrite purchase line `variable_values` into formula measures.

use sea_orm_migration::prelude::*;

use lariv_plugin_finance_products::migrations::formula_measure_sql::{
    ENSURE_FORMULA_MEASURE, MIGRATE_PURCHASE_VALUES,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(ENSURE_FORMULA_MEASURE).await?;
        conn.execute_unprepared(MIGRATE_PURCHASE_VALUES).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
