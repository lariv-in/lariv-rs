use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const COPY_ADDRESS_LINE_1: &str = r#"
UPDATE crm_companies
SET address_line_1 = address_line1
WHERE address_line_1 IS NULL
  AND address_line1 IS NOT NULL
  AND EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'crm_companies'
      AND column_name = 'address_line1'
  )
"#;

const COPY_ADDRESS_LINE_2: &str = r#"
UPDATE crm_companies
SET address_line_2 = address_line2
WHERE address_line_2 IS NULL
  AND address_line2 IS NOT NULL
  AND EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'crm_companies'
      AND column_name = 'address_line2'
  )
"#;

const DROP_ADDRESS_LINE1: &str =
    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS address_line1";
const DROP_ADDRESS_LINE2: &str =
    "ALTER TABLE crm_companies DROP COLUMN IF EXISTS address_line2";

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
        execute(manager, COPY_ADDRESS_LINE_1).await?;
        execute(manager, COPY_ADDRESS_LINE_2).await?;
        execute(manager, DROP_ADDRESS_LINE1).await?;
        execute(manager, DROP_ADDRESS_LINE2).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
