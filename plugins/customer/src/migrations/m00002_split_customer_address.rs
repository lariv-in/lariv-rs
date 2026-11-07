use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
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

const SPLIT_COLUMNS: [&str; 5] = [
    "address_line_1",
    "address_line_2",
    "city",
    "pincode",
    "state",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            return up_sqlite(manager).await;
        }
        exec_sql(manager, ADD_ADDRESS_LINE_1).await?;
        exec_sql(manager, ADD_ADDRESS_LINE_2).await?;
        exec_sql(manager, ADD_CITY).await?;
        exec_sql(manager, ADD_PINCODE).await?;
        exec_sql(manager, ADD_STATE).await?;
        exec_sql(manager, MIGRATE_AND_DROP_ADDRESS).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            return down_sqlite(manager).await;
        }
        exec_sql(manager, ADD_ADDRESS).await?;
        exec_sql(manager, MERGE_ADDRESS_FIELDS).await?;
        exec_sql(manager, DROP_ADDRESS_LINE_1).await?;
        exec_sql(manager, DROP_ADDRESS_LINE_2).await?;
        exec_sql(manager, DROP_CITY).await?;
        exec_sql(manager, DROP_PINCODE).await?;
        exec_sql(manager, DROP_STATE).await
    }
}

/// SQLite rejects `ADD COLUMN IF NOT EXISTS` and `DROP COLUMN IF EXISTS`.
/// `m00001` already creates the split columns on a fresh database.
async fn up_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for column in SPLIT_COLUMNS {
        if !column_exists(manager, column).await? {
            exec_sql(
                manager,
                &format!("ALTER TABLE customers ADD COLUMN {column} TEXT"),
            )
            .await?;
        }
    }
    if column_exists(manager, "address").await? {
        exec_sql(
            manager,
            "UPDATE customers SET address_line_1 = address WHERE address IS NOT NULL AND trim(address) <> ''",
        )
        .await?;
        exec_sql(manager, "ALTER TABLE customers DROP COLUMN address").await?;
    }
    Ok(())
}

async fn down_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if !column_exists(manager, "address").await? {
        exec_sql(manager, "ALTER TABLE customers ADD COLUMN address TEXT").await?;
    }
    exec_sql(
        manager,
        r#"
UPDATE customers
SET address = NULLIF(
  concat_ws(
    char(10),
    NULLIF(trim(address_line_1), ''),
    NULLIF(trim(address_line_2), ''),
    NULLIF(trim(city), ''),
    NULLIF(trim(pincode), ''),
    NULLIF(trim(state), '')
  ),
  ''
)
"#,
    )
    .await?;
    for column in SPLIT_COLUMNS {
        if column_exists(manager, column).await? {
            exec_sql(
                manager,
                &format!("ALTER TABLE customers DROP COLUMN {column}"),
            )
            .await?;
        }
    }
    Ok(())
}

async fn column_exists(manager: &SchemaManager<'_>, column: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            manager.get_database_backend(),
            format!("SELECT 1 FROM pragma_table_info('customers') WHERE name = '{column}'"),
        ))
        .await?;
    Ok(row.is_some())
}
