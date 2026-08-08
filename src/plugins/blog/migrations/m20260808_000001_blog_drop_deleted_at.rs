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
        exec(
            manager,
            r#"DELETE FROM p_blog_tags WHERE blog_id IN (SELECT id FROM blogs WHERE deleted_at IS NOT NULL)
               OR blog_tag_id IN (SELECT id FROM blog_tags WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        exec(manager, "DELETE FROM blogs WHERE deleted_at IS NOT NULL").await?;
        exec(manager, "DELETE FROM blog_tags WHERE deleted_at IS NOT NULL").await?;

        exec(manager, "DROP INDEX IF EXISTS idx_blogs_deleted_at").await?;
        exec(manager, "DROP INDEX IF EXISTS idx_blog_tags_deleted_at").await?;

        exec(manager, "ALTER TABLE blogs DROP COLUMN IF EXISTS deleted_at").await?;
        exec(
            manager,
            "ALTER TABLE blog_tags DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec(
            manager,
            "ALTER TABLE blog_tags ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec(
            manager,
            "ALTER TABLE blogs ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_blog_tags_deleted_at ON blog_tags (deleted_at)",
        )
        .await?;
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_blogs_deleted_at ON blogs (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
