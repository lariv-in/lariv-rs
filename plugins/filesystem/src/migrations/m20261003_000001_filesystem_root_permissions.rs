use sea_orm_migration::prelude::*;

use lariv_core::db::migration_sql::exec_sql;
use crate::permissions::NodePermissions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FilesystemPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FilesystemPreferences::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(FilesystemPreferences::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(FilesystemPreferences::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(FilesystemPreferences::OwnerId)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(FilesystemPreferences::RoleId)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(FilesystemPreferences::Permissions)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_filesystem_preferences_owner_id")
                    .from(FilesystemPreferences::Table, FilesystemPreferences::OwnerId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_filesystem_preferences_owner_id")
                    .table(FilesystemPreferences::Table)
                    .col(FilesystemPreferences::OwnerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_filesystem_preferences_role_id")
                    .from(FilesystemPreferences::Table, FilesystemPreferences::RoleId)
                    .to(Roles::Table, Roles::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_filesystem_preferences_role_id")
                    .table(FilesystemPreferences::Table)
                    .col(FilesystemPreferences::RoleId)
                    .to_owned(),
            )
            .await?;

        // Open root, matching access before this row existed.
        let permissions = NodePermissions::legacy().bits();
        exec_sql(
            manager,
            &format!(
                r#"
INSERT INTO filesystem_preferences (id, created_at, updated_at, owner_id, role_id, permissions)
SELECT 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, NULL, NULL, {permissions}
WHERE NOT EXISTS (SELECT 1 FROM filesystem_preferences WHERE id = 1)
"#
            ),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FilesystemPreferences::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum FilesystemPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
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
