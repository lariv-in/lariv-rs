use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FilesystemNodes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FilesystemNodes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FilesystemNodes::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FilesystemNodes::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FilesystemNodes::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FilesystemNodes::Name).text().not_null())
                    .col(
                        ColumnDef::new(FilesystemNodes::IsDirectory)
                            .boolean()
                            .not_null(),
                    )
                    .col(ColumnDef::new(FilesystemNodes::FilePath).text())
                    .col(ColumnDef::new(FilesystemNodes::ParentId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_filesystem_nodes_parent_id")
                            .from(FilesystemNodes::Table, FilesystemNodes::ParentId)
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_filesystem_nodes_deleted_at")
                    .table(FilesystemNodes::Table)
                    .col(FilesystemNodes::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_filesystem_nodes_parent_id")
                    .table(FilesystemNodes::Table)
                    .col(FilesystemNodes::ParentId)
                    .to_owned(),
            )
            .await?;

        // Application-level uniqueness (see `node::create`) enforces the equivalent of
        // Go's partial unique index on `(COALESCE(parent_id, 0), name, is_directory)
        // WHERE deleted_at IS NULL` — SeaORM's schema builder cannot express partial
        // indexes portably across SQLite/Postgres, and a full unique index would
        // incorrectly block re-using a name after soft-delete.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_filesystem_nodes_parent_name_dir")
                    .table(FilesystemNodes::Table)
                    .col(FilesystemNodes::ParentId)
                    .col(FilesystemNodes::Name)
                    .col(FilesystemNodes::IsDirectory)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FilesystemNodes::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum FilesystemNodes {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
    IsDirectory,
    FilePath,
    ParentId,
}
