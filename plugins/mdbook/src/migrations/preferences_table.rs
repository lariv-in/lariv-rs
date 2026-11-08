use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub enum MdBookPreferences {
    /// `MdBook` snake-cases to `md_book`, which does not match the entity table.
    #[sea_orm(iden = "mdbook_preferences")]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    SourceVnodeId,
    OutputVnodeId,
    UrlPrefix,
    BuildOnServe,
}

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    Id,
}

pub async fn create_mdbook_preferences(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MdBookPreferences::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(MdBookPreferences::Id)
                        .big_integer()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(MdBookPreferences::CreatedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(MdBookPreferences::UpdatedAt).timestamp_with_time_zone())
                .col(
                    ColumnDef::new(MdBookPreferences::SourceVnodeId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MdBookPreferences::OutputVnodeId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MdBookPreferences::UrlPrefix)
                        .text()
                        .not_null()
                        .default("/book"),
                )
                .col(
                    ColumnDef::new(MdBookPreferences::BuildOnServe)
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_mdbook_preferences_source_vnode_id")
                        .from(MdBookPreferences::Table, MdBookPreferences::SourceVnodeId)
                        .to(FilesystemNodes::Table, FilesystemNodes::Id)
                        .on_update(ForeignKeyAction::Cascade)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_mdbook_preferences_output_vnode_id")
                        .from(MdBookPreferences::Table, MdBookPreferences::OutputVnodeId)
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
                .if_not_exists()
                .name("idx_mdbook_preferences_source_vnode_id")
                .table(MdBookPreferences::Table)
                .col(MdBookPreferences::SourceVnodeId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_mdbook_preferences_output_vnode_id")
                .table(MdBookPreferences::Table)
                .col(MdBookPreferences::OutputVnodeId)
                .to_owned(),
        )
        .await
}
