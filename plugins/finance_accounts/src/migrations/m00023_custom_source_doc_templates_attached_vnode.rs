use lariv_core::db::migration_sql::is_postgres;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CustomSourceDocTemplates {
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
                    .table(CustomSourceDocTemplates::Table)
                    .add_column(
                        ColumnDef::new(CustomSourceDocTemplates::AttachedVnodeId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        if is_postgres(manager) {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_custom_source_doc_templates_attached_vnode_id")
                        .from(
                            CustomSourceDocTemplates::Table,
                            CustomSourceDocTemplates::AttachedVnodeId,
                        )
                        .to(FilesystemNodes::Table, FilesystemNodes::Id)
                        .on_update(ForeignKeyAction::Cascade)
                        .on_delete(ForeignKeyAction::SetNull)
                        .to_owned(),
                )
                .await?;
        }

        manager
            .create_index(
                Index::create()
                    .name("idx_custom_source_doc_templates_attached_vnode_id")
                    .table(CustomSourceDocTemplates::Table)
                    .col(CustomSourceDocTemplates::AttachedVnodeId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_custom_source_doc_templates_attached_vnode_id")
                        .table(CustomSourceDocTemplates::Table)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_custom_source_doc_templates_attached_vnode_id")
                    .table(CustomSourceDocTemplates::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CustomSourceDocTemplates::Table)
                    .drop_column(CustomSourceDocTemplates::AttachedVnodeId)
                    .to_owned(),
            )
            .await
    }
}
