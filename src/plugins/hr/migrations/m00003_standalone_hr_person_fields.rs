//! Repair HR stage tables created before standalone person fields.
//!
//! Early HR migrations stored only `applicant_id` on probations/employees/ex-employees.
//! Models now expect each table to carry its own `name`, `mobile`, and `email`.

use crate::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ADD_PROBATION_NAME: &str = "ALTER TABLE hr_probations ADD COLUMN IF NOT EXISTS name TEXT";
const ADD_PROBATION_MOBILE: &str = "ALTER TABLE hr_probations ADD COLUMN IF NOT EXISTS mobile TEXT";
const ADD_PROBATION_EMAIL: &str = "ALTER TABLE hr_probations ADD COLUMN IF NOT EXISTS email TEXT";

const ADD_EMPLOYEE_NAME: &str = "ALTER TABLE hr_employees ADD COLUMN IF NOT EXISTS name TEXT";
const ADD_EMPLOYEE_MOBILE: &str = "ALTER TABLE hr_employees ADD COLUMN IF NOT EXISTS mobile TEXT";
const ADD_EMPLOYEE_EMAIL: &str = "ALTER TABLE hr_employees ADD COLUMN IF NOT EXISTS email TEXT";

const ADD_EX_EMPLOYEE_NAME: &str = "ALTER TABLE hr_ex_employees ADD COLUMN IF NOT EXISTS name TEXT";
const ADD_EX_EMPLOYEE_MOBILE: &str =
    "ALTER TABLE hr_ex_employees ADD COLUMN IF NOT EXISTS mobile TEXT";
const ADD_EX_EMPLOYEE_EMAIL: &str =
    "ALTER TABLE hr_ex_employees ADD COLUMN IF NOT EXISTS email TEXT";

const BACKFILL_POSTGRES: &str = r#"
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = current_schema()
      AND table_name = 'hr_probations'
      AND column_name = 'applicant_id'
  ) THEN
    UPDATE hr_probations AS p
    SET
      name = COALESCE(NULLIF(BTRIM(p.name), ''), a.name, 'Unknown'),
      mobile = COALESCE(NULLIF(BTRIM(p.mobile), ''), a.mobile, 'Unknown'),
      email = COALESCE(NULLIF(BTRIM(p.email), ''), a.email, 'unknown@example.com')
    FROM hr_applicants AS a
    WHERE p.applicant_id = a.id;

    UPDATE hr_probations
    SET
      name = COALESCE(NULLIF(BTRIM(name), ''), 'Unknown'),
      mobile = COALESCE(NULLIF(BTRIM(mobile), ''), 'Unknown'),
      email = COALESCE(NULLIF(BTRIM(email), ''), 'unknown@example.com')
    WHERE name IS NULL OR BTRIM(name) = '' OR mobile IS NULL OR BTRIM(mobile) = ''
       OR email IS NULL OR BTRIM(email) = '';
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = current_schema()
      AND table_name = 'hr_employees'
      AND column_name = 'applicant_id'
  ) THEN
    UPDATE hr_employees AS e
    SET
      name = COALESCE(NULLIF(BTRIM(e.name), ''), a.name, 'Unknown'),
      mobile = COALESCE(NULLIF(BTRIM(e.mobile), ''), a.mobile, 'Unknown'),
      email = COALESCE(NULLIF(BTRIM(e.email), ''), a.email, 'unknown@example.com')
    FROM hr_applicants AS a
    WHERE e.applicant_id = a.id;

    UPDATE hr_employees
    SET
      name = COALESCE(NULLIF(BTRIM(name), ''), 'Unknown'),
      mobile = COALESCE(NULLIF(BTRIM(mobile), ''), 'Unknown'),
      email = COALESCE(NULLIF(BTRIM(email), ''), 'unknown@example.com')
    WHERE name IS NULL OR BTRIM(name) = '' OR mobile IS NULL OR BTRIM(mobile) = ''
       OR email IS NULL OR BTRIM(email) = '';
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = current_schema()
      AND table_name = 'hr_ex_employees'
      AND column_name = 'applicant_id'
  ) THEN
    UPDATE hr_ex_employees AS x
    SET
      name = COALESCE(NULLIF(BTRIM(x.name), ''), a.name, 'Unknown'),
      mobile = COALESCE(NULLIF(BTRIM(x.mobile), ''), a.mobile, 'Unknown'),
      email = COALESCE(NULLIF(BTRIM(x.email), ''), a.email, 'unknown@example.com')
    FROM hr_applicants AS a
    WHERE x.applicant_id = a.id;

    UPDATE hr_ex_employees
    SET
      name = COALESCE(NULLIF(BTRIM(name), ''), 'Unknown'),
      mobile = COALESCE(NULLIF(BTRIM(mobile), ''), 'Unknown'),
      email = COALESCE(NULLIF(BTRIM(email), ''), 'unknown@example.com')
    WHERE name IS NULL OR BTRIM(name) = '' OR mobile IS NULL OR BTRIM(mobile) = ''
       OR email IS NULL OR BTRIM(email) = '';
  END IF;
END;
$$;
"#;

const DROP_APPLICANT_ID_COLUMNS: &[&str] = &[
    "ALTER TABLE hr_probations DROP COLUMN IF EXISTS applicant_id",
    "ALTER TABLE hr_employees DROP COLUMN IF EXISTS applicant_id",
    "ALTER TABLE hr_ex_employees DROP COLUMN IF EXISTS applicant_id",
];


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            ADD_PROBATION_NAME,
            ADD_PROBATION_MOBILE,
            ADD_PROBATION_EMAIL,
            ADD_EMPLOYEE_NAME,
            ADD_EMPLOYEE_MOBILE,
            ADD_EMPLOYEE_EMAIL,
            ADD_EX_EMPLOYEE_NAME,
            ADD_EX_EMPLOYEE_MOBILE,
            ADD_EX_EMPLOYEE_EMAIL,
        ] {
            exec_sql(manager, sql).await?;
        }

        if matches!(
            manager.get_connection().get_database_backend(),
            sea_orm::DatabaseBackend::Postgres
        ) {
            exec_sql(manager, BACKFILL_POSTGRES).await?;
        }

        for sql in DROP_APPLICANT_ID_COLUMNS {
            exec_sql(manager, sql).await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
