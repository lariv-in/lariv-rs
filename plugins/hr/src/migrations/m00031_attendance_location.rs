use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrPreferences {
    Table,
    AttendanceLatitude,
    AttendanceLongitude,
    AttendanceRangeMeters,
}

#[derive(DeriveIden)]
enum HrAttendances {
    Table,
    StartedLatitude,
    StartedLongitude,
    EndedLatitude,
    EndedLongitude,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_double(
            manager,
            HrPreferences::Table,
            HrPreferences::AttendanceRangeMeters,
        )
        .await?;
        if is_postgres(manager) {
            exec_sql(manager, "CREATE EXTENSION IF NOT EXISTS postgis").await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_preferences ADD COLUMN attendance_position geography(Point, 4326)",
            )
            .await?;
            exec_sql(
                manager,
                "CREATE INDEX idx_hr_preferences_attendance_position ON hr_preferences USING gist (attendance_position)",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_attendances ADD COLUMN started_position geography(Point, 4326)",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_attendances ADD COLUMN ended_position geography(Point, 4326)",
            )
            .await?;
        } else {
            add_double(
                manager,
                HrPreferences::Table,
                HrPreferences::AttendanceLatitude,
            )
            .await?;
            add_double(
                manager,
                HrPreferences::Table,
                HrPreferences::AttendanceLongitude,
            )
            .await?;
            add_double(
                manager,
                HrAttendances::Table,
                HrAttendances::StartedLatitude,
            )
            .await?;
            add_double(
                manager,
                HrAttendances::Table,
                HrAttendances::StartedLongitude,
            )
            .await?;
            add_double(manager, HrAttendances::Table, HrAttendances::EndedLatitude).await?;
            add_double(manager, HrAttendances::Table, HrAttendances::EndedLongitude).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_sql(
                manager,
                "DROP INDEX IF EXISTS idx_hr_preferences_attendance_position",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_preferences DROP COLUMN IF EXISTS attendance_position",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_attendances DROP COLUMN IF EXISTS started_position",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE hr_attendances DROP COLUMN IF EXISTS ended_position",
            )
            .await?;
        } else {
            drop_column(
                manager,
                HrPreferences::Table,
                HrPreferences::AttendanceLatitude,
            )
            .await?;
            drop_column(
                manager,
                HrPreferences::Table,
                HrPreferences::AttendanceLongitude,
            )
            .await?;
            drop_column(
                manager,
                HrAttendances::Table,
                HrAttendances::StartedLatitude,
            )
            .await?;
            drop_column(
                manager,
                HrAttendances::Table,
                HrAttendances::StartedLongitude,
            )
            .await?;
            drop_column(manager, HrAttendances::Table, HrAttendances::EndedLatitude).await?;
            drop_column(manager, HrAttendances::Table, HrAttendances::EndedLongitude).await?;
        }
        drop_column(
            manager,
            HrPreferences::Table,
            HrPreferences::AttendanceRangeMeters,
        )
        .await?;
        Ok(())
    }
}

async fn add_double<T, C>(manager: &SchemaManager<'_>, table: T, column: C) -> Result<(), DbErr>
where
    T: IntoIden + 'static,
    C: IntoIden + 'static,
{
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .add_column(ColumnDef::new(column).double().null())
                .to_owned(),
        )
        .await
}

async fn drop_column<T, C>(manager: &SchemaManager<'_>, table: T, column: C) -> Result<(), DbErr>
where
    T: IntoIden + 'static,
    C: IntoIden + 'static,
{
    manager
        .alter_table(Table::alter().table(table).drop_column(column).to_owned())
        .await
}
