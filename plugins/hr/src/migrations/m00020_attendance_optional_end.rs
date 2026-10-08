//! Punch-in rows store a start and leave `ended_at` empty until punch-out.
//!
//! SQLite cannot `ALTER COLUMN … DROP NOT NULL`, so that backend copies the table.

use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_sql(
                manager,
                "ALTER TABLE hr_attendances ALTER COLUMN ended_at DROP NOT NULL",
            )
            .await?;
        } else {
            rebuild_sqlite(manager, true).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_sql(
                manager,
                "UPDATE hr_attendances SET ended_at = started_at WHERE ended_at IS NULL",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_attendances ALTER COLUMN ended_at SET NOT NULL",
            )
            .await?;
        } else {
            rebuild_sqlite(manager, false).await?;
        }
        Ok(())
    }
}

/// `optional` makes `ended_at` nullable. The reverse fills blanks from `started_at`.
async fn rebuild_sqlite(manager: &SchemaManager<'_>, optional: bool) -> Result<(), DbErr> {
    let ended_at = if optional {
        r#""ended_at" timestamp_with_timezone_text"#
    } else {
        r#""ended_at" timestamp_with_timezone_text NOT NULL"#
    };
    let ended_select = if optional {
        r#""ended_at""#
    } else {
        r#"COALESCE("ended_at", "started_at")"#
    };
    exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
    exec_sql(
        manager,
        &format!(
            r#"CREATE TABLE "hr_attendances__end" (
            "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
            "created_at" timestamp_with_timezone_text,
            "updated_at" timestamp_with_timezone_text,
            "user_id" integer NOT NULL,
            "started_at" timestamp_with_timezone_text NOT NULL,
            {ended_at},
            FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE ON UPDATE CASCADE
        )"#
        ),
    )
    .await?;
    exec_sql(
        manager,
        &format!(
            r#"INSERT INTO "hr_attendances__end" (
            "id", "created_at", "updated_at", "user_id", "started_at", "ended_at"
        )
        SELECT "id", "created_at", "updated_at", "user_id", "started_at", {ended_select}
        FROM "hr_attendances""#
        ),
    )
    .await?;
    exec_sql(manager, r#"DROP TABLE "hr_attendances""#).await?;
    exec_sql(
        manager,
        r#"ALTER TABLE "hr_attendances__end" RENAME TO "hr_attendances""#,
    )
    .await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'hr_attendances' WHERE name = 'hr_attendances__end'",
    )
    .await?;
    exec_sql(
        manager,
        "CREATE INDEX idx_hr_attendances_user_id ON hr_attendances (user_id)",
    )
    .await?;
    exec_sql(manager, "PRAGMA foreign_keys = ON").await
}
