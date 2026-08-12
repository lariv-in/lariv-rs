use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const CREATE_FUNCTION: &str = r#"
CREATE OR REPLACE FUNCTION accounts_enforce_parent_balance_type() RETURNS TRIGGER AS $fn$
BEGIN
  IF NEW.parent_id IS NOT NULL THEN
    IF NOT EXISTS (
      SELECT 1 FROM accounts AS p
      WHERE p.id = NEW.parent_id
        AND p.deleted_at IS NULL
        AND p.balance_type = NEW.balance_type
    ) THEN
      RAISE EXCEPTION 'balance_type must match the parent account balance_type';
    END IF;
  END IF;

  IF TG_OP = 'UPDATE' AND NEW.balance_type IS DISTINCT FROM OLD.balance_type THEN
    IF EXISTS (
      SELECT 1 FROM accounts AS c
      WHERE c.parent_id = NEW.id
        AND c.deleted_at IS NULL
        AND c.balance_type IS DISTINCT FROM NEW.balance_type
    ) THEN
      RAISE EXCEPTION 'cannot change balance_type while child accounts have a different balance_type';
    END IF;
  END IF;

  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql
"#;

const DROP_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS accounts_enforce_parent_balance_type_biud ON accounts";

const CREATE_TRIGGER: &str = r#"
CREATE TRIGGER accounts_enforce_parent_balance_type_biud
  BEFORE INSERT OR UPDATE ON accounts
  FOR EACH ROW EXECUTE PROCEDURE accounts_enforce_parent_balance_type()
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
        execute(manager, CREATE_FUNCTION).await?;
        execute(manager, DROP_TRIGGER).await?;
        execute(manager, CREATE_TRIGGER).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, DROP_TRIGGER).await?;
        execute(
            manager,
            "DROP FUNCTION IF EXISTS accounts_enforce_parent_balance_type()",
        )
        .await
    }
}
