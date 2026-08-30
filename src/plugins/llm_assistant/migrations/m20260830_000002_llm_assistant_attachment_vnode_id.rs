use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum InlineData {
    #[sea_orm(iden = "llm_assistant_session_message_inline_data")]
    Table,
    VnodeId,
}

#[derive(DeriveIden)]
enum FileData {
    #[sea_orm(iden = "llm_assistant_session_message_file_data")]
    Table,
    VnodeId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(InlineData::Table)
                    .add_column(ColumnDef::new(InlineData::VnodeId).big_integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(FileData::Table)
                    .add_column(ColumnDef::new(FileData::VnodeId).big_integer().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(FileData::Table)
                    .drop_column(FileData::VnodeId)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(InlineData::Table)
                    .drop_column(InlineData::VnodeId)
                    .to_owned(),
            )
            .await
    }
}
