use sea_orm_migration::prelude::*;

use crate::db::trigram;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        trigram::create_gin_index(db, backend, "customers_name_trgm_idx", "customers", "name")
            .await?;
        trigram::create_gin_index(
            db,
            backend,
            "customers_email_trgm_idx",
            "customers",
            "email",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        trigram::drop_gin_index(db, backend, "customers_email_trgm_idx").await?;
        trigram::drop_gin_index(db, backend, "customers_name_trgm_idx").await?;
        Ok(())
    }
}
