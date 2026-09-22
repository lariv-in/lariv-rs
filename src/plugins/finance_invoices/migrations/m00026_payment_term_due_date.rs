//! Store payment-term due dates as calendar dates (not timestamptz).
use crate::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Absolute dues were stored as end-of-day in the app default timezone; relative
        // resolved instants keep wall-clock time from the invoice date. Convert via
        // Asia/Kolkata so the calendar date matches what the UI showed.
        exec_sql(
            manager,
            "ALTER TABLE draft_payment_term_lines \
             ALTER COLUMN due_datetime TYPE date \
             USING (due_datetime AT TIME ZONE 'Asia/Kolkata')::date",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE draft_payment_term_lines \
             RENAME COLUMN due_datetime TO due_date",
        )
        .await?;

        exec_sql(
            manager,
            "ALTER TABLE posted_payment_term_lines \
             ALTER COLUMN due_datetime TYPE date \
             USING (due_datetime AT TIME ZONE 'Asia/Kolkata')::date",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE posted_payment_term_lines \
             RENAME COLUMN due_datetime TO due_date",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "ALTER TABLE posted_payment_term_lines \
             RENAME COLUMN due_date TO due_datetime",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE posted_payment_term_lines \
             ALTER COLUMN due_datetime TYPE timestamp with time zone \
             USING (due_datetime::timestamp AT TIME ZONE 'Asia/Kolkata')",
        )
        .await?;

        exec_sql(
            manager,
            "ALTER TABLE draft_payment_term_lines \
             RENAME COLUMN due_date TO due_datetime",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE draft_payment_term_lines \
             ALTER COLUMN due_datetime TYPE timestamp with time zone \
             USING (due_datetime::timestamp AT TIME ZONE 'Asia/Kolkata')",
        )
        .await?;

        Ok(())
    }
}
