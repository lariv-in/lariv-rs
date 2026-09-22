use crate::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ADD_SCHEDULED_START: &str =
    "ALTER TABLE meets_conference_rooms ADD COLUMN IF NOT EXISTS scheduled_start_at TIMESTAMPTZ";
const ADD_STARTED: &str =
    "ALTER TABLE meets_conference_rooms ADD COLUMN IF NOT EXISTS started_at TIMESTAMPTZ";
const ADD_ENDED: &str =
    "ALTER TABLE meets_conference_rooms ADD COLUMN IF NOT EXISTS ended_at TIMESTAMPTZ";

const MIGRATE_SCHEDULED: &str =
    "UPDATE meets_conference_rooms SET scheduled_start_at = start_at WHERE start_at IS NOT NULL";
const MIGRATE_STARTED: &str = "\
    UPDATE meets_conference_rooms \
    SET started_at = start_at \
    WHERE start_at IS NOT NULL AND start_at <= NOW()";
const DROP_START_AT: &str = "ALTER TABLE meets_conference_rooms DROP COLUMN IF EXISTS start_at";

const ADD_START_AT: &str =
    "ALTER TABLE meets_conference_rooms ADD COLUMN IF NOT EXISTS start_at TIMESTAMPTZ";
const RESTORE_START_AT: &str = "\
    UPDATE meets_conference_rooms \
    SET start_at = COALESCE(started_at, scheduled_start_at)";
const DROP_ENDED: &str = "ALTER TABLE meets_conference_rooms DROP COLUMN IF EXISTS ended_at";
const DROP_STARTED: &str = "ALTER TABLE meets_conference_rooms DROP COLUMN IF EXISTS started_at";
const DROP_SCHEDULED: &str =
    "ALTER TABLE meets_conference_rooms DROP COLUMN IF EXISTS scheduled_start_at";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, ADD_SCHEDULED_START).await?;
        exec_sql(manager, ADD_STARTED).await?;
        exec_sql(manager, ADD_ENDED).await?;
        exec_sql(manager, MIGRATE_SCHEDULED).await?;
        exec_sql(manager, MIGRATE_STARTED).await?;
        exec_sql(manager, DROP_START_AT).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, ADD_START_AT).await?;
        exec_sql(manager, RESTORE_START_AT).await?;
        exec_sql(manager, DROP_ENDED).await?;
        exec_sql(manager, DROP_STARTED).await?;
        exec_sql(manager, DROP_SCHEDULED).await
    }
}
