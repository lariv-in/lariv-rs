use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrLeaveCalcPreferences {
    Table,
    PunchInWindowMinutes,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("hr_leave_calc_preferences", "punch_in_window_minutes")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(HrLeaveCalcPreferences::Table)
                        .add_column(
                            ColumnDef::new(HrLeaveCalcPreferences::PunchInWindowMinutes)
                                .big_integer()
                                .not_null()
                                .default(15),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager
            .has_column("hr_leave_calc_preferences", "punch_in_window_minutes")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(HrLeaveCalcPreferences::Table)
                        .drop_column(HrLeaveCalcPreferences::PunchInWindowMinutes)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
