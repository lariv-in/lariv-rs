use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, Statement};
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

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => exec_sql(manager, UP_POSTGRES).await,
            // Fresh SQLite creates never had `address_line1` / `address_line2`.
            // SQLite rejects DROP COLUMN IF EXISTS, so drop only a column that is present.
            sea_orm::DatabaseBackend::Sqlite => {
                for (legacy, current) in [
                    ("address_line1", "address_line_1"),
                    ("address_line2", "address_line_2"),
                ] {
                    if !column_exists(manager, legacy).await? {
                        continue;
                    }
                    exec_sql(
                        manager,
                        &format!(
                            "UPDATE crm_companies SET {current} = {legacy} WHERE {current} IS NULL AND {legacy} IS NOT NULL"
                        ),
                    )
                    .await?;
                    exec_sql(
                        manager,
                        &format!("ALTER TABLE crm_companies DROP COLUMN {legacy}"),
                    )
                    .await?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn column_exists(manager: &SchemaManager<'_>, column: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            manager.get_database_backend(),
            format!("SELECT 1 FROM pragma_table_info('crm_companies') WHERE name = '{column}'"),
        ))
        .await?;
    Ok(row.is_some())
}
