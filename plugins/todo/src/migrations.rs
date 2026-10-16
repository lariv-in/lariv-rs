use sea_orm_migration::prelude::*;

use super::TodoTag;

mod m00001_create_todos;
mod m00002_todo_client_id;
mod m00003_todo_title_rev;
mod m00004_todo_pages;
mod m00005_todo_order;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_todos::Migration),
            Box::new(m00002_todo_client_id::Migration),
            Box::new(m00003_todo_title_rev::Migration),
            Box::new(m00004_todo_pages::Migration),
            Box::new(m00005_todo_order::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: TodoTag;
    migrator: Migrator;
}

/// Apply the to-do schema on a connection that already has a `users` table.
pub async fn migrate(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
    use sea_orm_migration::prelude::*;

    use super::{
        m00001_create_todos, m00002_todo_client_id, m00003_todo_title_rev, m00004_todo_pages,
        m00005_todo_order,
    };

    #[tokio::test]
    async fn migrator_applies_on_sqlite() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        db.execute_unprepared("CREATE TABLE users (id integer NOT NULL PRIMARY KEY AUTOINCREMENT)")
            .await
            .expect("users");
        super::Migrator::up(&db, None).await.expect("migrator");
        let pages = scalar(&db, "SELECT COUNT(*) AS n FROM todo_pages").await;
        assert_eq!(pages, 0);
    }

    #[tokio::test]
    async fn existing_todos_land_on_an_untitled_page_per_owner() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        db.execute_unprepared("CREATE TABLE users (id integer NOT NULL PRIMARY KEY AUTOINCREMENT)")
            .await
            .expect("users");
        db.execute_unprepared("INSERT INTO users (id) VALUES (1), (2)")
            .await
            .expect("seed users");
        let manager = SchemaManager::new(&db);
        m00001_create_todos::Migration
            .up(&manager)
            .await
            .expect("m00001");
        m00002_todo_client_id::Migration
            .up(&manager)
            .await
            .expect("m00002");
        m00003_todo_title_rev::Migration
            .up(&manager)
            .await
            .expect("m00003");
        db.execute_unprepared(
            "INSERT INTO todos (user_id, title, checked, title_rev, client_id) VALUES
             (1, 'a', 1, 2, 'c-a'),
             (1, 'b', 0, 0, NULL),
             (2, 'c', 0, 0, 'c-c')",
        )
        .await
        .expect("seed todos");

        m00004_todo_pages::Migration
            .up(&manager)
            .await
            .expect("m00004");

        let pages = scalar(
            &db,
            "SELECT COUNT(*) AS n FROM todo_pages WHERE title = 'Untitled'",
        )
        .await;
        assert_eq!(pages, 2);
        let linked = scalar(
            &db,
            "SELECT COUNT(*) AS n FROM todos t
             JOIN todo_pages p ON p.id = t.todo_page_id
             WHERE t.user_id = p.created_by_id AND p.title = 'Untitled'",
        )
        .await;
        assert_eq!(linked, 3);
        let crossed = scalar(
            &db,
            "SELECT COUNT(*) AS n FROM todos t
             JOIN todo_pages p ON p.id = t.todo_page_id
             WHERE t.user_id <> p.created_by_id",
        )
        .await;
        assert_eq!(crossed, 0);
        let kept = scalar(
            &db,
            "SELECT COUNT(*) AS n FROM todos WHERE title = 'a' AND title_rev = 2 AND client_id = 'c-a' AND checked = 1",
        )
        .await;
        assert_eq!(kept, 1);
        let required = scalar(
            &db,
            "SELECT \"notnull\" AS n FROM pragma_table_info('todos') WHERE name = 'todo_page_id'",
        )
        .await;
        assert_eq!(required, 1);

        m00005_todo_order::Migration
            .up(&manager)
            .await
            .expect("m00005");
        let first = scalar(&db, "SELECT sort_order AS n FROM todos WHERE title = 'a'").await;
        let second = scalar(&db, "SELECT sort_order AS n FROM todos WHERE title = 'b'").await;
        let third = scalar(&db, "SELECT sort_order AS n FROM todos WHERE title = 'c'").await;
        assert_eq!(first, 0);
        assert_eq!(second, 1);
        assert_eq!(third, 0);
        m00005_todo_order::Migration
            .down(&manager)
            .await
            .expect("down order");

        m00004_todo_pages::Migration
            .down(&manager)
            .await
            .expect("down");
        let still = scalar(&db, "SELECT COUNT(*) AS n FROM todos").await;
        assert_eq!(still, 3);
        let column = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT 1 AS n FROM pragma_table_info('todos') WHERE name = 'todo_page_id'",
            ))
            .await
            .expect("pragma");
        assert!(column.is_none());
        let table = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT 1 AS n FROM sqlite_master WHERE type = 'table' AND name = 'todo_pages'",
            ))
            .await
            .expect("sqlite_master");
        assert!(table.is_none());
    }

    async fn scalar(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
        let row = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                sql.to_owned(),
            ))
            .await
            .expect("query")
            .expect("row");
        row.try_get("", "n").expect("n")
    }
}
