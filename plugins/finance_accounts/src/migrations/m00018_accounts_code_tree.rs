use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const DROP_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS accounts_enforce_parent_balance_type_biud ON accounts";

const DROP_FUNCTION: &str = "DROP FUNCTION IF EXISTS accounts_enforce_parent_balance_type()";

/// Nearest existing ancestor by code:
/// `70301` → `70300`, else `70000`; `70200` → `70000`; multiples of 10000 are roots.
/// Duplicate parent codes resolve to the lowest id.
const REBUILD_PARENTS: &str = r#"
UPDATE accounts AS child
SET parent_id = picked.id
FROM accounts AS c
CROSS JOIN LATERAL (
  SELECT CASE
    WHEN c.code % 100 <> 0 THEN c.code - (c.code % 100)
    ELSE c.code
  END AS group_code
) AS grp
LEFT JOIN LATERAL (
  SELECT p.id
  FROM accounts AS p
  WHERE p.id <> c.id
    AND (
      (c.code % 100 <> 0 AND p.code = grp.group_code)
      OR (
        grp.group_code % 10000 <> 0
        AND p.code = grp.group_code - (grp.group_code % 10000)
      )
    )
  ORDER BY p.code DESC, p.id ASC
  LIMIT 1
) AS picked ON true
WHERE child.id = c.id
"#;

/// Matches the function as left by m00016 (no `deleted_at`).
const RESTORE_FUNCTION: &str = r#"
CREATE OR REPLACE FUNCTION accounts_enforce_parent_balance_type() RETURNS TRIGGER AS $fn$
BEGIN
  IF NEW.parent_id IS NOT NULL THEN
    IF NOT EXISTS (
      SELECT 1 FROM accounts AS p
      WHERE p.id = NEW.parent_id
        AND p.balance_type = NEW.balance_type
    ) THEN
      RAISE EXCEPTION 'balance_type must match the parent account balance_type';
    END IF;
  END IF;
  IF TG_OP = 'UPDATE' AND NEW.balance_type IS DISTINCT FROM OLD.balance_type THEN
    IF EXISTS (
      SELECT 1 FROM accounts AS c
      WHERE c.parent_id = NEW.id
        AND c.balance_type IS DISTINCT FROM NEW.balance_type
    ) THEN
      RAISE EXCEPTION 'cannot change balance_type while child accounts have a different balance_type';
    END IF;
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql
"#;

const CREATE_TRIGGER: &str = r#"
CREATE TRIGGER accounts_enforce_parent_balance_type_biud
  BEFORE INSERT OR UPDATE ON accounts
  FOR EACH ROW EXECUTE PROCEDURE accounts_enforce_parent_balance_type()
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, DROP_TRIGGER).await?;
        exec_sql(manager, DROP_FUNCTION).await?;
        exec_sql(manager, REBUILD_PARENTS).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, RESTORE_FUNCTION).await?;
        exec_sql(manager, DROP_TRIGGER).await?;
        exec_sql(manager, CREATE_TRIGGER).await
    }
}
