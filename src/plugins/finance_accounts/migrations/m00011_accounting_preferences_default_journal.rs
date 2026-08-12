use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum AccountingPreferences {
    Table,
    DefaultJournalId,
}

#[derive(DeriveIden)]
enum Journals {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPreferences::Table)
                    .add_column(ColumnDef::new(AccountingPreferences::DefaultJournalId).big_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_accounting_preferences_default_journal_id")
                    .from(AccountingPreferences::Table, AccountingPreferences::DefaultJournalId)
                    .to(Journals::Table, Journals::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPreferences::Table)
                    .drop_column(AccountingPreferences::DefaultJournalId)
                    .to_owned(),
            )
            .await
    }
}
