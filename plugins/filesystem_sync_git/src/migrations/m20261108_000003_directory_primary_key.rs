use lariv_core::db::migration_sql::exec_sql;
use sea_orm::DatabaseBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DatabaseBackend::Postgres {
            exec_sql(
                manager,
                r#"
                DO $$
                DECLARE r record;
                BEGIN
                    FOR r IN
                        SELECT conname
                        FROM pg_constraint
                        WHERE conrelid = 'filesystem_git_syncs'::regclass
                          AND contype = 'p'
                    LOOP
                        EXECUTE format(
                            'ALTER TABLE filesystem_git_syncs DROP CONSTRAINT %I',
                            r.conname
                        );
                    END LOOP;
                END $$;
                "#,
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE filesystem_git_syncs DROP COLUMN IF EXISTS id",
            )
            .await?;
            exec_sql(
                manager,
                "DROP INDEX IF EXISTS idx_filesystem_git_syncs_directory_vnode_id",
            )
            .await?;
            return exec_sql(
                manager,
                "ALTER TABLE filesystem_git_syncs ADD PRIMARY KEY (directory_vnode_id)",
            )
            .await;
        }

        exec_sql(
            manager,
            r#"
            CREATE TABLE filesystem_git_syncs__next (
                created_at TEXT,
                updated_at TEXT,
                git_url TEXT NOT NULL,
                git_identifier TEXT NOT NULL,
                last_synced_commit_hash TEXT,
                subpath TEXT NOT NULL,
                directory_vnode_id INTEGER NOT NULL PRIMARY KEY,
                FOREIGN KEY (directory_vnode_id) REFERENCES filesystem_nodes (id)
                    ON UPDATE CASCADE ON DELETE RESTRICT
            );
            INSERT INTO filesystem_git_syncs__next (
                created_at, updated_at, git_url, git_identifier,
                last_synced_commit_hash, subpath, directory_vnode_id
            )
            SELECT
                created_at, updated_at, git_url, git_identifier,
                last_synced_commit_hash, subpath, directory_vnode_id
            FROM filesystem_git_syncs;
            DROP TABLE filesystem_git_syncs;
            ALTER TABLE filesystem_git_syncs__next RENAME TO filesystem_git_syncs;
            "#,
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _backend = manager.get_database_backend();
        Ok(())
    }
}
