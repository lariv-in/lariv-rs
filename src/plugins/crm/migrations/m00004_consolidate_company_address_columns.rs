use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Legacy Go/early schemas used `address_line1` / `address_line2`.
/// Fresh creates already use `address_line_1` / `address_line_2` — skip when absent.
///
/// Column refs must live inside the `IF` body: Postgres plans a plain
/// `UPDATE … AND EXISTS (information_schema…)` and still fails if the
/// legacy column is missing.
const UP_POSTGRES: &str = r#"
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'crm_companies'
      AND column_name = 'address_line1'
  ) THEN
    UPDATE crm_companies
    SET address_line_1 = address_line1
    WHERE address_line_1 IS NULL
      AND address_line1 IS NOT NULL;
    ALTER TABLE crm_companies DROP COLUMN address_line1;
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'crm_companies'
      AND column_name = 'address_line2'
  ) THEN
    UPDATE crm_companies
    SET address_line_2 = address_line2
    WHERE address_line_2 IS NULL
      AND address_line2 IS NOT NULL;
    ALTER TABLE crm_companies DROP COLUMN address_line2;
  END IF;
END $$;
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
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => execute(manager, UP_POSTGRES).await,
            // Fresh SQLite creates never had the legacy names; DROP IF EXISTS is enough.
            sea_orm::DatabaseBackend::Sqlite => {
                execute(
                    manager,
                    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS address_line1",
                )
                .await?;
                execute(
                    manager,
                    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS address_line2",
                )
                .await
            }
            _ => Ok(()),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
