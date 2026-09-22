use crate::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum TaskStatuses {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Color,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Title,
    Description,
    AssignedToId,
    StatusId,
    Priority,
    DueDatetime,
}

#[derive(DeriveIden)]
enum TaskLogs {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    TaskId,
    Description,
    Datetime,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

const SEED_TODO: i32 = 0x0063_66F1;
const SEED_IN_PROGRESS: i32 = 0x00F5_9E0B;
const SEED_DONE: i32 = 0x0022_C55E;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TaskStatuses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskStatuses::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskStatuses::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskStatuses::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskStatuses::Name).text().not_null())
                    .col(ColumnDef::new(TaskStatuses::Color).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_task_statuses_name")
                    .table(TaskStatuses::Table)
                    .col(TaskStatuses::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Tasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tasks::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tasks::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Tasks::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Tasks::Title).text().not_null())
                    .col(
                        ColumnDef::new(Tasks::Description)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Tasks::AssignedToId).big_integer().not_null())
                    .col(ColumnDef::new(Tasks::StatusId).big_integer().not_null())
                    .col(
                        ColumnDef::new(Tasks::Priority)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Tasks::DueDatetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tasks_assigned_to_id")
                            .from(Tasks::Table, Tasks::AssignedToId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tasks_status_id")
                            .from(Tasks::Table, Tasks::StatusId)
                            .to(TaskStatuses::Table, TaskStatuses::Id)
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
                    .name("idx_tasks_assigned_to_id")
                    .table(Tasks::Table)
                    .col(Tasks::AssignedToId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tasks_status_id")
                    .table(Tasks::Table)
                    .col(Tasks::StatusId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TaskLogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskLogs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskLogs::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskLogs::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskLogs::TaskId).big_integer().not_null())
                    .col(ColumnDef::new(TaskLogs::Description).text().not_null())
                    .col(
                        ColumnDef::new(TaskLogs::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_logs_task_id")
                            .from(TaskLogs::Table, TaskLogs::TaskId)
                            .to(Tasks::Table, Tasks::Id)
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
                    .name("idx_task_logs_task_id")
                    .table(TaskLogs::Table)
                    .col(TaskLogs::TaskId)
                    .to_owned(),
            )
            .await?;

        seed_statuses(manager).await?;
        copy_from_crm_if_present(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TaskLogs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tasks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TaskStatuses::Table).to_owned())
            .await
    }
}

async fn seed_statuses(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let now_expr = if is_postgres(manager) {
        "NOW()"
    } else {
        "datetime('now')"
    };
    exec_sql(
        manager,
        &format!(
            r#"
            INSERT INTO task_statuses (created_at, updated_at, name, color)
            VALUES
                ({now}, {now}, 'To Do', {todo}),
                ({now}, {now}, 'In Progress', {in_progress}),
                ({now}, {now}, 'Done', {done})
            "#,
            now = now_expr,
            todo = SEED_TODO,
            in_progress = SEED_IN_PROGRESS,
            done = SEED_DONE,
        ),
    )
    .await
}

async fn copy_from_crm_if_present(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if !manager.has_table("crm_tasks").await? {
        return Ok(());
    }
    let postgres = is_postgres(manager);
    let due_expr = if postgres {
        "COALESCE(t.due_date::timestamptz, t.created_at, NOW())"
    } else {
        "COALESCE(t.due_date, t.created_at, datetime('now'))"
    };
    let has_completed = manager.has_table("crm_completed_tasks").await?;
    let status_expr = if has_completed {
        "(CASE WHEN c.task_id IS NOT NULL THEN (SELECT id FROM task_statuses WHERE name = 'Done') ELSE (SELECT id FROM task_statuses WHERE name = 'To Do') END)"
    } else {
        "(SELECT id FROM task_statuses WHERE name = 'To Do')"
    };
    let join = if has_completed {
        "LEFT JOIN crm_completed_tasks c ON c.task_id = t.id"
    } else {
        ""
    };
    exec_sql(
        manager,
        &format!(
            r#"
            INSERT INTO tasks (
                id, created_at, updated_at, title, description,
                assigned_to_id, status_id, priority, due_datetime
            )
            SELECT
                t.id,
                t.created_at,
                t.updated_at,
                t.title,
                COALESCE(t.description, ''),
                t.assigned_to_id,
                {status},
                0,
                {due}
            FROM crm_tasks t
            {join}
            "#,
            status = status_expr,
            due = due_expr,
            join = join,
        ),
    )
    .await?;

    if postgres {
        exec_sql(
            manager,
            r#"
            SELECT setval(
                pg_get_serial_sequence('tasks', 'id'),
                COALESCE((SELECT MAX(id) FROM tasks), 1)
            )
            "#,
        )
        .await?;
    }

    if has_completed {
        exec_sql(
            manager,
            r#"
            INSERT INTO task_logs (created_at, updated_at, task_id, description, datetime)
            SELECT c.created_at, c.created_at, c.task_id, 'Completed', c.completed_at
            FROM crm_completed_tasks c
            INNER JOIN tasks t ON t.id = c.task_id
            "#,
        )
        .await?;
        if postgres {
            exec_sql(
                manager,
                r#"
                SELECT setval(
                    pg_get_serial_sequence('task_logs', 'id'),
                    COALESCE((SELECT MAX(id) FROM task_logs), 1)
                )
                "#,
            )
            .await?;
        }
    }
    Ok(())
}
