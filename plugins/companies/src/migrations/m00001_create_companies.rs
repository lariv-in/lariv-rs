//! Companies live in `companies`.
//!
//! Databases that already have CRM's `crm_companies` table are renamed in place.
//! A deployment that has neither table gets the current columns here.
//!
//! Contacts used to register a migration with this same name that only created
//! `crm_companies`. SeaORM will not run this file again on those databases.
//! `m00001_ensure_companies` repeats this step under a new name.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_POSTGRES: &str = r#"
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'crm_companies'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'companies'
  ) THEN
    ALTER TABLE crm_companies RENAME TO companies;
  END IF;
END $$;
"#;

const DOWN_POSTGRES: &str = r#"
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'companies'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'crm_companies'
  ) THEN
    ALTER TABLE companies RENAME TO crm_companies;
  END IF;
END $$;
"#;

#[derive(DeriveIden)]
enum Companies {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    City,
    Pincode,
    State,
    Website,
    Gstin,
    Cin,
    Pan,
    Phone,
    Email,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_companies(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_if_needed(manager, "companies", "crm_companies", DOWN_POSTGRES).await
    }
}

pub(crate) async fn ensure_companies(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    rename_if_needed(manager, "crm_companies", "companies", UP_POSTGRES).await?;
    manager
        .create_table(
            Table::create()
                .table(Companies::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Companies::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(Companies::CreatedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(Companies::UpdatedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(Companies::Name).text().not_null())
                .col(ColumnDef::new(Alias::new("address_line_1")).text())
                .col(ColumnDef::new(Alias::new("address_line_2")).text())
                .col(ColumnDef::new(Companies::City).text())
                .col(ColumnDef::new(Companies::Pincode).text())
                .col(ColumnDef::new(Companies::State).text())
                .col(ColumnDef::new(Companies::Website).text())
                .col(ColumnDef::new(Companies::Gstin).text())
                .col(ColumnDef::new(Companies::Cin).text())
                .col(ColumnDef::new(Companies::Pan).text())
                .col(ColumnDef::new(Companies::Phone).text())
                .col(ColumnDef::new(Companies::Email).text())
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn rename_if_needed(
    manager: &SchemaManager<'_>,
    from: &str,
    to: &str,
    postgres_sql: &str,
) -> Result<(), DbErr> {
    match manager.get_connection().get_database_backend() {
        sea_orm::DatabaseBackend::Postgres => exec_sql(manager, postgres_sql).await,
        sea_orm::DatabaseBackend::Sqlite => {
            let has_from = sqlite_table_exists(manager, from).await?;
            let has_to = sqlite_table_exists(manager, to).await?;
            if has_from && !has_to {
                exec_sql(manager, &format!("ALTER TABLE {from} RENAME TO {to}")).await?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn sqlite_table_exists(manager: &SchemaManager<'_>, name: &str) -> Result<bool, DbErr> {
    let sql = format!("SELECT 1 AS ok FROM sqlite_master WHERE type = 'table' AND name = '{name}'");
    Ok(manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
        ))
        .await?
        .is_some())
}
