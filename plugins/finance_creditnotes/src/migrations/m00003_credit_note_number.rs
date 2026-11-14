//! Unique credit note numbers, backfilled from the fallback format.
use lariv_core::db::migration_sql::exec_sql;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

use crate::logic::credit_note_number::{
    DEFAULT_CREDIT_NOTE_NUMBER_FORMAT, format_credit_note_number,
};
use lariv_plugin_finance_common::fiscal_year::FiscalYear;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        exec_sql(
            manager,
            "CREATE TABLE IF NOT EXISTS credit_note_preferences ( \
                id bigint PRIMARY KEY, \
                created_at timestamptz, \
                updated_at timestamptz, \
                credit_note_number_format text \
            )",
        )
        .await?;
        exec_sql(
            manager,
            "INSERT INTO credit_note_preferences (id, created_at, updated_at) \
             VALUES (1, NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE credit_notes ADD COLUMN IF NOT EXISTS credit_note_number text",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE credit_notes ADD COLUMN IF NOT EXISTS original_note_number text",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE credit_notes ADD COLUMN IF NOT EXISTS original_note_date date",
        )
        .await?;
        backfill_credit_note_numbers(conn).await?;
        exec_sql(
            manager,
            "ALTER TABLE credit_notes ALTER COLUMN credit_note_number SET NOT NULL",
        )
        .await?;
        exec_sql(
            manager,
            "CREATE UNIQUE INDEX IF NOT EXISTS uix_credit_notes_number ON credit_notes (credit_note_number)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, "DROP INDEX IF EXISTS uix_credit_notes_number").await?;
        exec_sql(
            manager,
            "ALTER TABLE credit_notes DROP COLUMN IF EXISTS original_note_date",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE credit_notes DROP COLUMN IF EXISTS original_note_number",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE credit_notes DROP COLUMN IF EXISTS credit_note_number",
        )
        .await?;
        exec_sql(manager, "DROP TABLE IF EXISTS credit_note_preferences").await?;
        Ok(())
    }
}

async fn backfill_credit_note_numbers<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
    let rows = conn
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT id, datetime FROM credit_notes WHERE credit_note_number IS NULL ORDER BY id"
                .to_owned(),
        ))
        .await?;
    let mut fiscal_counts: HashMap<i32, i64> = HashMap::new();
    let mut seen = HashSet::new();
    for row in rows {
        let id: i64 = row.try_get("", "id")?;
        let datetime: DateTime<Utc> = row.try_get("", "datetime")?;
        let start_year = FiscalYear::for_datetime(datetime).start_year;
        let fiscal_seq = {
            let count = fiscal_counts.entry(start_year).or_insert(0);
            *count += 1;
            *count
        };
        let number =
            format_credit_note_number(DEFAULT_CREDIT_NOTE_NUMBER_FORMAT, datetime, id, fiscal_seq);
        if !seen.insert(number.clone()) {
            return Err(DbErr::Custom(format!(
                "credit note number format produced duplicate {number}"
            )));
        }
        conn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE credit_notes SET credit_note_number = $1 WHERE id = $2",
            [number.into(), id.into()],
        ))
        .await?;
    }
    Ok(())
}
