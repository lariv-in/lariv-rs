use lariv_core::db::migration_sql::exec_sql;
use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Soft-deleted parents may still have live children; remove the whole subtree.
        exec_sql(
            manager,
            r#"
DELETE FROM filesystem_nodes
WHERE id IN (
    WITH RECURSIVE doomed AS (
        SELECT id FROM filesystem_nodes WHERE deleted_at IS NOT NULL
        UNION ALL
        SELECT n.id FROM filesystem_nodes n
        INNER JOIN doomed d ON n.parent_id = d.id
    )
    SELECT id FROM doomed
)
"#,
        )
        .await?;

        exec_sql(
            manager,
            "DROP INDEX IF EXISTS idx_filesystem_nodes_deleted_at",
        )
        .await?;
        // SQLite accepts DROP COLUMN, but not DROP COLUMN IF EXISTS.
        let drop_deleted_at = match manager.get_database_backend() {
            DbBackend::Postgres => "DROP COLUMN IF EXISTS deleted_at",
            _ => "DROP COLUMN deleted_at",
        };
        exec_sql(
            manager,
            &format!("ALTER TABLE filesystem_nodes {drop_deleted_at}"),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let add_deleted_at = match manager.get_database_backend() {
            DbBackend::Postgres => "ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
            _ => "ADD COLUMN deleted_at timestamp_with_timezone_text",
        };
        exec_sql(
            manager,
            &format!("ALTER TABLE filesystem_nodes {add_deleted_at}"),
        )
        .await?;
        exec_sql(manager,
            "CREATE INDEX IF NOT EXISTS idx_filesystem_nodes_deleted_at ON filesystem_nodes (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
