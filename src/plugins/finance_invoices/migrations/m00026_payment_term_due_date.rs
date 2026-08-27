//! Store payment-term due dates as calendar dates (not timestamptz).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_unprepared(sql)
        .await
        .map(|_| ())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Absolute dues were stored as end-of-day in the app default timezone; relative
        // resolved instants keep wall-clock time from the invoice date. Convert via
        // Asia/Kolkata so the calendar date matches what the UI showed.
        execute(
            manager,
            "ALTER TABLE draft_payment_term_lines \
             ALTER COLUMN due_datetime TYPE date \
             USING (due_datetime AT TIME ZONE 'Asia/Kolkata')::date",
        )
        .await?;
        execute(
            manager,
            "ALTER TABLE draft_payment_term_lines \
             RENAME COLUMN due_datetime TO due_date",
        )
        .await?;

        execute(
            manager,
            "ALTER TABLE posted_payment_term_lines \
             ALTER COLUMN due_datetime TYPE date \
             USING (due_datetime AT TIME ZONE 'Asia/Kolkata')::date",
        )
        .await?;
        execute(
            manager,
            "ALTER TABLE posted_payment_term_lines \
             RENAME COLUMN due_datetime TO due_date",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(
            manager,
            "ALTER TABLE posted_payment_term_lines \
             RENAME COLUMN due_date TO due_datetime",
        )
        .await?;
        execute(
            manager,
            "ALTER TABLE posted_payment_term_lines \
             ALTER COLUMN due_datetime TYPE timestamp with time zone \
             USING (due_datetime::timestamp AT TIME ZONE 'Asia/Kolkata')",
        )
        .await?;

        execute(
            manager,
            "ALTER TABLE draft_payment_term_lines \
             RENAME COLUMN due_date TO due_datetime",
        )
        .await?;
        execute(
            manager,
            "ALTER TABLE draft_payment_term_lines \
             ALTER COLUMN due_datetime TYPE timestamp with time zone \
             USING (due_datetime::timestamp AT TIME ZONE 'Asia/Kolkata')",
        )
        .await?;

        Ok(())
    }
}
