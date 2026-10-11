use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PeriodicTasks {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Title,
    Description,
    AssignedToId,
    StatusId,
    ParentId,
    Priority,
    DueDatetime,
    Frequency,
    LastAddedAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum TaskStatuses {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PeriodicTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PeriodicTasks::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PeriodicTasks::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PeriodicTasks::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PeriodicTasks::Title).text().not_null())
                    .col(
                        ColumnDef::new(PeriodicTasks::Description)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(PeriodicTasks::AssignedToId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeriodicTasks::StatusId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PeriodicTasks::ParentId).big_integer().null())
                    .col(
                        ColumnDef::new(PeriodicTasks::Priority)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(PeriodicTasks::DueDatetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeriodicTasks::Frequency)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeriodicTasks::LastAddedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_periodic_tasks_assigned_to_id")
                            .from(PeriodicTasks::Table, PeriodicTasks::AssignedToId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_periodic_tasks_status_id")
                            .from(PeriodicTasks::Table, PeriodicTasks::StatusId)
                            .to(TaskStatuses::Table, TaskStatuses::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_periodic_tasks_parent_id")
                            .from(PeriodicTasks::Table, PeriodicTasks::ParentId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PeriodicTasks::Table).to_owned())
            .await
    }
}
