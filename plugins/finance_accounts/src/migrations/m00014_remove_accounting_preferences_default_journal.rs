use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum AccountingPreferences {
    Table,
    DefaultJournalId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPreferences::Table)
                    .drop_column(AccountingPreferences::DefaultJournalId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPreferences::Table)
                    .add_column(
                        ColumnDef::new(AccountingPreferences::DefaultJournalId).big_integer(),
                    )
                    .to_owned(),
            )
            .await
    }
}
