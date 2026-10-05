use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum AccountingPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    InvoiceNumberFormat,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AccountingPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AccountingPreferences::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AccountingPreferences::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(AccountingPreferences::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(AccountingPreferences::DeletedAt).timestamp_with_time_zone(),
                    )
                    .col(ColumnDef::new(AccountingPreferences::InvoiceNumberFormat).text())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_accounting_preferences_deleted_at")
                    .table(AccountingPreferences::Table)
                    .col(AccountingPreferences::DeletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AccountingPreferences::Table).to_owned())
            .await
    }
}
