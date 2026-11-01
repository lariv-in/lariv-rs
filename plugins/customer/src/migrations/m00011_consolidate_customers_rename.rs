//! Idempotent follow-up to `m00010`: copy `crm_customers` into `customers`.
//!
//! Checkpoint restores can revive a populated `crm_customers` after `m00010` is
//! already recorded in `seaql_migrations`, so that migration never runs again.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

use super::consolidate_crm_customers;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in consolidate_crm_customers::UP {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
