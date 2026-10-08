use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // `exec_sql` uses a prepared statement, which accepts one command.
        exec_sql(
            manager,
            "ALTER TABLE journals DROP COLUMN IF EXISTS journal_type",
        )
        .await?;
        exec_sql(manager, "DROP TYPE IF EXISTS journal_type").await?;
        exec_sql(manager, r#"DROP TYPE IF EXISTS "JournalType""#).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            r#"
DO $do$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'journal_type') THEN
    CREATE TYPE journal_type AS ENUM ('Credit', 'Debit');
  END IF;
END
$do$
"#,
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE journals ADD COLUMN IF NOT EXISTS journal_type journal_type NOT NULL DEFAULT 'Debit'",
        )
        .await
    }
}
