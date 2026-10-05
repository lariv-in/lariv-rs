use lariv_core::db::migration_sql::is_postgres;
use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Documents {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DocumentType,
    DocumentTypeId,
}

#[derive(DeriveIden)]
enum DocumentTypeType {
    #[sea_orm(iden = "document_type")]
    Enum,
    #[sea_orm(iden = "aadhar_card")]
    AadharCard,
}

#[derive(DeriveIden)]
enum AadharCards {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    VnodeId,
    AadharNumber,
    Name,
    Gender,
    DateOfBirth,
    Address,
}

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            manager
                .create_type(
                    Type::create()
                        .as_enum(DocumentTypeType::Enum)
                        .values([DocumentTypeType::AadharCard])
                        .to_owned(),
                )
                .await?;
        }

        let mut documents = Table::create();
        documents
            .table(Documents::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(Documents::Id)
                    .big_integer()
                    .not_null()
                    .auto_increment()
                    .primary_key(),
            )
            .col(ColumnDef::new(Documents::CreatedAt).timestamp_with_time_zone())
            .col(ColumnDef::new(Documents::UpdatedAt).timestamp_with_time_zone());
        if is_postgres(manager) {
            documents.col(
                ColumnDef::new(Documents::DocumentType)
                    .custom(DocumentTypeType::Enum)
                    .not_null(),
            );
        } else {
            documents.col(
                ColumnDef::new(Documents::DocumentType)
                    .string_len(64)
                    .not_null(),
            );
        }
        documents.col(
            ColumnDef::new(Documents::DocumentTypeId)
                .integer()
                .not_null(),
        );
        manager.create_table(documents).await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_documents_document_type_document_type_id")
                    .table(Documents::Table)
                    .col(Documents::DocumentType)
                    .col(Documents::DocumentTypeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AadharCards::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AadharCards::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AadharCards::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(AadharCards::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(AadharCards::VnodeId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AadharCards::AadharNumber).text().not_null())
                    .col(ColumnDef::new(AadharCards::Name).text().not_null())
                    .col(
                        ColumnDef::new(AadharCards::Gender)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(AadharCards::DateOfBirth).date().not_null())
                    .col(ColumnDef::new(AadharCards::Address).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_aadhar_cards_vnode_id")
                            .from(AadharCards::Table, AadharCards::VnodeId)
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_aadhar_cards_vnode_id")
                    .table(AadharCards::Table)
                    .col(AadharCards::VnodeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_aadhar_cards_aadhar_number")
                    .table(AadharCards::Table)
                    .col(AadharCards::AadharNumber)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AadharCards::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Documents::Table).to_owned())
            .await?;
        if is_postgres(manager) {
            manager
                .drop_type(Type::drop().name(DocumentTypeType::Enum).to_owned())
                .await?;
        }
        Ok(())
    }
}
