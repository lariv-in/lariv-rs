//! Quantity unit `Number` and decimal unit `Seconds` on invoice line measures.

use sea_orm_migration::prelude::*;

use lariv_plugin_finance_products::migrations::formula_measure_sql::{
    REFRESH_FORMULA_FIXED_UNITS, REFRESH_INVOICE_VALUE_UNITS,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(REFRESH_FORMULA_FIXED_UNITS).await?;
        conn.execute_unprepared(REFRESH_INVOICE_VALUE_UNITS).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
