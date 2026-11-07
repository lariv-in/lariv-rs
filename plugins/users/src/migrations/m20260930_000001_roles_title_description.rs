use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Roles {
    Table,
    Title,
    Description,
}

/// Backfill roles inserted by earlier migrations and startup seeds.
///
/// Known names match `users` unassigned and HR applicant / probation / employee /
/// ex-employee. Any other existing row gets a title from its name so the columns
/// can become non-null.
const BACKFILL_SQL: &str = r#"
UPDATE roles
SET
    title = CASE name
        WHEN 'unassigned' THEN 'Unassigned'
        WHEN 'applicant' THEN 'Applicant'
        WHEN 'probation' THEN 'Probation'
        WHEN 'employee' THEN 'Employee'
        WHEN 'ex-employee' THEN 'Ex-Employee'
        ELSE COALESCE(NULLIF(TRIM(name), ''), 'Role')
    END,
    description = CASE name
        WHEN 'unassigned' THEN 'Default role for users who have not been assigned a specific role.'
        WHEN 'applicant' THEN 'Person who has applied and is being considered for a position.'
        WHEN 'probation' THEN 'Employee serving a probationary period before confirmation.'
        WHEN 'employee' THEN 'Current employee of the organization.'
        WHEN 'ex-employee' THEN 'Former employee who has left the organization.'
        ELSE 'Custom user role.'
    END
WHERE title IS NULL
   OR description IS NULL
   OR TRIM(title) = ''
   OR TRIM(description) = ''
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            // A previous run can drop `roles` and then fail to rename the copy
            // back, because sqlx still has the old table in its statement cache.
            repair_interrupted_sqlite_rebuild(manager).await?;
        }

        if !roles_column_exists(manager, RolesColumn::Title).await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Roles::Table)
                        .add_column(ColumnDef::new(Roles::Title).text().null())
                        .to_owned(),
                )
                .await?;
        }
        if !roles_column_exists(manager, RolesColumn::Description).await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Roles::Table)
                        .add_column(ColumnDef::new(Roles::Description).text().null())
                        .to_owned(),
                )
                .await?;
        }

        exec_sql(manager, BACKFILL_SQL).await?;
        set_not_null(manager).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Roles::Table)
                    .drop_column(Roles::Description)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Roles::Table)
                    .drop_column(Roles::Title)
                    .to_owned(),
            )
            .await
    }
}

enum RolesColumn {
    Title,
    Description,
}

async fn set_not_null(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    match manager.get_database_backend() {
        DbBackend::Postgres => {
            exec_sql(manager, "ALTER TABLE roles ALTER COLUMN title SET NOT NULL").await?;
            exec_sql(
                manager,
                "ALTER TABLE roles ALTER COLUMN description SET NOT NULL",
            )
            .await
        }
        // SQLite cannot `ALTER COLUMN … SET NOT NULL`. Rebuilding `roles` and
        // renaming the copy back fails while sqlx still has the old table cached:
        // "there is already another table or index with this name: roles".
        // The backfill above fills existing rows; new writes set both columns.
        _ => Ok(()),
    }
}

/// Finish a rebuild that dropped `roles` and then failed on the rename.
async fn repair_interrupted_sqlite_rebuild(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let copy_exists = sqlite_row_exists(
        manager,
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'roles__title_desc'",
    )
    .await?;
    if !copy_exists {
        return Ok(());
    }
    let roles_exists = sqlite_row_exists(
        manager,
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'roles'",
    )
    .await?;
    if roles_exists {
        return exec_sql(manager, r#"DROP TABLE "roles__title_desc""#).await;
    }
    exec_sql(
        manager,
        r#"ALTER TABLE "roles__title_desc" RENAME TO "roles""#,
    )
    .await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'roles' WHERE name = 'roles__title_desc'",
    )
    .await
}

async fn roles_column_exists(
    manager: &SchemaManager<'_>,
    column: RolesColumn,
) -> Result<bool, DbErr> {
    let sql = match (manager.get_database_backend(), column) {
        (DbBackend::Postgres, RolesColumn::Title) => {
            "SELECT 1 FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = 'roles' AND column_name = 'title'"
        }
        (DbBackend::Postgres, RolesColumn::Description) => {
            "SELECT 1 FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = 'roles' AND column_name = 'description'"
        }
        (_, RolesColumn::Title) => "SELECT 1 FROM pragma_table_info('roles') WHERE name = 'title'",
        (_, RolesColumn::Description) => {
            "SELECT 1 FROM pragma_table_info('roles') WHERE name = 'description'"
        }
    };
    sqlite_row_exists(manager, sql).await
}

async fn sqlite_row_exists(manager: &SchemaManager<'_>, sql: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(manager.get_database_backend(), sql))
        .await?;
    Ok(row.is_some())
}
