use sea_orm_migration::prelude::*;

use crate::db::trigram;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        trigram::recreate_gin_index(db, backend, "skills_name_trgm_idx", "skills", "name").await?;
        trigram::recreate_gin_index(
            db,
            backend,
            "skills_description_trgm_idx",
            "skills",
            "description",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
