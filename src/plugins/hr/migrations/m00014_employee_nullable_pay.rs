use crate::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrEmployees {
    Table,
    WorkStart,
    WorkEnd,
    BaseSalary,
    HourlyWage,
}

/// Text columns that were required and become optional profile data.
const RELAX_WITH_EMPTY_DEFAULT: &[&str] = &[
    "fathers_name",
    "marital_status",
    "nationality",
    "identification_mark",
    "present_address",
    "present_pin_code",
    "permanent_address",
    "permanent_pin_code",
    "emergency_contact_name",
    "emergency_contact_relation",
    "emergency_contact_mobile",
    "account_holder_name",
    "account_number",
    "account_ifsc_code",
    "account_type",
    "qualifications",
];

/// Identity columns required to open the record's user, now optional on the employee row.
const RELAX_IDENTITY: &[&str] = &["name", "mobile", "email"];

/// Already nullable; blank strings become SQL NULL.
const BLANK_TO_NULL: &[&str] = &["disability_type", "gender"];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            relax_postgres(manager).await?;
            add_pay_columns(manager).await?;
        } else {
            rebuild_sqlite_employees(manager).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .drop_column(HrEmployees::WorkStart)
                    .drop_column(HrEmployees::WorkEnd)
                    .drop_column(HrEmployees::BaseSalary)
                    .drop_column(HrEmployees::HourlyWage)
                    .to_owned(),
            )
            .await?;
        if is_postgres(manager) {
            restore_postgres(manager).await?;
        }
        Ok(())
    }
}

async fn add_pay_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(HrEmployees::Table)
                .add_column(ColumnDef::new(HrEmployees::WorkStart).time().null())
                .add_column(ColumnDef::new(HrEmployees::WorkEnd).time().null())
                .add_column(
                    ColumnDef::new(HrEmployees::BaseSalary)
                        .decimal_len(19, 6)
                        .null(),
                )
                .add_column(
                    ColumnDef::new(HrEmployees::HourlyWage)
                        .decimal_len(19, 6)
                        .null(),
                )
                .to_owned(),
        )
        .await
}

async fn relax_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for column in RELAX_IDENTITY
        .iter()
        .chain(RELAX_WITH_EMPTY_DEFAULT)
        .chain(BLANK_TO_NULL)
    {
        exec_sql(
            manager,
            &format!("ALTER TABLE hr_employees ALTER COLUMN {column} DROP NOT NULL"),
        )
        .await?;
        exec_sql(
            manager,
            &format!("ALTER TABLE hr_employees ALTER COLUMN {column} DROP DEFAULT"),
        )
        .await?;
        exec_sql(
            manager,
            &format!(
                "UPDATE hr_employees SET {column} = NULL WHERE {column} IS NOT NULL AND btrim({column}) = ''"
            ),
        )
        .await?;
    }
    exec_sql(
        manager,
        "ALTER TABLE hr_employees ALTER COLUMN is_disabled DROP NOT NULL",
    )
    .await?;
    exec_sql(
        manager,
        "ALTER TABLE hr_employees ALTER COLUMN is_disabled DROP DEFAULT",
    )
    .await?;
    Ok(())
}

async fn restore_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for column in RELAX_IDENTITY {
        exec_sql(
            manager,
            &format!("UPDATE hr_employees SET {column} = '' WHERE {column} IS NULL"),
        )
        .await?;
        exec_sql(
            manager,
            &format!("ALTER TABLE hr_employees ALTER COLUMN {column} SET NOT NULL"),
        )
        .await?;
    }
    for column in RELAX_WITH_EMPTY_DEFAULT {
        exec_sql(
            manager,
            &format!("UPDATE hr_employees SET {column} = '' WHERE {column} IS NULL"),
        )
        .await?;
        exec_sql(
            manager,
            &format!("ALTER TABLE hr_employees ALTER COLUMN {column} SET DEFAULT ''"),
        )
        .await?;
        exec_sql(
            manager,
            &format!("ALTER TABLE hr_employees ALTER COLUMN {column} SET NOT NULL"),
        )
        .await?;
    }
    for column in BLANK_TO_NULL {
        exec_sql(
            manager,
            &format!("UPDATE hr_employees SET {column} = '' WHERE {column} IS NULL"),
        )
        .await?;
    }
    exec_sql(
        manager,
        "UPDATE hr_employees SET is_disabled = false WHERE is_disabled IS NULL",
    )
    .await?;
    exec_sql(
        manager,
        "ALTER TABLE hr_employees ALTER COLUMN is_disabled SET DEFAULT false",
    )
    .await?;
    exec_sql(
        manager,
        "ALTER TABLE hr_employees ALTER COLUMN is_disabled SET NOT NULL",
    )
    .await?;
    Ok(())
}

