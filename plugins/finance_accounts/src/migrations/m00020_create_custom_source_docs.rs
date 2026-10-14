use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CustomSourceDocs {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Title,
    Content,
    CreatedById,
    JournalEntryId,
}

#[derive(DeriveIden)]
enum Users {
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
                    .table(CustomSourceDocs::Table)
                    .col(
                        ColumnDef::new(CustomSourceDocs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CustomSourceDocs::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CustomSourceDocs::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CustomSourceDocs::Title).text().not_null())
                    .col(
                        ColumnDef::new(CustomSourceDocs::Content)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocs::CreatedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocs::JournalEntryId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_custom_source_docs_created_by_id")
                            .from(CustomSourceDocs::Table, CustomSourceDocs::CreatedById)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_custom_source_docs_journal_entry_id")
                            .from(CustomSourceDocs::Table, CustomSourceDocs::JournalEntryId)
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
                    .name("uq_custom_source_docs_journal_entry_id")
                    .table(CustomSourceDocs::Table)
                    .col(CustomSourceDocs::JournalEntryId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_custom_source_docs_created_by_id")
                    .table(CustomSourceDocs::Table)
                    .col(CustomSourceDocs::CreatedById)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CustomSourceDocs::Table).to_owned())
            .await
    }
}
