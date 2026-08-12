use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_unprepared(sql)
        .await
        .map(|_| ())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() != sea_orm::DatabaseBackend::Postgres {
            return Ok(());
        }
        // m00018 updated invoice rows but missed payment_terms.type; fix if table still exists.
        execute(
            manager,
            r#"
            DO $$
            BEGIN
              IF EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = 'payment_terms'
              ) THEN
                UPDATE payment_terms
                SET type = replace(type, 'p_uniquity_finance_', 'p_finance_')
                WHERE type LIKE 'p_uniquity_finance_%';
              END IF;
            END $$;
            "#,
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() != sea_orm::DatabaseBackend::Postgres {
            return Ok(());
        }
        execute(
            manager,
            r#"
            DO $$
            BEGIN
              IF EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = 'payment_terms'
              ) THEN
                UPDATE payment_terms
                SET type = replace(type, 'p_finance_', 'p_uniquity_finance_')
                WHERE type LIKE 'p_finance_%';
              END IF;
            END $$;
            "#,
        )
        .await
    }
}
