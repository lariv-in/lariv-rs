//! Move invoice presentation prefs from `accounting_preferences` into `invoice_preferences`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum InvoicePreferences {
    Table,
    InvoiceNumberFormat,
    InvoicePdfTemplate,
}

#[derive(DeriveIden)]
enum AccountingPreferences {
    Table,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .add_column(ColumnDef::new(InvoicePreferences::InvoiceNumberFormat).text())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .add_column(ColumnDef::new(InvoicePreferences::InvoicePdfTemplate).text())
                    .to_owned(),
            )
            .await?;

        // Copy existing values (if any) then drop the accounts-owned table.
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            INSERT INTO invoice_preferences (id, created_at, updated_at, invoice_number_format, invoice_pdf_template)
            SELECT 1, created_at, updated_at, invoice_number_format, invoice_pdf_template
            FROM accounting_preferences
            WHERE id = 1
            ON CONFLICT (id) DO UPDATE SET
                invoice_number_format = COALESCE(
                    EXCLUDED.invoice_number_format,
                    invoice_preferences.invoice_number_format
                ),
                invoice_pdf_template = COALESCE(
                    EXCLUDED.invoice_pdf_template,
                    invoice_preferences.invoice_pdf_template
                ),
                updated_at = COALESCE(EXCLUDED.updated_at, invoice_preferences.updated_at)
            "#,
        )
        .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(AccountingPreferences::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AccountingPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at")).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("updated_at")).timestamp_with_time_zone(),
                    )
                    .col(ColumnDef::new(Alias::new("invoice_number_format")).text())
                    .col(ColumnDef::new(Alias::new("invoice_pdf_template")).text())
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            INSERT INTO accounting_preferences (id, created_at, updated_at, invoice_number_format, invoice_pdf_template)
            SELECT id, created_at, updated_at, invoice_number_format, invoice_pdf_template
            FROM invoice_preferences
            WHERE id = 1
            ON CONFLICT (id) DO UPDATE SET
                invoice_number_format = EXCLUDED.invoice_number_format,
                invoice_pdf_template = EXCLUDED.invoice_pdf_template,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .drop_column(InvoicePreferences::InvoicePdfTemplate)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(InvoicePreferences::Table)
                    .drop_column(InvoicePreferences::InvoiceNumberFormat)
                    .to_owned(),
            )
            .await
    }
}
