//! Audit-log schema. Trigger attachment runs again at the end of every migration pass.

use futures_util::future::BoxFuture;
use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::*;

use super::LoggingTag;

mod m00001_create_logs;
mod m00002_index_logs_user_id;
mod m00003_logs_description_jsonb;

pub const INSTALL_SQL: &str = include_str!("install.sql");

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_logs::Migration),
            Box::new(m00002_index_logs_user_id::Migration),
            Box::new(m00003_logs_description_jsonb::Migration),
        ]
    }
}

/// Replace the audit functions and attach triggers to every eligible table.
///
/// The migration that creates the functions runs once. This hook runs after every
/// pass, so a table added by a later migration is covered on the next migrate.
pub fn after_migrate(db: &DatabaseConnection) -> BoxFuture<'_, Result<(), DbErr>> {
    Box::pin(async move {
        if db.get_database_backend() != sea_orm::DatabaseBackend::Postgres {
            return Ok(());
        }
        db.execute_unprepared(INSTALL_SQL).await?;
        db.execute_unprepared("SELECT logging_install_triggers()")
            .await?;
        Ok(())
    })
}

lariv_core::define_register_migrations! {
    plugin: LoggingTag;
    migrator: Migrator;
    after: after_migrate;
}

/// The SQL that defines the audit functions. Tests read it directly.
pub fn install_sql() -> &'static str {
    INSTALL_SQL
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
    use sea_orm_migration::MigratorTrait;

    #[test]
    fn install_sql_defines_the_audit_trigger() {
        let sql = super::install_sql();
        assert!(sql.contains("logging_audit_row"));
        assert!(sql.contains("logging_install_triggers"));
        assert!(sql.contains("CREATE TRIGGER"));
        assert!(sql.contains("'before'"));
        assert!(sql.contains("'after'"));
    }

    #[tokio::test]
    async fn migrator_creates_logs_on_sqlite() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        super::Migrator::up(&db, None).await.expect("migrator");
        let columns = scalar(
            &db,
            "SELECT COUNT(*) AS n FROM pragma_table_info('logs')
             WHERE name IN ('table_name', 'row_id', 'log_kind', 'user_id', 'description', 'created_at')",
        )
        .await;
        assert_eq!(columns, 6);
        let user_index = scalar(
            &db,
            "SELECT COUNT(*) AS n FROM pragma_index_list('logs') WHERE name = 'idx_logs_user_id'",
        )
        .await;
        assert_eq!(user_index, 1);
        super::after_migrate(&db)
            .await
            .expect("sqlite hook is a no-op");
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
