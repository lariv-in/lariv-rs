use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmTasks {
    Table,
    Id,
    CompletedAt,
}

#[derive(DeriveIden)]
enum CrmCompletedTasks {
    Table,
    Id,
    CreatedAt,
    TaskId,
    CompletedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CrmCompletedTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmCompletedTasks::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmCompletedTasks::CreatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(CrmCompletedTasks::TaskId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrmCompletedTasks::CompletedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_completed_tasks_task_id")
                            .from(CrmCompletedTasks::Table, CrmCompletedTasks::TaskId)
                            .to(CrmTasks::Table, CrmTasks::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_crm_completed_tasks_task_id")
                    .table(CrmCompletedTasks::Table)
                    .col(CrmCompletedTasks::TaskId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                INSERT INTO crm_completed_tasks (created_at, task_id, completed_at)
                SELECT completed_at, id, completed_at
                FROM crm_tasks
                WHERE completed_at IS NOT NULL
                "#,
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmTasks::Table)
                    .drop_column(CrmTasks::CompletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CrmTasks::Table)
                    .add_column(ColumnDef::new(CrmTasks::CompletedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                UPDATE crm_tasks
                SET completed_at = (
                    SELECT c.completed_at
                    FROM crm_completed_tasks c
                    WHERE c.task_id = crm_tasks.id
                )
                "#,
            )
            .await?;

        manager
            .drop_table(Table::drop().table(CrmCompletedTasks::Table).to_owned())
            .await
    }
}
