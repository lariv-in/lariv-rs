//! Replace role foreign keys with the role name string and drop `roles`.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    RoleId,
    Role,
}

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    RoleId,
    Role,
}

#[derive(DeriveIden)]
enum FilesystemPreferences {
    Table,
    RoleId,
    Role,
}

#[derive(DeriveIden)]
enum Roles {
    Table,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_role_name(manager, Users::Table, Users::Role).await?;
        exec_sql(
            manager,
            "UPDATE users SET role = COALESCE((SELECT name FROM roles WHERE roles.id = users.role_id), '')",
        )
        .await?;
        set_role_not_null(manager, Users::Table, Users::Role).await?;
        drop_role_fk(
            manager,
            Users::Table,
            Users::Table,
            Users::Table,
            "fk_users_role_id",
            "idx_users_role_id",
            Users::RoleId,
        )
        .await?;

        add_role_name(manager, FilesystemNodes::Table, FilesystemNodes::Role).await?;
        exec_sql(
            manager,
            "UPDATE filesystem_nodes SET role = (SELECT name FROM roles WHERE roles.id = filesystem_nodes.role_id) WHERE role_id IS NOT NULL",
        )
        .await?;
        drop_role_fk(
            manager,
            FilesystemNodes::Table,
            FilesystemNodes::Table,
            FilesystemNodes::Table,
            "fk_filesystem_nodes_role_id",
            "idx_filesystem_nodes_role_id",
            FilesystemNodes::RoleId,
        )
        .await?;

        add_role_name(
            manager,
            FilesystemPreferences::Table,
            FilesystemPreferences::Role,
        )
        .await?;
        exec_sql(
            manager,
            "UPDATE filesystem_preferences SET role = (SELECT name FROM roles WHERE roles.id = filesystem_preferences.role_id) WHERE role_id IS NOT NULL",
        )
        .await?;
        drop_role_fk(
            manager,
            FilesystemPreferences::Table,
            FilesystemPreferences::Table,
            FilesystemPreferences::Table,
            "fk_filesystem_preferences_role_id",
            "idx_filesystem_preferences_role_id",
            FilesystemPreferences::RoleId,
        )
        .await?;

        manager
            .drop_table(Table::drop().table(Roles::Table).to_owned())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _ = manager;
        Err(DbErr::Custom(
            "replacing role ids with role names cannot be reversed".into(),
        ))
    }
}

async fn add_role_name(
    manager: &SchemaManager<'_>,
    table: impl IntoIden,
    role: impl IntoIden,
) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .add_column(ColumnDef::new(role).text().null())
                .to_owned(),
        )
        .await
}

async fn set_role_not_null(
    manager: &SchemaManager<'_>,
    table: impl IntoIden,
    role: impl IntoIden,
) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .modify_column(ColumnDef::new(role).text().not_null())
                .to_owned(),
        )
        .await
}

async fn drop_role_fk(
    manager: &SchemaManager<'_>,
    table_fk: impl IntoIden,
    table_idx: impl IntoIden,
    table_col: impl IntoIden,
    fk_name: &str,
    index_name: &str,
    role_id: impl IntoIden,
) -> Result<(), DbErr> {
    manager
        .drop_foreign_key(ForeignKey::drop().name(fk_name).table(table_fk).to_owned())
        .await?;
    manager
        .drop_index(Index::drop().name(index_name).table(table_idx).to_owned())
        .await?;
    manager
        .alter_table(
            Table::alter()
                .table(table_col)
                .drop_column(role_id)
                .to_owned(),
        )
        .await
}
