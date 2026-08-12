use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum SourceDocs {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SourceDocType,
    SourceDocId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SourceDocs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SourceDocs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SourceDocs::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(SourceDocs::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(SourceDocs::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(SourceDocs::SourceDocType).text().not_null())
                    .col(ColumnDef::new(SourceDocs::SourceDocId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_source_docs_deleted_at")
                    .table(SourceDocs::Table)
                    .col(SourceDocs::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_source_docs_type_id")
                    .table(SourceDocs::Table)
                    .col(SourceDocs::SourceDocType)
                    .col(SourceDocs::SourceDocId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SourceDocs::Table).to_owned())
            .await
    }
}
