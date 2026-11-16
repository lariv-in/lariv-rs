use sea_orm::DatabaseBackend;
use sea_orm_migration::prelude::*;

use super::INSTALL_SQL;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Logs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Logs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Logs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Logs::TableName).text().not_null())
                    .col(ColumnDef::new(Logs::RowId).big_integer().not_null())
                    .col(ColumnDef::new(Logs::LogKind).string_len(16).not_null())
                    .col(ColumnDef::new(Logs::UserId).big_integer().null())
                    .col(ColumnDef::new(Logs::Description).json_binary().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_logs_table_name_row_id")
                    .table(Logs::Table)
                    .col(Logs::TableName)
                    .col(Logs::RowId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_logs_created_at")
                    .table(Logs::Table)
                    .col(Logs::CreatedAt)
                    .to_owned(),
            )
            .await?;

        if manager.get_database_backend() == DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(INSTALL_SQL)
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute_unprepared(
                    "DROP FUNCTION IF EXISTS logging_install_triggers() CASCADE;
                     DROP FUNCTION IF EXISTS logging_audit_row() CASCADE;",
                )
                .await?;
        }
        manager
            .drop_table(Table::drop().table(Logs::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Logs {
    Table,
    Id,
    CreatedAt,
    TableName,
    RowId,
    LogKind,
    UserId,
    Description,
}
