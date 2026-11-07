use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // `exec_sql` uses a prepared statement, which accepts one command.
        // SQLite accepts DROP COLUMN, but not DROP COLUMN IF EXISTS.
        let drop_journal_type = if is_postgres(manager) {
            "ALTER TABLE journals DROP COLUMN IF EXISTS journal_type"
        } else {
            "ALTER TABLE journals DROP COLUMN journal_type"
        };
        exec_sql(manager, drop_journal_type).await?;
        if is_postgres(manager) {
            exec_sql(manager, "DROP TYPE IF EXISTS journal_type").await?;
            exec_sql(manager, r#"DROP TYPE IF EXISTS "JournalType""#).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !is_postgres(manager) {
            return exec_sql(
                manager,
                "ALTER TABLE journals ADD COLUMN journal_type text NOT NULL DEFAULT 'Debit'",
            )
            .await;
        }
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
