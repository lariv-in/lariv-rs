use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeads {
    Table,
    Source,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .modify_column(ColumnDef::new(CrmLeads::Source).string_len(32).null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let update = Query::update()
            .table(CrmLeads::Table)
            .value(CrmLeads::Source, "web_form")
            .and_where(Expr::col(CrmLeads::Source).is_null())
            .to_owned();
        manager
            .get_connection()
            .execute(backend.build(&update))
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .modify_column(
                        ColumnDef::new(CrmLeads::Source)
                            .string_len(32)
                            .not_null()
                            .default("web_form"),
                    )
                    .to_owned(),
            )
            .await
    }
}
