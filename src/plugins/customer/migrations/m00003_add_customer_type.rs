use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Customers {
    Table,
    CustomerType,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .add_column(
                        ColumnDef::new(Customers::CustomerType)
                            .string_len(32)
                            .not_null()
                            .default("business"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .drop_column(Customers::CustomerType)
                    .to_owned(),
            )
            .await
    }
}
