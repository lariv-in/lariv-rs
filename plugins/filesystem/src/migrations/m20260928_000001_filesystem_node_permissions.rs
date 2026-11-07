use sea_orm_migration::prelude::*;

use crate::permissions::NodePermissions;
use lariv_core::db::migration_sql::{exec_sql, is_postgres};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite allows one ALTER TABLE option per statement.
        for column in [
            ColumnDef::new(FilesystemNodes::OwnerId)
                .big_integer()
                .null()
                .take(),
            ColumnDef::new(FilesystemNodes::RoleId)
                .big_integer()
                .null()
                .take(),
            ColumnDef::new(FilesystemNodes::Permissions)
                .integer()
                .not_null()
                .default(NodePermissions::legacy().bits())
                .take(),
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(FilesystemNodes::Table)
                        .add_column(column)
                        .to_owned(),
                )
                .await?;
        }

        // SQLite cannot add a foreign key to an existing table.
        if is_postgres(manager) {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_filesystem_nodes_owner_id")
                        .from(FilesystemNodes::Table, FilesystemNodes::OwnerId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::SetNull)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_filesystem_nodes_owner_id")
                    .table(FilesystemNodes::Table)
                    .col(FilesystemNodes::OwnerId)
                    .to_owned(),
            )
            .await?;

        if is_postgres(manager) {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_filesystem_nodes_role_id")
                        .from(FilesystemNodes::Table, FilesystemNodes::RoleId)
                        .to(Roles::Table, Roles::Id)
                        .on_delete(ForeignKeyAction::SetNull)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_filesystem_nodes_role_id")
                    .table(FilesystemNodes::Table)
                    .col(FilesystemNodes::RoleId)
                    .to_owned(),
            )
            .await?;

        if is_postgres(manager) {
            exec_sql(
                manager,
                "ALTER TABLE filesystem_nodes ALTER COLUMN permissions DROP DEFAULT",
            )
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_filesystem_nodes_owner_id")
                        .table(FilesystemNodes::Table)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_filesystem_nodes_owner_id")
                    .table(FilesystemNodes::Table)
                    .to_owned(),
            )
            .await?;
        if is_postgres(manager) {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_filesystem_nodes_role_id")
                        .table(FilesystemNodes::Table)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_filesystem_nodes_role_id")
                    .table(FilesystemNodes::Table)
                    .to_owned(),
            )
            .await?;
        for column in [
            FilesystemNodes::OwnerId,
            FilesystemNodes::RoleId,
            FilesystemNodes::Permissions,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(FilesystemNodes::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(Iden)]
enum FilesystemNodes {
    Table,
    OwnerId,
    RoleId,
    Permissions,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[derive(Iden)]
enum Roles {
    Table,
    Id,
}
