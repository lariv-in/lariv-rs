//! Recreate accounts-owned `accounting_preferences` after m00015 dropped it.
//!
//! Must live in the invoices migrator so it runs after the drop (accounts
//! migrations run earlier in install order).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum AccountingPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DefaultCurrencyId,
}

#[derive(DeriveIden)]
enum Currencies {
    Table,
    Id,
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
                    .col(ColumnDef::new(AccountingPreferences::DefaultCurrencyId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounting_preferences_default_currency_id")
                            .from(
                                AccountingPreferences::Table,
                                AccountingPreferences::DefaultCurrencyId,
                            )
                            .to(Currencies::Table, Currencies::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(AccountingPreferences::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
