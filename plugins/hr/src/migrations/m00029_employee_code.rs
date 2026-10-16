use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrPreferences {
    Table,
    EmployeeIdFormat,
}

#[derive(DeriveIden)]
enum HrEmployees {
    Table,
    EmployeeCode,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrPreferences::Table)
                    .add_column(
                        ColumnDef::new(HrPreferences::EmployeeIdFormat)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .add_column(
                        ColumnDef::new(HrEmployees::EmployeeCode)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        let backfill = if is_postgres(manager) {
            "UPDATE hr_employees SET employee_code = to_char(COALESCE(date_of_joining, (hired_at AT TIME ZONE 'UTC')::date), 'YYYY') || lpad(user_id::text, 5, '0') WHERE employee_code = ''"
        } else {
            "UPDATE hr_employees SET employee_code = strftime('%Y', COALESCE(date_of_joining, hired_at)) || printf('%05d', user_id) WHERE employee_code = ''"
        };
        exec_sql(manager, backfill).await?;
        exec_sql(
            manager,
            "CREATE UNIQUE INDEX IF NOT EXISTS uix_hr_employees_employee_code ON hr_employees (employee_code) WHERE employee_code <> ''",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "DROP INDEX IF EXISTS uix_hr_employees_employee_code",
        )
        .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .drop_column(HrEmployees::EmployeeCode)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrPreferences::Table)
                    .drop_column(HrPreferences::EmployeeIdFormat)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
