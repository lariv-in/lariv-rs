use crate::db::migration_sql::{exec_sql, is_postgres};
use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;
use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Forms {
    Table,
    Uid,
    AccessStatus,
}

#[derive(DeriveIden)]
enum AccessStatusType {
    #[sea_orm(iden = "access_status")]
    Enum,
    #[sea_orm(iden = "NotAcceptingSubmissions")]
    NotAcceptingSubmissions,
    #[sea_orm(iden = "AnyoneWithLink")]
    AnyoneWithLink,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            manager
                .create_type(
                    Type::create()
                        .as_enum(AccessStatusType::Enum)
                        .values([
                            AccessStatusType::NotAcceptingSubmissions,
                            AccessStatusType::AnyoneWithLink,
                        ])
                        .to_owned(),
                )
                .await?;
            manager
                .alter_table(
                    Table::alter()
                        .table(Forms::Table)
                        .add_column(
                            ColumnDef::new(Forms::AccessStatus)
                                .custom(AccessStatusType::Enum)
                                .not_null()
                                .default("NotAcceptingSubmissions"),
                        )
                        .to_owned(),
                )
                .await?;
            manager
                .alter_table(
                    Table::alter()
                        .table(Forms::Table)
                        .add_column(ColumnDef::new(Forms::Uid).uuid())
                        .to_owned(),
                )
                .await?;
        } else {
            manager
                .alter_table(
                    Table::alter()
                        .table(Forms::Table)
                        .add_column(
                            ColumnDef::new(Forms::AccessStatus)
                                .string_len(64)
                                .not_null()
                                .default("NotAcceptingSubmissions"),
                        )
                        .to_owned(),
                )
                .await?;
            manager
                .alter_table(
                    Table::alter()
                        .table(Forms::Table)
                        .add_column(ColumnDef::new(Forms::Uid).string_len(36))
                        .to_owned(),
                )
                .await?;
        }

        backfill_uids(manager).await?;

        if is_postgres(manager) {
            exec_sql(manager, "ALTER TABLE forms ALTER COLUMN uid SET NOT NULL").await?;
        }

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_forms_uid")
                    .table(Forms::Table)
                    .col(Forms::Uid)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_forms_uid")
                    .table(Forms::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Forms::Table)
                    .drop_column(Forms::Uid)
                    .drop_column(Forms::AccessStatus)
                    .to_owned(),
            )
            .await?;
        if is_postgres(manager) {
            manager
                .drop_type(Type::drop().name(AccessStatusType::Enum).to_owned())
                .await?;
        }
        Ok(())
    }
}

async fn backfill_uids(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    let rows = manager
        .get_connection()
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT id FROM forms WHERE uid IS NULL",
        ))
        .await?;
    for row in rows {
        let id: i64 = row.try_get("", "id")?;
        let uid = Uuid::new_v4();
        exec_sql(
            manager,
            &format!("UPDATE forms SET uid = '{uid}' WHERE id = {id}"),
        )
        .await?;
    }
    Ok(())
}
