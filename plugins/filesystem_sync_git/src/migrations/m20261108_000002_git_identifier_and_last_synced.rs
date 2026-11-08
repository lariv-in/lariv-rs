use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
enum FilesystemGitSyncs {
    #[sea_orm(iden = "filesystem_git_syncs")]
    Table,
    #[sea_orm(iden = "git_hash")]
    GitHash,
    #[sea_orm(iden = "git_identifier")]
    GitIdentifier,
    #[sea_orm(iden = "last_synced_commit_hash")]
    LastSyncedCommitHash,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(FilesystemGitSyncs::Table)
                    .rename_column(
                        FilesystemGitSyncs::GitHash,
                        FilesystemGitSyncs::GitIdentifier,
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(FilesystemGitSyncs::Table)
                    .add_column(
                        ColumnDef::new(FilesystemGitSyncs::LastSyncedCommitHash)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(FilesystemGitSyncs::Table)
                    .drop_column(FilesystemGitSyncs::LastSyncedCommitHash)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(FilesystemGitSyncs::Table)
                    .rename_column(
                        FilesystemGitSyncs::GitIdentifier,
                        FilesystemGitSyncs::GitHash,
                    )
                    .to_owned(),
            )
            .await
    }
}
