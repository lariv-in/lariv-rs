use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const COLUMNS: [&str; 6] = [
    "address_line_1",
    "address_line_2",
    "city",
    "pincode",
    "state",
    "website",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in COLUMNS {
            let sql = match manager.get_database_backend() {
                DbBackend::Postgres => {
                    format!("ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS {column} TEXT")
                }
                _ if column_exists(manager, column).await? => continue,
                _ => format!("ALTER TABLE crm_companies ADD COLUMN {column} TEXT"),
            };
            exec_sql(manager, &sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in COLUMNS {
            let sql = match manager.get_database_backend() {
                DbBackend::Postgres => {
                    format!("ALTER TABLE crm_companies DROP COLUMN IF EXISTS {column}")
                }
                _ if !column_exists(manager, column).await? => continue,
                _ => format!("ALTER TABLE crm_companies DROP COLUMN {column}"),
            };
            exec_sql(manager, &sql).await?;
        }
        Ok(())
    }
}

async fn column_exists(manager: &SchemaManager<'_>, column: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            manager.get_database_backend(),
            format!("SELECT 1 FROM pragma_table_info('crm_companies') WHERE name = '{column}'"),
        ))
        .await?;
    Ok(row.is_some())
}
