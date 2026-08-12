use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'JournalType') THEN
    ALTER TABLE journals ALTER COLUMN journal_type TYPE TEXT USING journal_type::TEXT;
    UPDATE journals SET journal_type = 'Debit' WHERE journal_type = 'General';
    DROP TYPE "JournalType";
    CREATE TYPE "JournalType" AS ENUM ('Credit', 'Debit');
    ALTER TABLE journals ALTER COLUMN journal_type TYPE "JournalType" USING journal_type::"JournalType";
  ELSIF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'journal_type')
    AND EXISTS (
      SELECT 1 FROM pg_enum e
      JOIN pg_type t ON e.enumtypid = t.oid
      WHERE t.typname = 'journal_type' AND e.enumlabel = 'General'
    )
    AND NOT EXISTS (
      SELECT 1 FROM pg_enum e
      JOIN pg_type t ON e.enumtypid = t.oid
      WHERE t.typname = 'journal_type' AND e.enumlabel = 'Debit'
    )
  THEN
    ALTER TABLE journals ALTER COLUMN journal_type TYPE TEXT USING journal_type::TEXT;
    UPDATE journals SET journal_type = 'Debit' WHERE journal_type = 'General';
    DROP TYPE journal_type;
    CREATE TYPE journal_type AS ENUM ('Credit', 'Debit');
    ALTER TABLE journals ALTER COLUMN journal_type TYPE journal_type USING journal_type::journal_type;
  END IF;
END;
$$;
"#;

const DOWN: &str = r#"
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'JournalType') THEN
    ALTER TABLE journals ALTER COLUMN journal_type TYPE TEXT USING journal_type::TEXT;
    UPDATE journals SET journal_type = 'General' WHERE journal_type IN ('Credit', 'Debit');
    DROP TYPE "JournalType";
    CREATE TYPE "JournalType" AS ENUM ('General');
    ALTER TABLE journals ALTER COLUMN journal_type TYPE "JournalType" USING journal_type::"JournalType";
  ELSIF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'journal_type')
    AND EXISTS (
      SELECT 1 FROM pg_enum e
      JOIN pg_type t ON e.enumtypid = t.oid
      WHERE t.typname = 'journal_type' AND e.enumlabel IN ('Credit', 'Debit')
    )
    AND NOT EXISTS (
      SELECT 1 FROM pg_enum e
      JOIN pg_type t ON e.enumtypid = t.oid
      WHERE t.typname = 'journal_type' AND e.enumlabel = 'General'
    )
  THEN
    ALTER TABLE journals ALTER COLUMN journal_type TYPE TEXT USING journal_type::TEXT;
    UPDATE journals SET journal_type = 'General' WHERE journal_type IN ('Credit', 'Debit');
    DROP TYPE journal_type;
    CREATE TYPE journal_type AS ENUM ('General');
    ALTER TABLE journals ALTER COLUMN journal_type TYPE journal_type USING journal_type::journal_type;
  END IF;
END;
$$;
"#;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
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
        execute(manager, UP).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, DOWN).await
    }
}
