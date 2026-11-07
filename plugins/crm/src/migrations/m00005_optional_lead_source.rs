use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeads {
    Table,
    Source,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            return make_source_nullable_sqlite(manager).await;
        }
        if is_postgres(manager) {
            manager
                .alter_table(
                    Table::alter()
                        .table(CrmLeads::Table)
                        .modify_column(ColumnDef::new(CrmLeads::Source).string_len(32).null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let update = Query::update()
            .table(CrmLeads::Table)
            .value(CrmLeads::Source, "web_form")
            .and_where(Expr::col(CrmLeads::Source).is_null())
            .to_owned();
        manager.get_connection().execute(&update).await?;

        if manager.get_database_backend() == DbBackend::Sqlite {
            // SQLite cannot put NOT NULL back with ALTER COLUMN. New rows still
            // receive the application default; existing rows were filled above.
            return Ok(());
        }

        if is_postgres(manager) {
            manager
                .alter_table(
                    Table::alter()
                        .table(CrmLeads::Table)
                        .modify_column(
                            ColumnDef::new(CrmLeads::Source)
                                .string_len(32)
                                .not_null()
                                .default("web_form"),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

/// SQLite cannot `ALTER COLUMN`. Rebuild `crm_leads` with a nullable `source`.
async fn make_source_nullable_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    repair_interrupted_rebuild(manager).await?;
    if !source_is_not_null(manager).await? {
        return Ok(());
    }

    // Child tables reference `crm_leads`. Disable enforcement across the drop.
    exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
    exec_sql(
        manager,
        r#"
CREATE TABLE "crm_leads__source" (
    "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    "created_at" timestamp_with_timezone_text,
    "updated_at" timestamp_with_timezone_text,
    "company_name" text,
    "first_name" text,
    "last_name" text,
    "email" text,
    "phone" text,
    "source" varchar(32),
    "notes" text
)"#,
    )
    .await?;
    exec_sql(
        manager,
        r#"
INSERT INTO "crm_leads__source" (
    id, created_at, updated_at, company_name, first_name, last_name, email, phone, source, notes
)
SELECT
    id, created_at, updated_at, company_name, first_name, last_name, email, phone, source, notes
FROM crm_leads"#,
    )
    .await?;
    exec_sql(manager, r#"DROP TABLE "crm_leads""#).await?;
    exec_sql(
        manager,
        r#"ALTER TABLE "crm_leads__source" RENAME TO "crm_leads""#,
    )
    .await?;
    exec_sql(
        manager,
        "DELETE FROM sqlite_sequence WHERE name = 'crm_leads'",
    )
    .await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'crm_leads' WHERE name = 'crm_leads__source'",
    )
    .await?;
    exec_sql(manager, "PRAGMA foreign_keys = ON").await
}

/// A previous run can drop `crm_leads` and then fail to rename the copy back
/// while a prepared statement still sees the old table.
async fn repair_interrupted_rebuild(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if !table_exists(manager, "crm_leads__source").await? {
        return Ok(());
    }
    if table_exists(manager, "crm_leads").await? {
        return exec_sql(manager, r#"DROP TABLE "crm_leads__source""#).await;
    }
    exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
    exec_sql(
        manager,
        r#"ALTER TABLE "crm_leads__source" RENAME TO "crm_leads""#,
    )
    .await?;
    exec_sql(
        manager,
        "DELETE FROM sqlite_sequence WHERE name = 'crm_leads'",
    )
    .await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'crm_leads' WHERE name = 'crm_leads__source'",
    )
    .await?;
    exec_sql(manager, "PRAGMA foreign_keys = ON").await
}

async fn source_is_not_null(manager: &SchemaManager<'_>) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT 1 FROM pragma_table_info('crm_leads') WHERE name = 'source' AND \"notnull\" = 1",
        ))
        .await?;
    Ok(row.is_some())
}

async fn table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
        ))
        .await?;
    Ok(row.is_some())
}
