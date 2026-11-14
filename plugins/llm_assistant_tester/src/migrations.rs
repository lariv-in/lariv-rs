use sea_orm_migration::prelude::*;

use super::LlmAssistantTesterTag;

mod m20261113_000001_create_test_cases;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261113_000001_create_test_cases::Migration)]
    }
}

lariv_core::define_register_migrations! {
    plugin: LlmAssistantTesterTag;
    migrator: Migrator;
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
    use sea_orm_migration::MigratorTrait;

    #[tokio::test]
    async fn migrator_applies_on_sqlite() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        db.execute_unprepared("CREATE TABLE users (id integer NOT NULL PRIMARY KEY AUTOINCREMENT)")
            .await
            .expect("users");
        super::Migrator::up(&db, None).await.expect("migrator");
        let count = scalar(&db, "SELECT COUNT(*) AS n FROM test_cases").await;
        assert_eq!(count, 0);
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
