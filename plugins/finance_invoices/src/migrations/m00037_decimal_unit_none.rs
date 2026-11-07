//! Decimal unit `None` on invoice line measures.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

use lariv_plugin_finance_products::migrations::formula_measure_sql::{
    REFRESH_FORMULA_FIXED_UNITS, REFRESH_INVOICE_VALUE_UNITS,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, REFRESH_FORMULA_FIXED_UNITS).await?;
        exec_sql(manager, REFRESH_INVOICE_VALUE_UNITS).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
