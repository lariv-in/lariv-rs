use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_connection().get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

async fn execute_if_exists(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    let wrapped = format!(
        r#"
DO $do$
BEGIN
  BEGIN
    {sql};
  EXCEPTION
    WHEN undefined_table THEN NULL;
  END;
END
$do$;
"#
    );
    execute(manager, &wrapped).await
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // cancelled_invoices may still FK to soft-deleted credit notes (invoices plugin
        // may already be installed on upgrade; absent on fresh installs before invoices).
        execute_if_exists(
            manager,
            r#"DELETE FROM cancelled_invoice_lines
               WHERE cancelled_invoice_id IN (
                 SELECT id FROM cancelled_invoices
                 WHERE credit_note_id IN (
                   SELECT id FROM credit_notes WHERE deleted_at IS NOT NULL
                 )
               )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM cancelled_invoices
               WHERE credit_note_id IN (
                 SELECT id FROM credit_notes WHERE deleted_at IS NOT NULL
               )"#,
        )
        .await?;

        execute(
            manager,
            "DELETE FROM credit_notes WHERE deleted_at IS NOT NULL",
        )
        .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_credit_notes_deleted_at")
                    .table(Alias::new("credit_notes"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("credit_notes"))
                    .drop_column(Alias::new("deleted_at"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("credit_notes"))
                    .add_column(ColumnDef::new(Alias::new("deleted_at")).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_credit_notes_deleted_at")
                    .table(Alias::new("credit_notes"))
                    .col(Alias::new("deleted_at"))
                    .to_owned(),
            )
            .await
    }
}
