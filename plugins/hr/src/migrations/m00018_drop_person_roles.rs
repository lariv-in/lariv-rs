//! Applicant, employee, and ex-employee are records, not roles.
//! Move anyone still stored under those names to `unassigned`.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "UPDATE users SET role = 'unassigned' WHERE role IN ('applicant', 'employee', 'ex-employee')",
        )
        .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
