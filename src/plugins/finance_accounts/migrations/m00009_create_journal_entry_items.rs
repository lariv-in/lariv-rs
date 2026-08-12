use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum JournalEntryItems {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Datetime,
    AccountId,
    Amount,
    JournalEntryId,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum JournalEntries {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JournalEntryItems::Table)
                    .col(
                        ColumnDef::new(JournalEntryItems::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryItems::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryItems::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryItems::DeletedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryItems::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(JournalEntryItems::AccountId).big_integer().not_null())
                    .col(
                        ColumnDef::new(JournalEntryItems::Amount)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryItems::JournalEntryId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_journal_entry_items_account_id")
                            .from(JournalEntryItems::Table, JournalEntryItems::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_journal_entry_items_journal_entry_id")
                            .from(JournalEntryItems::Table, JournalEntryItems::JournalEntryId)
                            .to(JournalEntries::Table, JournalEntries::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_journal_entry_items_deleted_at")
                    .table(JournalEntryItems::Table)
                    .col(JournalEntryItems::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_journal_entry_items_account_id")
                    .table(JournalEntryItems::Table)
                    .col(JournalEntryItems::AccountId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_journal_entry_items_journal_entry_id")
                    .table(JournalEntryItems::Table)
                    .col(JournalEntryItems::JournalEntryId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JournalEntryItems::Table).to_owned())
            .await
    }
}
