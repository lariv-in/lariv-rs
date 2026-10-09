use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrLeaveCalcPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    LeaveType,
    ConsecutiveRequired,
    LeaveAllocated,
    ScheduleKind,
    Month,
    DaySpec,
    Timezone,
}

#[derive(DeriveIden)]
enum HrLeaveAttendanceUses {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    LeaveType,
    AttendanceDate,
}

#[derive(DeriveIden)]
enum HrLeaveEvaluationRuns {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    LeaveType,
    PeriodKey,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrLeaveCalcPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::LeaveType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::ConsecutiveRequired)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::LeaveAllocated)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::ScheduleKind)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(ColumnDef::new(HrLeaveCalcPreferences::Month).integer())
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::DaySpec)
                            .string_len(8)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveCalcPreferences::Timezone)
                            .string_len(64)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_hr_leave_calc_preferences_leave_type")
                    .table(HrLeaveCalcPreferences::Table)
                    .col(HrLeaveCalcPreferences::LeaveType)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrLeaveAttendanceUses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrLeaveAttendanceUses::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveAttendanceUses::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveAttendanceUses::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveAttendanceUses::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveAttendanceUses::LeaveType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveAttendanceUses::AttendanceDate)
                            .date()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_leave_attendance_uses_user_id")
                            .from(HrLeaveAttendanceUses::Table, HrLeaveAttendanceUses::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_hr_leave_attendance_uses_user_type_date")
                    .table(HrLeaveAttendanceUses::Table)
                    .col(HrLeaveAttendanceUses::UserId)
                    .col(HrLeaveAttendanceUses::LeaveType)
                    .col(HrLeaveAttendanceUses::AttendanceDate)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrLeaveEvaluationRuns::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrLeaveEvaluationRuns::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveEvaluationRuns::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveEvaluationRuns::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveEvaluationRuns::LeaveType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveEvaluationRuns::PeriodKey)
                            .string_len(32)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_hr_leave_evaluation_runs_type_period")
                    .table(HrLeaveEvaluationRuns::Table)
                    .col(HrLeaveEvaluationRuns::LeaveType)
                    .col(HrLeaveEvaluationRuns::PeriodKey)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrLeaveEvaluationRuns::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(HrLeaveAttendanceUses::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(HrLeaveCalcPreferences::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
