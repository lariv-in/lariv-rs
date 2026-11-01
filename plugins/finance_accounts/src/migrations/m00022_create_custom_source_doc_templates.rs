use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CustomSourceDocTemplates {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Remarks,
    Variables,
}

#[derive(DeriveIden)]
enum CustomSourceDocJournalEntryItemTemplates {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    CustomSourceDocTemplateId,
    AccountId,
    Formula,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CustomSourceDocTemplates::Table)
                    .col(
                        ColumnDef::new(CustomSourceDocTemplates::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocTemplates::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocTemplates::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocTemplates::Name)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CustomSourceDocTemplates::Remarks).text())
                    .col(
                        ColumnDef::new(CustomSourceDocTemplates::Variables)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CustomSourceDocJournalEntryItemTemplates::Table)
                    .col(
                        ColumnDef::new(CustomSourceDocJournalEntryItemTemplates::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocJournalEntryItemTemplates::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocJournalEntryItemTemplates::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(
                            CustomSourceDocJournalEntryItemTemplates::CustomSourceDocTemplateId,
                        )
                        .big_integer()
                        .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocJournalEntryItemTemplates::AccountId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomSourceDocJournalEntryItemTemplates::Formula)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_csd_journal_item_templates_template_id")
                            .from(
                                CustomSourceDocJournalEntryItemTemplates::Table,
                                CustomSourceDocJournalEntryItemTemplates::CustomSourceDocTemplateId,
                            )
                            .to(
                                CustomSourceDocTemplates::Table,
                                CustomSourceDocTemplates::Id,
                            )
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_csd_journal_item_templates_account_id")
                            .from(
                                CustomSourceDocJournalEntryItemTemplates::Table,
                                CustomSourceDocJournalEntryItemTemplates::AccountId,
                            )
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_csd_journal_item_templates_template_id")
                    .table(CustomSourceDocJournalEntryItemTemplates::Table)
                    .col(CustomSourceDocJournalEntryItemTemplates::CustomSourceDocTemplateId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_csd_journal_item_templates_account_id")
                    .table(CustomSourceDocJournalEntryItemTemplates::Table)
                    .col(CustomSourceDocJournalEntryItemTemplates::AccountId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(CustomSourceDocJournalEntryItemTemplates::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(CustomSourceDocTemplates::Table)
                    .to_owned(),
            )
            .await
    }
}
