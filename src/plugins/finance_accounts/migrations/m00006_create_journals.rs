use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Journals {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
    IsActive,
    CurrencyId,
    JournalType,
}

#[derive(DeriveIden)]
enum JournalType {
    #[sea_orm(iden = "journal_type")]
    Enum,
    #[sea_orm(iden = "Credit")]
    Credit,
    #[sea_orm(iden = "Debit")]
    Debit,
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
            .create_type(
                Type::create()
                    .as_enum(JournalType::Enum)
                    .values([JournalType::Credit, JournalType::Debit])
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Journals::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Journals::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Journals::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Journals::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Journals::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Journals::Name).text().not_null())
                    .col(
                        ColumnDef::new(Journals::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(Journals::CurrencyId).big_integer().not_null())
                    .col(
                        ColumnDef::new(Journals::JournalType)
                            .custom(JournalType::Enum)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_journals_currency_id")
                            .from(Journals::Table, Journals::CurrencyId)
                            .to(Currencies::Table, Currencies::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_journals_deleted_at")
                    .table(Journals::Table)
                    .col(Journals::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_journals_currency_id")
                    .table(Journals::Table)
                    .col(Journals::CurrencyId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Journals::Table).to_owned())
            .await?;
        manager
            .drop_type(Type::drop().name(JournalType::Enum).to_owned())
            .await
    }
}
