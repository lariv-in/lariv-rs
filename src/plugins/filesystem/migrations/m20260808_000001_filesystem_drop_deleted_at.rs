use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn exec(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_connection().get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Soft-deleted parents may still have live children; remove the whole subtree.
        exec(
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

        exec(
            manager,
            "DROP INDEX IF EXISTS idx_filesystem_nodes_deleted_at",
        )
        .await?;
        exec(
            manager,
            "ALTER TABLE filesystem_nodes DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec(
            manager,
            "ALTER TABLE filesystem_nodes ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_filesystem_nodes_deleted_at ON filesystem_nodes (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
