use crate::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const RENAME_TO_VNODES: &[&str] = &[
    "ALTER TABLE hr_employees RENAME COLUMN aadhar_document_id TO aadhar_vnode_id",
    "ALTER TABLE hr_employees RENAME COLUMN pan_document_id TO pan_vnode_id",
    "ALTER TABLE hr_employees RENAME COLUMN passport_document_id TO passport_vnode_id",
];

const DROP_DOC_INDEXES: &[&str] = &[
    "DROP INDEX IF EXISTS idx_hr_employees_aadhar_document_id",
    "DROP INDEX IF EXISTS idx_hr_employees_pan_document_id",
    "DROP INDEX IF EXISTS idx_hr_employees_passport_document_id",
];

const CREATE_VNODE_INDEXES: &[&str] = &[
    "CREATE INDEX idx_hr_employees_aadhar_vnode_id ON hr_employees (aadhar_vnode_id)",
    "CREATE INDEX idx_hr_employees_pan_vnode_id ON hr_employees (pan_vnode_id)",
    "CREATE INDEX idx_hr_employees_passport_vnode_id ON hr_employees (passport_vnode_id)",
];

const POSTGRES_CONSTRAINTS: &[&str] = &[
    "ALTER TABLE hr_employees ADD CONSTRAINT fk_hr_employees_aadhar_vnode_id FOREIGN KEY (aadhar_vnode_id) REFERENCES filesystem_nodes (id) ON DELETE SET NULL",
    "ALTER TABLE hr_employees ADD CONSTRAINT fk_hr_employees_pan_vnode_id FOREIGN KEY (pan_vnode_id) REFERENCES filesystem_nodes (id) ON DELETE SET NULL",
    "ALTER TABLE hr_employees ADD CONSTRAINT fk_hr_employees_passport_vnode_id FOREIGN KEY (passport_vnode_id) REFERENCES filesystem_nodes (id) ON DELETE SET NULL",
];

const POSTGRES_DROP_CONSTRAINTS: &[&str] = &[
    "ALTER TABLE hr_employees DROP CONSTRAINT IF EXISTS fk_hr_employees_aadhar_vnode_id",
    "ALTER TABLE hr_employees DROP CONSTRAINT IF EXISTS fk_hr_employees_pan_vnode_id",
    "ALTER TABLE hr_employees DROP CONSTRAINT IF EXISTS fk_hr_employees_passport_vnode_id",
];

const DROP_VNODE_INDEXES: &[&str] = &[
    "DROP INDEX IF EXISTS idx_hr_employees_aadhar_vnode_id",
    "DROP INDEX IF EXISTS idx_hr_employees_pan_vnode_id",
    "DROP INDEX IF EXISTS idx_hr_employees_passport_vnode_id",
];

const CREATE_DOC_INDEXES: &[&str] = &[
    "CREATE INDEX idx_hr_employees_aadhar_document_id ON hr_employees (aadhar_vnode_id)",
    "CREATE INDEX idx_hr_employees_pan_document_id ON hr_employees (pan_vnode_id)",
    "CREATE INDEX idx_hr_employees_passport_document_id ON hr_employees (passport_vnode_id)",
];

const RENAME_TO_DOCUMENTS: &[&str] = &[
    "ALTER TABLE hr_employees RENAME COLUMN aadhar_vnode_id TO aadhar_document_id",
    "ALTER TABLE hr_employees RENAME COLUMN pan_vnode_id TO pan_document_id",
    "ALTER TABLE hr_employees RENAME COLUMN passport_vnode_id TO passport_document_id",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_sql(
                manager,
                "ALTER TABLE hr_employees ALTER COLUMN disability_type DROP NOT NULL",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_employees ALTER COLUMN disability_type DROP DEFAULT",
            )
            .await?;
        }

        for sql in RENAME_TO_VNODES {
            exec_sql(manager, sql).await?;
        }

        for sql in DROP_DOC_INDEXES {
            exec_sql(manager, sql).await?;
        }

        for sql in CREATE_VNODE_INDEXES {
            exec_sql(manager, sql).await?;
        }

        if is_postgres(manager) {
            for sql in POSTGRES_CONSTRAINTS {
                exec_sql(manager, sql).await?;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            for sql in POSTGRES_DROP_CONSTRAINTS {
                exec_sql(manager, sql).await?;
            }
        }

        for sql in DROP_VNODE_INDEXES {
            exec_sql(manager, sql).await?;
        }

        for sql in CREATE_DOC_INDEXES {
            exec_sql(manager, sql).await?;
        }

        for sql in RENAME_TO_DOCUMENTS {
            exec_sql(manager, sql).await?;
        }

        if is_postgres(manager) {
            exec_sql(
                manager,
                "UPDATE hr_employees SET disability_type = '' WHERE disability_type IS NULL",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_employees ALTER COLUMN disability_type SET DEFAULT ''",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_employees ALTER COLUMN disability_type SET NOT NULL",
            )
            .await?;
        }

        Ok(())
    }
}
