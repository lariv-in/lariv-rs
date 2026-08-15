use sea_orm_migration::prelude::*;

use crate::db::trigram;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        trigram::create_gin_index(
            db,
            backend,
            "draft_invoices_number_trgm_idx",
            "draft_invoices",
            "number",
        )
        .await?;
        trigram::create_gin_index(
            db,
            backend,
            "draft_invoices_reference_trgm_idx",
            "draft_invoices",
            "reference",
        )
        .await?;
        trigram::create_gin_index(
            db,
            backend,
            "posted_invoices_number_trgm_idx",
            "posted_invoices",
            "number",
        )
        .await?;
        trigram::create_gin_index(
            db,
            backend,
            "posted_invoices_reference_trgm_idx",
            "posted_invoices",
            "reference",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        trigram::drop_gin_index(db, backend, "posted_invoices_reference_trgm_idx").await?;
        trigram::drop_gin_index(db, backend, "posted_invoices_number_trgm_idx").await?;
        trigram::drop_gin_index(db, backend, "draft_invoices_reference_trgm_idx").await?;
        trigram::drop_gin_index(db, backend, "draft_invoices_number_trgm_idx").await?;
        Ok(())
    }
}
