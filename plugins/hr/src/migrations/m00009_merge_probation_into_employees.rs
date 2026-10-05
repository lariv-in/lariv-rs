use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "ALTER TABLE hr_employees ADD COLUMN is_probationary BOOLEAN NOT NULL DEFAULT false",
    r#"
INSERT INTO hr_employees (
    created_at, updated_at, user_id, name, mobile, email, hired_at, is_probationary
)
SELECT created_at, updated_at, user_id, name, mobile, email, started_at, true
FROM hr_probations
"#,
    "DROP TABLE hr_probations",
];

const DOWN: &[&str] = &[
    r#"
CREATE TABLE hr_probations (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    user_id BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    mobile TEXT NOT NULL,
    email TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL
)
"#,
    "CREATE INDEX idx_hr_probations_user_id ON hr_probations (user_id)",
    r#"
INSERT INTO hr_probations (
    created_at, updated_at, user_id, name, mobile, email, started_at
)
SELECT created_at, updated_at, user_id, name, mobile, email, hired_at
FROM hr_employees
WHERE is_probationary
"#,
    "DELETE FROM hr_employees WHERE is_probationary",
    "ALTER TABLE hr_employees DROP COLUMN is_probationary",
];

async fn exec_each(manager: &SchemaManager<'_>, statements: &[&str]) -> Result<(), DbErr> {
    for sql in statements {
        exec_sql(manager, sql).await?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_each(manager, UP).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_each(manager, DOWN).await
    }
}
