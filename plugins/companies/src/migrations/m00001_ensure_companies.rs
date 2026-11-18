//! Repeat company-table setup under a name contacts never recorded.
//!
//! Contacts shipped `m00001_create_companies`, which created `crm_companies`.
//! SeaORM treats that version as applied, so the companies plugin's file of the
//! same name does not run and `companies` is still missing when memberships
//! are created.

use sea_orm_migration::prelude::*;

use super::m00001_create_companies::ensure_companies;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_companies(manager).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
