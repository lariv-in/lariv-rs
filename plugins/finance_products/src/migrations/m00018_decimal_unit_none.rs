//! Decimal unit `None`.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

use super::formula_measure_sql::{REFRESH_FORMULA_FIXED_UNITS, REFRESH_PRODUCT_SCHEMA_UNITS};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, REFRESH_FORMULA_FIXED_UNITS).await?;
        exec_sql(manager, REFRESH_PRODUCT_SCHEMA_UNITS).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
