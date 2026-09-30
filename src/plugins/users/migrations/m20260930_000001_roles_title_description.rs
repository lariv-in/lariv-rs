use crate::db::migration_sql::exec_sql;
use sea_orm::DbBackend;
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
        manager
            .alter_table(
                Table::alter()
                    .table(Roles::Table)
                    .add_column(ColumnDef::new(Roles::Title).text().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Roles::Table)
                    .add_column(ColumnDef::new(Roles::Description).text().null())
                    .to_owned(),
            )
            .await?;

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
        _ => rebuild_sqlite_roles_not_null(manager).await,
    }
}

/// SQLite cannot `ALTER COLUMN … SET NOT NULL`. Copy `roles` into a new table
/// whose title and description are required, then swap the names.
async fn rebuild_sqlite_roles_not_null(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
    exec_sql(
        manager,
        r#"CREATE TABLE "roles__title_desc" (
            "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
            "created_at" timestamp_with_timezone_text,
            "updated_at" timestamp_with_timezone_text,
            "name" text UNIQUE,
            "title" text NOT NULL,
            "description" text NOT NULL
        )"#,
    )
    .await?;
    exec_sql(
        manager,
        r#"INSERT INTO "roles__title_desc"
            ("id", "created_at", "updated_at", "name", "title", "description")
           SELECT "id", "created_at", "updated_at", "name", "title", "description"
           FROM "roles""#,
    )
    .await?;
    exec_sql(manager, r#"DROP TABLE "roles""#).await?;
    exec_sql(
        manager,
        r#"ALTER TABLE "roles__title_desc" RENAME TO "roles""#,
    )
    .await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'roles' WHERE name = 'roles__title_desc'",
    )
    .await?;
    exec_sql(manager, "PRAGMA foreign_keys = ON").await
}