/// SQLite cannot `ALTER COLUMN … DROP NOT NULL`. Copy `hr_employees` into a table
/// whose profile columns are optional, then swap the names.
async fn rebuild_sqlite_employees(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
    exec_sql(
        manager,
        r#"CREATE TABLE "hr_employees__nullable" (
            "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
            "created_at" timestamp_with_timezone_text,
            "updated_at" timestamp_with_timezone_text,
            "user_id" integer NOT NULL,
            "name" text,
            "mobile" text,
            "email" text,
            "hired_at" timestamp_with_timezone_text NOT NULL,
            "is_probationary" boolean NOT NULL DEFAULT 0,
            "fathers_name" text,
            "date_of_birth" date_text,
            "gender" text,
            "marital_status" text,
            "nationality" text,
            "is_disabled" boolean,
            "disability_type" text,
            "photograph_vnode_id" integer,
            "blood_group" text,
            "identification_mark" text,
            "present_address" text,
            "present_pin_code" text,
            "permanent_address" text,
            "permanent_pin_code" text,
            "emergency_contact_name" text,
            "emergency_contact_relation" text,
            "emergency_contact_mobile" text,
            "aadhar_vnode_id" integer,
            "pan_vnode_id" integer,
            "passport_vnode_id" integer,
            "account_holder_name" text,
            "account_number" text,
            "account_ifsc_code" text,
            "account_type" text,
            "qualifications" text,
            "date_of_joining" date_text,
            "probation_end_date" date_text,
            "work_start" time_text,
            "work_end" time_text,
            "base_salary" real(19, 6),
            "hourly_wage" real(19, 6),
            FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE,
            FOREIGN KEY ("photograph_vnode_id") REFERENCES "filesystem_nodes" ("id") ON DELETE SET NULL,
            FOREIGN KEY ("aadhar_vnode_id") REFERENCES "filesystem_nodes" ("id") ON DELETE SET NULL,
            FOREIGN KEY ("pan_vnode_id") REFERENCES "filesystem_nodes" ("id") ON DELETE SET NULL,
            FOREIGN KEY ("passport_vnode_id") REFERENCES "filesystem_nodes" ("id") ON DELETE SET NULL
        )"#,
    )
    .await?;
    exec_sql(
        manager,
        r#"INSERT INTO "hr_employees__nullable" (
            "id", "created_at", "updated_at", "user_id", "name", "mobile", "email", "hired_at",
            "is_probationary", "fathers_name", "date_of_birth", "gender", "marital_status",
            "nationality", "is_disabled", "disability_type", "photograph_vnode_id", "blood_group",
            "identification_mark", "present_address", "present_pin_code", "permanent_address",
            "permanent_pin_code", "emergency_contact_name", "emergency_contact_relation",
            "emergency_contact_mobile", "aadhar_vnode_id", "pan_vnode_id", "passport_vnode_id",
            "account_holder_name", "account_number", "account_ifsc_code", "account_type",
            "qualifications", "date_of_joining", "probation_end_date", "work_start", "work_end",
            "base_salary", "hourly_wage"
        )
        SELECT
            "id", "created_at", "updated_at", "user_id",
            NULLIF(trim("name"), ''), NULLIF(trim("mobile"), ''), NULLIF(trim("email"), ''),
            "hired_at", "is_probationary",
            NULLIF(trim("fathers_name"), ''), "date_of_birth", NULLIF(trim("gender"), ''),
            NULLIF(trim("marital_status"), ''), NULLIF(trim("nationality"), ''), "is_disabled",
            NULLIF(trim("disability_type"), ''), "photograph_vnode_id", "blood_group",
            NULLIF(trim("identification_mark"), ''), NULLIF(trim("present_address"), ''),
            NULLIF(trim("present_pin_code"), ''), NULLIF(trim("permanent_address"), ''),
            NULLIF(trim("permanent_pin_code"), ''), NULLIF(trim("emergency_contact_name"), ''),
            NULLIF(trim("emergency_contact_relation"), ''),
            NULLIF(trim("emergency_contact_mobile"), ''), "aadhar_vnode_id", "pan_vnode_id",
            "passport_vnode_id", NULLIF(trim("account_holder_name"), ''),
            NULLIF(trim("account_number"), ''), NULLIF(trim("account_ifsc_code"), ''),
            NULLIF(trim("account_type"), ''), NULLIF(trim("qualifications"), ''),
            "date_of_joining", "probation_end_date", NULL, NULL, NULL, NULL
        FROM "hr_employees""#,
    )
    .await?;
    exec_sql(manager, r#"DROP TABLE "hr_employees""#).await?;
    exec_sql(
        manager,
        r#"ALTER TABLE "hr_employees__nullable" RENAME TO "hr_employees""#,
    )
    .await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'hr_employees' WHERE name = 'hr_employees__nullable'",
    )
    .await?;
    for sql in [
        "CREATE INDEX idx_hr_employees_user_id ON hr_employees (user_id)",
        "CREATE INDEX idx_hr_employees_photograph_vnode_id ON hr_employees (photograph_vnode_id)",
        "CREATE INDEX idx_hr_employees_aadhar_vnode_id ON hr_employees (aadhar_vnode_id)",
        "CREATE INDEX idx_hr_employees_pan_vnode_id ON hr_employees (pan_vnode_id)",
        "CREATE INDEX idx_hr_employees_passport_vnode_id ON hr_employees (passport_vnode_id)",
    ] {
        exec_sql(manager, sql).await?;
    }
    exec_sql(manager, "PRAGMA foreign_keys = ON").await
}
