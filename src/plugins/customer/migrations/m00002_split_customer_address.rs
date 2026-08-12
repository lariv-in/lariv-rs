use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ADD_ADDRESS_LINE_1: &str =
    "ALTER TABLE customers ADD COLUMN IF NOT EXISTS address_line_1 TEXT";
const ADD_ADDRESS_LINE_2: &str =
    "ALTER TABLE customers ADD COLUMN IF NOT EXISTS address_line_2 TEXT";
const ADD_CITY: &str = "ALTER TABLE customers ADD COLUMN IF NOT EXISTS city TEXT";
const ADD_PINCODE: &str = "ALTER TABLE customers ADD COLUMN IF NOT EXISTS pincode TEXT";
const ADD_STATE: &str = "ALTER TABLE customers ADD COLUMN IF NOT EXISTS state TEXT";

const MIGRATE_AND_DROP_ADDRESS: &str = r#"
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'customers' AND column_name = 'address'
  ) THEN
    UPDATE customers
    SET address_line_1 = address
    WHERE address IS NOT NULL AND btrim(address) <> '';
    ALTER TABLE customers DROP COLUMN address;
  END IF;
END;
$$;
"#;

const ADD_ADDRESS: &str = "ALTER TABLE customers ADD COLUMN IF NOT EXISTS address TEXT";

const MERGE_ADDRESS_FIELDS: &str = r#"
UPDATE customers
SET address = NULLIF(
  concat_ws(
    E'\n',
    NULLIF(btrim(address_line_1), ''),
    NULLIF(btrim(address_line_2), ''),
    NULLIF(btrim(city), ''),
    NULLIF(btrim(pincode), ''),
    NULLIF(btrim(state), '')
  ),
  ''
)
"#;

const DROP_ADDRESS_LINE_1: &str = "ALTER TABLE customers DROP COLUMN IF EXISTS address_line_1";
const DROP_ADDRESS_LINE_2: &str = "ALTER TABLE customers DROP COLUMN IF EXISTS address_line_2";
const DROP_CITY: &str = "ALTER TABLE customers DROP COLUMN IF EXISTS city";
const DROP_PINCODE: &str = "ALTER TABLE customers DROP COLUMN IF EXISTS pincode";
const DROP_STATE: &str = "ALTER TABLE customers DROP COLUMN IF EXISTS state";

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
        execute(manager, ADD_ADDRESS_LINE_1).await?;
        execute(manager, ADD_ADDRESS_LINE_2).await?;
        execute(manager, ADD_CITY).await?;
        execute(manager, ADD_PINCODE).await?;
        execute(manager, ADD_STATE).await?;
        execute(manager, MIGRATE_AND_DROP_ADDRESS).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, ADD_ADDRESS).await?;
        execute(manager, MERGE_ADDRESS_FIELDS).await?;
        execute(manager, DROP_ADDRESS_LINE_1).await?;
        execute(manager, DROP_ADDRESS_LINE_2).await?;
        execute(manager, DROP_CITY).await?;
        execute(manager, DROP_PINCODE).await?;
        execute(manager, DROP_STATE).await
    }
}
