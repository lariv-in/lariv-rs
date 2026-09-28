use sea_orm_migration::prelude::*;

use crate::db::migration_sql::{exec_sql, is_postgres};
use crate::plugins::filesystem::permissions::NodePermissions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(FilesystemNodes::Table)
                    .add_column(
                        ColumnDef::new(FilesystemNodes::OwnerId)
                            .big_integer()
                            .null(),
                    )
                    .add_column(ColumnDef::new(FilesystemNodes::RoleId).big_integer().null())
                    .add_column(
                        ColumnDef::new(FilesystemNodes::Permissions)
                            .integer()
                            .not_null()
                            .default(NodePermissions::legacy().bits()),
                    )
                    .to_owned(),
            )
            .await?;

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
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_filesystem_nodes_owner_id")
                    .table(FilesystemNodes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_filesystem_nodes_owner_id")
                    .table(FilesystemNodes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_filesystem_nodes_role_id")
                    .table(FilesystemNodes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_filesystem_nodes_role_id")
                    .table(FilesystemNodes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(FilesystemNodes::Table)
                    .drop_column(FilesystemNodes::OwnerId)
                    .drop_column(FilesystemNodes::RoleId)
                    .drop_column(FilesystemNodes::Permissions)
                    .to_owned(),
            )
            .await?;
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
