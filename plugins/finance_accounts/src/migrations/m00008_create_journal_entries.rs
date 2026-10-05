use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum JournalEntries {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Datetime,
    SourceDocId,
    JournalId,
}

#[derive(DeriveIden)]
enum SourceDocs {
    Table,
    Id,
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
            .create_table(
                Table::create()
                    .table(JournalEntries::Table)
                    .col(
                        ColumnDef::new(JournalEntries::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(JournalEntries::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(JournalEntries::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(JournalEntries::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(JournalEntries::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntries::SourceDocId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntries::JournalId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_journal_entries_source_doc_id")
                            .from(JournalEntries::Table, JournalEntries::SourceDocId)
                            .to(SourceDocs::Table, SourceDocs::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_journal_entries_journal_id")
                            .from(JournalEntries::Table, JournalEntries::JournalId)
                            .to(Journals::Table, Journals::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_journal_entries_deleted_at")
                    .table(JournalEntries::Table)
                    .col(JournalEntries::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_journal_entries_source_doc_id")
                    .table(JournalEntries::Table)
                    .col(JournalEntries::SourceDocId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_journal_entries_journal_id")
                    .table(JournalEntries::Table)
                    .col(JournalEntries::JournalId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JournalEntries::Table).to_owned())
            .await
    }
}
