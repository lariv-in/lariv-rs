use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use lariv_plugin_users::roles::Superuser;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrEmployees {
    Table,
    ManagerId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .add_column(ColumnDef::new(HrEmployees::ManagerId).big_integer().null())
                    .to_owned(),
            )
            .await?;

        // Existing rows take the earliest superuser. The form does not preselect anyone.
        exec_sql(
            manager,
            &format!(
                "UPDATE hr_employees SET manager_id = (\
                    SELECT id FROM users WHERE role = '{role}' ORDER BY id ASC LIMIT 1\
                )",
                role = Superuser::NAME
            ),
        )
        .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_employees_manager_id")
                    .table(HrEmployees::Table)
                    .col(HrEmployees::ManagerId)
                    .to_owned(),
            )
            .await?;

        // SQLite cannot add a foreign key to an existing table.
        if is_postgres(manager) {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_hr_employees_manager_id")
                        .from(HrEmployees::Table, HrEmployees::ManagerId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::SetNull)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_hr_employees_manager_id")
                        .table(HrEmployees::Table)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_hr_employees_manager_id")
                    .table(HrEmployees::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .drop_column(HrEmployees::ManagerId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
