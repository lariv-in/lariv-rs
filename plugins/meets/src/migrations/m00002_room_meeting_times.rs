use lariv_core::db::migration_sql::exec_sql;
use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, &add_column(manager, "scheduled_start_at")).await?;
        exec_sql(manager, &add_column(manager, "started_at")).await?;
        exec_sql(manager, &add_column(manager, "ended_at")).await?;
        exec_sql(
            manager,
            "UPDATE meets_conference_rooms SET scheduled_start_at = start_at WHERE start_at IS NOT NULL",
        )
        .await?;
        let now = match manager.get_database_backend() {
            DbBackend::Postgres => "NOW()",
            _ => "CURRENT_TIMESTAMP",
        };
        exec_sql(
            manager,
            &format!(
                "UPDATE meets_conference_rooms SET started_at = start_at WHERE start_at IS NOT NULL AND start_at <= {now}"
            ),
        )
        .await?;
        exec_sql(manager, &drop_column(manager, "start_at")).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, &add_column(manager, "start_at")).await?;
        exec_sql(
            manager,
            "UPDATE meets_conference_rooms SET start_at = COALESCE(started_at, scheduled_start_at)",
        )
        .await?;
        exec_sql(manager, &drop_column(manager, "ended_at")).await?;
        exec_sql(manager, &drop_column(manager, "started_at")).await?;
        exec_sql(manager, &drop_column(manager, "scheduled_start_at")).await
    }
}

fn timestamp_type(manager: &SchemaManager<'_>) -> &'static str {
    match manager.get_database_backend() {
        DbBackend::Postgres => "TIMESTAMPTZ",
        _ => "timestamp_with_timezone_text",
    }
}

/// Postgres accepts `ADD COLUMN IF NOT EXISTS`. SQLite does not.
fn add_column(manager: &SchemaManager<'_>, name: &str) -> String {
    let column_type = timestamp_type(manager);
    match manager.get_database_backend() {
        DbBackend::Postgres => format!(
            "ALTER TABLE meets_conference_rooms ADD COLUMN IF NOT EXISTS {name} {column_type}"
        ),
        _ => format!("ALTER TABLE meets_conference_rooms ADD COLUMN {name} {column_type}"),
    }
}

/// Postgres accepts `DROP COLUMN IF EXISTS`. SQLite does not.
fn drop_column(manager: &SchemaManager<'_>, name: &str) -> String {
    match manager.get_database_backend() {
        DbBackend::Postgres => {
            format!("ALTER TABLE meets_conference_rooms DROP COLUMN IF EXISTS {name}")
        }
        _ => format!("ALTER TABLE meets_conference_rooms DROP COLUMN {name}"),
    }
}
