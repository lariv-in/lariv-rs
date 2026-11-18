//! Single-row table tracking the last successful Razorpay IFSC directory sync.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_POSTGRES: &str = r#"
CREATE TABLE IF NOT EXISTS bank_directory_syncs (
    id BIGINT PRIMARY KEY,
    last_success_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT bank_directory_syncs_singleton CHECK (id = 1)
);
"#;

const DOWN_POSTGRES: &str = r#"
DROP TABLE IF EXISTS bank_directory_syncs;
"#;

#[derive(DeriveIden)]
enum BankDirectorySyncs {
    Table,
    Id,
    LastSuccessAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => exec_sql(manager, UP_POSTGRES).await,
            _ => {
                manager
                    .create_table(
                        Table::create()
                            .table(BankDirectorySyncs::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(BankDirectorySyncs::Id)
                                    .big_integer()
                                    .not_null()
                                    .primary_key(),
                            )
                            .col(
                                ColumnDef::new(BankDirectorySyncs::LastSuccessAt)
                                    .timestamp_with_time_zone()
                                    .not_null(),
                            )
                            .to_owned(),
                    )
                    .await
            }
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => exec_sql(manager, DOWN_POSTGRES).await,
            _ => {
                manager
                    .drop_table(Table::drop().table(BankDirectorySyncs::Table).to_owned())
                    .await
            }
        }
    }
}
