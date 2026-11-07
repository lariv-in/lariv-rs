//! Unique payment numbers, backfilled from Payment number format.
use lariv_core::db::migration_sql::exec_sql;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

use crate::logic::payment_number::{DEFAULT_PAYMENT_NUMBER_FORMAT, format_payment_number};
use lariv_plugin_finance_common::fiscal_year::FiscalYear;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        exec_sql(
            manager,
            "ALTER TABLE payment_preferences ADD COLUMN IF NOT EXISTS payment_number_format text",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE payments ADD COLUMN IF NOT EXISTS number text",
        )
        .await?;

        let format = load_payment_number_format(conn).await?;
        backfill_payment_numbers(conn, &format).await?;

        exec_sql(
            manager,
            "ALTER TABLE payments ALTER COLUMN number SET NOT NULL",
        )
        .await?;
        exec_sql(
            manager,
            "CREATE UNIQUE INDEX IF NOT EXISTS uix_payments_number ON payments (number)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, "DROP INDEX IF EXISTS uix_payments_number").await?;
        exec_sql(manager, "ALTER TABLE payments DROP COLUMN IF EXISTS number").await?;
        exec_sql(
            manager,
            "ALTER TABLE payment_preferences DROP COLUMN IF EXISTS payment_number_format",
        )
        .await?;
        Ok(())
    }
}

async fn load_payment_number_format<C: ConnectionTrait>(conn: &C) -> Result<String, DbErr> {
    let row = conn
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT payment_number_format FROM payment_preferences WHERE id = 1".to_owned(),
        ))
        .await?;
    let stored = match row {
        Some(row) => row.try_get::<Option<String>>("", "payment_number_format")?,
        None => None,
    };
    let stored = stored.unwrap_or_default();
    if stored.trim().is_empty() {
        Ok(DEFAULT_PAYMENT_NUMBER_FORMAT.to_owned())
    } else {
        Ok(stored)
    }
}

async fn backfill_payment_numbers<C: ConnectionTrait>(conn: &C, format: &str) -> Result<(), DbErr> {
    let rows = conn
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT id, datetime FROM payments WHERE number IS NULL ORDER BY id".to_owned(),
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
        let number = format_payment_number(format, datetime, id, fiscal_seq);
        if !seen.insert(number.clone()) {
            return Err(DbErr::Custom(format!(
                "payment number format produced duplicate {number}"
            )));
        }
        conn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE payments SET number = $1 WHERE id = $2",
            [number.into(), id.into()],
        ))
        .await?;
    }
    Ok(())
}
