use sea_orm_migration::prelude::*;

use crate::db::trigram;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        trigram::create_gin_index(db, backend, "products_name_trgm_idx", "products", "name")
            .await?;
        trigram::create_gin_index(
            db,
            backend,
            "products_reference_trgm_idx",
            "products",
            "reference",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        trigram::drop_gin_index(db, backend, "products_reference_trgm_idx").await?;
        trigram::drop_gin_index(db, backend, "products_name_trgm_idx").await?;
        Ok(())
    }
}
