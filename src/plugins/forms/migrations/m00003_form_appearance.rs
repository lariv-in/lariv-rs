use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Forms {
    Table,
    Description,
    ThemeColor,
    BackgroundVnodeId,
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
                    .table(Forms::Table)
                    .add_column(
                        ColumnDef::new(Forms::Description)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(Forms::ThemeColor)
                            .integer()
                            .not_null()
                            .default(0x0063_66F1i64),
                    )
                    .add_column(
                        ColumnDef::new(Forms::BackgroundVnodeId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_forms_background_vnode_id")
                    .from(Forms::Table, Forms::BackgroundVnodeId)
                    .to(FilesystemNodes::Table, FilesystemNodes::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_forms_background_vnode_id")
                    .table(Forms::Table)
                    .col(Forms::BackgroundVnodeId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_forms_background_vnode_id")
                    .table(Forms::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_forms_background_vnode_id")
                    .table(Forms::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Forms::Table)
                    .drop_column(Forms::Description)
                    .drop_column(Forms::ThemeColor)
                    .drop_column(Forms::BackgroundVnodeId)
                    .to_owned(),
            )
            .await
    }
}
