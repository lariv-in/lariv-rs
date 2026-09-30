use crate::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const PROFILE_COLUMNS: &str = r#"
ALTER TABLE hr_employees
    ADD COLUMN fathers_name TEXT NOT NULL DEFAULT '',
    ADD COLUMN date_of_birth DATE,
    ADD COLUMN gender VARCHAR(32),
    ADD COLUMN marital_status TEXT NOT NULL DEFAULT '',
    ADD COLUMN nationality TEXT NOT NULL DEFAULT '',
    ADD COLUMN is_disabled BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN disability_type TEXT NOT NULL DEFAULT '',
    ADD COLUMN photograph_vnode_id BIGINT,
    ADD COLUMN identification_mark TEXT NOT NULL DEFAULT '',
    ADD COLUMN present_address TEXT NOT NULL DEFAULT '',
    ADD COLUMN present_pin_code TEXT NOT NULL DEFAULT '',
    ADD COLUMN permanent_address TEXT NOT NULL DEFAULT '',
    ADD COLUMN permanent_pin_code TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_contact_name TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_contact_relation TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_contact_mobile TEXT NOT NULL DEFAULT '',
    ADD COLUMN aadhar_document_id BIGINT,
    ADD COLUMN pan_document_id BIGINT,
    ADD COLUMN passport_document_id BIGINT,
    ADD COLUMN account_holder_name TEXT NOT NULL DEFAULT '',
    ADD COLUMN account_number TEXT NOT NULL DEFAULT '',
    ADD COLUMN account_ifsc_code TEXT NOT NULL DEFAULT '',
    ADD COLUMN account_type TEXT NOT NULL DEFAULT '',
    ADD COLUMN qualifications TEXT NOT NULL DEFAULT '',
    ADD COLUMN date_of_joining DATE,
    ADD COLUMN probation_end_date DATE
"#;

const INDEXES: &[&str] = &[
    "CREATE INDEX idx_hr_employees_photograph_vnode_id ON hr_employees (photograph_vnode_id)",
    "CREATE INDEX idx_hr_employees_aadhar_document_id ON hr_employees (aadhar_document_id)",
    "CREATE INDEX idx_hr_employees_pan_document_id ON hr_employees (pan_document_id)",
    "CREATE INDEX idx_hr_employees_passport_document_id ON hr_employees (passport_document_id)",
    r#"
ALTER TABLE hr_employees
    ADD CONSTRAINT fk_hr_employees_photograph_vnode_id
    FOREIGN KEY (photograph_vnode_id) REFERENCES filesystem_nodes (id) ON DELETE SET NULL
"#,
];

const DROP_PROFILE: &str = r#"
ALTER TABLE hr_employees
    DROP COLUMN IF EXISTS fathers_name,
    DROP COLUMN IF EXISTS date_of_birth,
    DROP COLUMN IF EXISTS gender,
    DROP COLUMN IF EXISTS marital_status,
    DROP COLUMN IF EXISTS nationality,
    DROP COLUMN IF EXISTS is_disabled,
    DROP COLUMN IF EXISTS disability_type,
    DROP COLUMN IF EXISTS photograph_vnode_id,
    DROP COLUMN IF EXISTS blood_group,
    DROP COLUMN IF EXISTS identification_mark,
    DROP COLUMN IF EXISTS present_address,
    DROP COLUMN IF EXISTS present_pin_code,
    DROP COLUMN IF EXISTS permanent_address,
    DROP COLUMN IF EXISTS permanent_pin_code,
    DROP COLUMN IF EXISTS emergency_contact_name,
    DROP COLUMN IF EXISTS emergency_contact_relation,
    DROP COLUMN IF EXISTS emergency_contact_mobile,
    DROP COLUMN IF EXISTS aadhar_document_id,
    DROP COLUMN IF EXISTS pan_document_id,
    DROP COLUMN IF EXISTS passport_document_id,
    DROP COLUMN IF EXISTS account_holder_name,
    DROP COLUMN IF EXISTS account_number,
    DROP COLUMN IF EXISTS account_ifsc_code,
    DROP COLUMN IF EXISTS account_type,
    DROP COLUMN IF EXISTS qualifications,
    DROP COLUMN IF EXISTS date_of_joining,
    DROP COLUMN IF EXISTS probation_end_date
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, PROFILE_COLUMNS).await?;
        if is_postgres(manager) {
            exec_sql(
                manager,
                "CREATE TYPE hr_blood_group AS ENUM ('A+', 'A-', 'B+', 'B-', 'AB+', 'AB-', 'O+', 'O-')",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_employees ADD COLUMN blood_group hr_blood_group",
            )
            .await?;
        } else {
            exec_sql(
                manager,
                "ALTER TABLE hr_employees ADD COLUMN blood_group VARCHAR(8)",
            )
            .await?;
        }
        for sql in INDEXES {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, DROP_PROFILE).await?;
        if is_postgres(manager) {
            exec_sql(manager, "DROP TYPE IF EXISTS hr_blood_group").await?;
        }
        Ok(())
    }
}
