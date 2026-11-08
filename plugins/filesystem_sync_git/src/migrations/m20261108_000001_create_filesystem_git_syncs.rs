use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
enum FilesystemGitSyncs {
    #[sea_orm(iden = "filesystem_git_syncs")]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    GitUrl,
    GitHash,
    Subpath,
    DirectoryVnodeId,
}

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    Id,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FilesystemGitSyncs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FilesystemGitSyncs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FilesystemGitSyncs::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FilesystemGitSyncs::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FilesystemGitSyncs::GitUrl).text().not_null())
                    .col(
                        ColumnDef::new(FilesystemGitSyncs::GitHash)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FilesystemGitSyncs::Subpath)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FilesystemGitSyncs::DirectoryVnodeId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_filesystem_git_syncs_directory_vnode_id")
                            .from(
                                FilesystemGitSyncs::Table,
                                FilesystemGitSyncs::DirectoryVnodeId,
                            )
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_filesystem_git_syncs_directory_vnode_id")
                    .table(FilesystemGitSyncs::Table)
                    .col(FilesystemGitSyncs::DirectoryVnodeId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FilesystemGitSyncs::Table).to_owned())
            .await
    }
}
