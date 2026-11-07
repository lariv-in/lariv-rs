//! Rewrite invoice line `variable_values` into formula measures.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

use lariv_plugin_finance_products::migrations::formula_measure_sql::{
    ENSURE_FORMULA_MEASURE, MIGRATE_INVOICE_VALUES,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, ENSURE_FORMULA_MEASURE).await?;
        exec_sql(manager, MIGRATE_INVOICE_VALUES).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
