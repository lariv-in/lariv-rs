use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CustomSourceDocs {
    Table,
    AttachedVnodeId,
}

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CustomSourceDocs::Table)
                    .add_column(
                        ColumnDef::new(CustomSourceDocs::AttachedVnodeId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_custom_source_docs_attached_vnode_id")
                    .from(CustomSourceDocs::Table, CustomSourceDocs::AttachedVnodeId)
                    .to(FilesystemNodes::Table, FilesystemNodes::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_custom_source_docs_attached_vnode_id")
                    .table(CustomSourceDocs::Table)
                    .col(CustomSourceDocs::AttachedVnodeId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_custom_source_docs_attached_vnode_id")
                    .table(CustomSourceDocs::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_custom_source_docs_attached_vnode_id")
                    .table(CustomSourceDocs::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CustomSourceDocs::Table)
                    .drop_column(CustomSourceDocs::AttachedVnodeId)
                    .to_owned(),
            )
            .await
    }
}
