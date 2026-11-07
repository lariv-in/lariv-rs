//! Replace role foreign keys with the role name string and drop `roles`.
use lariv_core::db::migration_sql::{exec_sql, is_postgres};

use sea_orm::{ConnectionTrait, DbBackend, Statement};
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
        if manager.get_database_backend() == DbBackend::Sqlite {
            return up_sqlite(manager).await;
        }

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
    if is_postgres(manager) {
        manager
            .alter_table(
                Table::alter()
                    .table(table)
                    .modify_column(ColumnDef::new(role).text().not_null())
                    .to_owned(),
            )
            .await?;
    }
    Ok(())
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
    if is_postgres(manager) {
        manager
            .drop_foreign_key(ForeignKey::drop().name(fk_name).table(table_fk).to_owned())
            .await?;
    };
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

/// SQLite cannot drop a foreign key or `ALTER COLUMN … SET NOT NULL`.
/// `users.role_id` is declared as a foreign key, so the table is rebuilt.
async fn up_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    repair_users_rebuild(manager).await?;
    if sqlite_column_exists(manager, "users", "role_id").await? {
        exec_sql(
            manager,
            r#"
CREATE TABLE "users__role_name" (
    "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    "created_at" timestamp_with_timezone_text,
    "updated_at" timestamp_with_timezone_text,
    "name" text NOT NULL,
    "email" text UNIQUE,
    "phone" text UNIQUE,
    "is_superuser" boolean NOT NULL DEFAULT FALSE,
    "role" text NOT NULL,
    "password" blob(1),
    "password_salt" blob(1),
    "timezone" text DEFAULT 'Asia/Kolkata',
    "last_login_time" timestamp_with_timezone_text NULL
)"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"
INSERT INTO "users__role_name" (
    id, created_at, updated_at, name, email, phone, is_superuser, role,
    password, password_salt, timezone, last_login_time
)
SELECT
    id, created_at, updated_at, name, email, phone, is_superuser,
    COALESCE((SELECT name FROM roles WHERE roles.id = users.role_id), ''),
    password, password_salt, timezone, last_login_time
FROM users"#,
        )
        .await?;
        exec_sql(manager, r#"DROP TABLE "users""#).await?;
        exec_sql(
            manager,
            r#"ALTER TABLE "users__role_name" RENAME TO "users""#,
        )
        .await?;
        exec_sql(manager, "DELETE FROM sqlite_sequence WHERE name = 'users'").await?;
        exec_sql(
            manager,
            "UPDATE sqlite_sequence SET name = 'users' WHERE name = 'users__role_name'",
        )
        .await?;
    }

    replace_role_id_sqlite(manager, "filesystem_nodes", "idx_filesystem_nodes_role_id").await?;
    replace_role_id_sqlite(
        manager,
        "filesystem_preferences",
        "idx_filesystem_preferences_role_id",
    )
    .await?;

    if sqlite_table_exists(manager, "roles").await? {
        exec_sql(manager, r#"DROP TABLE "roles""#).await?;
    }
    Ok(())
}

async fn replace_role_id_sqlite(
    manager: &SchemaManager<'_>,
    table: &str,
    index_name: &str,
) -> Result<(), DbErr> {
    if !sqlite_column_exists(manager, table, "role").await? {
        exec_sql(
            manager,
            &format!(r#"ALTER TABLE "{table}" ADD COLUMN "role" text"#),
        )
        .await?;
    }
    if sqlite_column_exists(manager, table, "role_id").await? {
        exec_sql(
            manager,
            &format!(
                r#"UPDATE "{table}" SET role = (SELECT name FROM roles WHERE roles.id = "{table}".role_id) WHERE role_id IS NOT NULL"#
            ),
        )
        .await?;
        exec_sql(manager, &format!("DROP INDEX IF EXISTS {index_name}")).await?;
        exec_sql(
            manager,
            &format!(r#"ALTER TABLE "{table}" DROP COLUMN "role_id""#),
        )
        .await?;
    }
    Ok(())
}

/// A previous run can drop `users` and then fail to rename the copy back
/// while a prepared statement still sees the old table.
async fn repair_users_rebuild(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if !sqlite_table_exists(manager, "users__role_name").await? {
        return Ok(());
    }
    if sqlite_table_exists(manager, "users").await? {
        return exec_sql(manager, r#"DROP TABLE "users__role_name""#).await;
    }
    exec_sql(
        manager,
        r#"ALTER TABLE "users__role_name" RENAME TO "users""#,
    )
    .await?;
    exec_sql(manager, "DELETE FROM sqlite_sequence WHERE name = 'users'").await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'users' WHERE name = 'users__role_name'",
    )
    .await
}

async fn sqlite_table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    sqlite_row_exists(
        manager,
        &format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
    )
    .await
}

async fn sqlite_column_exists(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    sqlite_row_exists(
        manager,
        &format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = '{column}'"),
    )
    .await
}

async fn sqlite_row_exists(manager: &SchemaManager<'_>, sql: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(manager.get_database_backend(), sql))
        .await?;
    Ok(row.is_some())
}
