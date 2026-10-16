//! Position of each to-do on its page.
//!
//! Existing rows keep their previous order: created time, then id, starting at 0
//! within each page.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, Statement, TryGetable};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Todos {
    Table,
    SortOrder,
    TodoPageId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Todos::Table)
                    .add_column(
                        ColumnDef::new(Todos::SortOrder)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        backfill_order(manager).await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_todos_page_sort_order")
                    .table(Todos::Table)
                    .col(Todos::TodoPageId)
                    .col(Todos::SortOrder)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_todos_page_sort_order")
                    .table(Todos::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Todos::Table)
                    .drop_column(Todos::SortOrder)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

async fn backfill_order(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    let rows = manager
        .get_connection()
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT id, todo_page_id FROM todos ORDER BY todo_page_id, created_at IS NULL, created_at, id",
        ))
        .await?;
    let mut page_id: Option<i64> = None;
    let mut order = 0_i64;
    for row in rows {
        let id: i64 = try_get(&row, "id")?;
        let row_page: i64 = try_get(&row, "todo_page_id")?;
        if page_id != Some(row_page) {
            page_id = Some(row_page);
            order = 0;
        }
        exec_sql(
            manager,
            &format!("UPDATE todos SET sort_order = {order} WHERE id = {id}"),
        )
        .await?;
        order += 1;
    }
    Ok(())
}

fn try_get<T: TryGetable>(row: &sea_orm::QueryResult, column: &str) -> Result<T, DbErr> {
    row.try_get("", column)
}
