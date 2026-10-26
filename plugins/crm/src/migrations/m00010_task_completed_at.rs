use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

const UP_POSTGRES: &str = r#"
DO $$
BEGIN
    IF to_regclass('public.crm_tasks') IS NULL THEN
        RETURN;
    END IF;
    ALTER TABLE crm_tasks ADD COLUMN IF NOT EXISTS completed_at timestamptz;
END $$;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmTasks {
    Table,
    CompletedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return exec_sql(manager, UP_POSTGRES).await;
        }

        manager
            .alter_table(
                Table::alter()
                    .table(CrmTasks::Table)
                    .add_column(ColumnDef::new(CrmTasks::CompletedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CrmTasks::Table)
                    .drop_column(CrmTasks::CompletedAt)
                    .to_owned(),
            )
            .await
    }
}
