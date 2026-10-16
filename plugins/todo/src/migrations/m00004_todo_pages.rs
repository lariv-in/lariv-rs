//! To-do pages, and a required page on every to-do.
//!
//! Each user who already has rows gets one page titled "Untitled", and those
//! rows point at that page. A page is visible only to its creator, so owners
//! do not share a page.

use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum TodoPages {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Title,
    CreatedById,
}

#[derive(Iden)]
enum Todos {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    Checked,
    Title,
    TitleRev,
    ClientId,
    TodoPageId,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

const TEMP_TODOS: &str = "todos__page";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_pages(manager).await?;
        exec_sql(
            manager,
            r#"INSERT INTO todo_pages (title, created_by_id)
               SELECT 'Untitled', user_id FROM todos GROUP BY user_id"#,
        )
        .await?;
        if is_postgres(manager) {
            add_page_fk_postgres(manager).await?;
        } else {
            rebuild_sqlite(manager, true).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            drop_page_fk_postgres(manager).await?;
        } else {
            rebuild_sqlite(manager, false).await?;
        }
        manager
            .drop_table(Table::drop().table(TodoPages::Table).to_owned())
            .await?;
        Ok(())
    }
}

async fn create_pages(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(TodoPages::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(TodoPages::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(TodoPages::CreatedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(TodoPages::UpdatedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(TodoPages::Title).text().not_null())
                .col(
                    ColumnDef::new(TodoPages::CreatedById)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_todo_pages_created_by_id")
                        .from(TodoPages::Table, TodoPages::CreatedById)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_todo_pages_created_by_id")
                .table(TodoPages::Table)
                .col(TodoPages::CreatedById)
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn add_page_fk_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(Todos::Table)
                .add_column(ColumnDef::new(Todos::TodoPageId).big_integer().null())
                .to_owned(),
        )
        .await?;
    exec_sql(
        manager,
        r#"UPDATE todos
           SET todo_page_id = (
               SELECT todo_pages.id FROM todo_pages
               WHERE todo_pages.created_by_id = todos.user_id
           )"#,
    )
    .await?;
    exec_sql(
        manager,
        "ALTER TABLE todos ALTER COLUMN todo_page_id SET NOT NULL",
    )
    .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_todos_todo_page_id")
                .from(Todos::Table, Todos::TodoPageId)
                .to(TodoPages::Table, TodoPages::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_todos_todo_page_id")
                .table(Todos::Table)
                .col(Todos::TodoPageId)
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn drop_page_fk_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .drop_foreign_key(
            ForeignKey::drop()
                .name("fk_todos_todo_page_id")
                .table(Todos::Table)
                .to_owned(),
        )
        .await?;
    manager
        .drop_index(
            Index::drop()
                .name("idx_todos_todo_page_id")
                .table(Todos::Table)
                .to_owned(),
        )
        .await?;
    manager
        .alter_table(
            Table::alter()
                .table(Todos::Table)
                .drop_column(Todos::TodoPageId)
                .to_owned(),
        )
        .await?;
    Ok(())
}

/// SQLite cannot add a `NOT NULL` foreign key to an existing table. Copy `todos`
/// into a new table, then swap the names. `with_page` includes `todo_page_id`.
async fn rebuild_sqlite(manager: &SchemaManager<'_>, with_page: bool) -> Result<(), DbErr> {
    exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
    let mut create = Table::create();
    create
        .table(Alias::new(TEMP_TODOS))
        .col(
            ColumnDef::new(Todos::Id)
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        .col(ColumnDef::new(Todos::CreatedAt).timestamp_with_time_zone())
        .col(ColumnDef::new(Todos::UpdatedAt).timestamp_with_time_zone())
        .col(ColumnDef::new(Todos::UserId).big_integer().not_null())
        .col(
            ColumnDef::new(Todos::Checked)
                .boolean()
                .not_null()
                .default(false),
        )
        .col(ColumnDef::new(Todos::Title).text().not_null())
        .col(
            ColumnDef::new(Todos::TitleRev)
                .big_integer()
                .not_null()
                .default(0),
        )
        .col(ColumnDef::new(Todos::ClientId).text())
        .foreign_key(
            ForeignKey::create()
                .name("fk_todos_user_id")
                .from(Alias::new(TEMP_TODOS), Todos::UserId)
                .to(Users::Table, Users::Id)
                .on_delete(ForeignKeyAction::Cascade),
        );
    if with_page {
        create
            .col(ColumnDef::new(Todos::TodoPageId).big_integer().not_null())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_todos_todo_page_id")
                    .from(Alias::new(TEMP_TODOS), Todos::TodoPageId)
                    .to(TodoPages::Table, TodoPages::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            );
    }
    manager.create_table(create.to_owned()).await?;

    let copy = if with_page {
        r#"INSERT INTO "todos__page" (
               "id", "created_at", "updated_at", "user_id", "checked", "title",
               "title_rev", "client_id", "todo_page_id"
           )
           SELECT t."id", t."created_at", t."updated_at", t."user_id", t."checked",
                  t."title", t."title_rev", t."client_id", p."id"
           FROM "todos" AS t
           INNER JOIN "todo_pages" AS p ON p."created_by_id" = t."user_id""#
    } else {
        r#"INSERT INTO "todos__page" (
               "id", "created_at", "updated_at", "user_id", "checked", "title",
               "title_rev", "client_id"
           )
           SELECT "id", "created_at", "updated_at", "user_id", "checked", "title",
                  "title_rev", "client_id"
           FROM "todos""#
    };
    exec_sql(manager, copy).await?;
    exec_sql(manager, r#"DROP TABLE "todos""#).await?;
    exec_sql(manager, r#"ALTER TABLE "todos__page" RENAME TO "todos""#).await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'todos' WHERE name = 'todos__page'",
    )
    .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_todos_user_id")
                .table(Todos::Table)
                .col(Todos::UserId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_todos_client_id")
                .table(Todos::Table)
                .col(Todos::ClientId)
                .unique()
                .to_owned(),
        )
        .await?;
    if with_page {
        manager
            .create_index(
                Index::create()
                    .name("idx_todos_todo_page_id")
                    .table(Todos::Table)
                    .col(Todos::TodoPageId)
                    .to_owned(),
            )
            .await?;
    }
    exec_sql(manager, "PRAGMA foreign_keys = ON").await
}
