use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CreditNotes {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Datetime,
    Reason,
    JournalEntryId,
    ReversedJournalEntryId,
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
                    .table(CreditNotes::Table)
                    .col(
                        ColumnDef::new(CreditNotes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CreditNotes::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CreditNotes::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CreditNotes::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(CreditNotes::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CreditNotes::Reason).text())
                    .col(
                        ColumnDef::new(CreditNotes::JournalEntryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CreditNotes::ReversedJournalEntryId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_credit_notes_journal_entry_id")
                            .from(CreditNotes::Table, CreditNotes::JournalEntryId)
                            .to(JournalEntries::Table, JournalEntries::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_credit_notes_reversed_journal_entry_id")
                            .from(CreditNotes::Table, CreditNotes::ReversedJournalEntryId)
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
                    .name("idx_credit_notes_deleted_at")
                    .table(CreditNotes::Table)
                    .col(CreditNotes::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_credit_notes_journal_entry_id")
                    .table(CreditNotes::Table)
                    .col(CreditNotes::JournalEntryId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_credit_notes_reversed_journal_entry_id")
                    .table(CreditNotes::Table)
                    .col(CreditNotes::ReversedJournalEntryId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CreditNotes::Table).to_owned())
            .await
    }
}
