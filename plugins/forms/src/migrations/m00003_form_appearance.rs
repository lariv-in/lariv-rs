use lariv_core::db::migration_sql::is_postgres;
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
        // SQLite allows one ALTER TABLE option per statement.
        for column in [
            ColumnDef::new(Forms::Description)
                .text()
                .not_null()
                .default("")
                .take(),
            ColumnDef::new(Forms::ThemeColor)
                .integer()
                .not_null()
                .default(0x0063_66F1i64)
                .take(),
            ColumnDef::new(Forms::BackgroundVnodeId)
                .big_integer()
                .null()
                .take(),
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Forms::Table)
                        .add_column(column)
                        .to_owned(),
                )
                .await?;
        }

        // SQLite cannot add a foreign key to an existing table.
        if is_postgres(manager) {
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
        }
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
        if is_postgres(manager) {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_forms_background_vnode_id")
                        .table(Forms::Table)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_forms_background_vnode_id")
                    .table(Forms::Table)
                    .to_owned(),
            )
            .await?;
        for column in [
            Forms::Description,
            Forms::ThemeColor,
            Forms::BackgroundVnodeId,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Forms::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
