use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrLeaveCalcPreferences {
    Table,
    LeaveAllocated,
    AllocationFormula,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // m00023 was edited after it had already created `allocation_formula`.
        // Databases that applied that version never received `leave_allocated`.
        if !manager
            .has_column("hr_leave_calc_preferences", "leave_allocated")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(HrLeaveCalcPreferences::Table)
                        .add_column(
                            ColumnDef::new(HrLeaveCalcPreferences::LeaveAllocated)
                                .big_integer()
                                .not_null()
                                .default(0),
                        )
                        .to_owned(),
                )
                .await?;
        }

        if manager
            .has_column("hr_leave_calc_preferences", "allocation_formula")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(HrLeaveCalcPreferences::Table)
                        .drop_column(HrLeaveCalcPreferences::AllocationFormula)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("hr_leave_calc_preferences", "allocation_formula")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(HrLeaveCalcPreferences::Table)
                        .add_column(
                            ColumnDef::new(HrLeaveCalcPreferences::AllocationFormula)
                                .text()
                                .not_null()
                                .default(""),
                        )
                        .to_owned(),
                )
                .await?;
        }

        if manager
            .has_column("hr_leave_calc_preferences", "leave_allocated")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(HrLeaveCalcPreferences::Table)
                        .drop_column(HrLeaveCalcPreferences::LeaveAllocated)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
