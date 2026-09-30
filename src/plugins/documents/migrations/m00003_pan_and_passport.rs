use crate::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const PAN_TABLE: &str = r#"
CREATE TABLE pan_cards (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    vnode_id BIGINT NOT NULL REFERENCES filesystem_nodes (id) ON DELETE RESTRICT,
    pan_number TEXT NOT NULL,
    name TEXT NOT NULL,
    date_of_birth DATE NOT NULL
)
"#;

const PASSPORT_TABLE: &str = r#"
CREATE TABLE passports (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    vnode_id BIGINT NOT NULL REFERENCES filesystem_nodes (id) ON DELETE RESTRICT,
    passport_number TEXT NOT NULL,
    name TEXT NOT NULL,
    gender VARCHAR(32) NOT NULL,
    date_of_birth DATE NOT NULL,
    nationality TEXT NOT NULL,
    expiry_date DATE NOT NULL
)
"#;

const INDEXES: &[&str] = &[
    "CREATE INDEX idx_pan_cards_vnode_id ON pan_cards (vnode_id)",
    "CREATE UNIQUE INDEX uix_pan_cards_pan_number ON pan_cards (pan_number)",
    "CREATE INDEX idx_passports_vnode_id ON passports (vnode_id)",
    "CREATE UNIQUE INDEX uix_passports_passport_number ON passports (passport_number)",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_sql(
                manager,
                "ALTER TYPE document_type ADD VALUE IF NOT EXISTS 'pan'",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TYPE document_type ADD VALUE IF NOT EXISTS 'passport'",
            )
            .await?;
        }
        exec_sql(manager, PAN_TABLE).await?;
        exec_sql(manager, PASSPORT_TABLE).await?;
        for sql in INDEXES {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, "DROP TABLE IF EXISTS passports").await?;
        exec_sql(manager, "DROP TABLE IF EXISTS pan_cards").await?;
        Ok(())
    }
}
