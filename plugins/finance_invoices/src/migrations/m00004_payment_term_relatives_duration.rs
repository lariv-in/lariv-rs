use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PaymentTermRelatives {
    Table,
    Duration,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(PaymentTermRelatives::Table)
                    .add_column(
                        ColumnDef::new(PaymentTermRelatives::Duration)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PaymentTermRelatives::Table)
                    .modify_column(
                        ColumnDef::new(PaymentTermRelatives::Duration)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(PaymentTermRelatives::Table)
                    .drop_column(PaymentTermRelatives::Duration)
                    .to_owned(),
            )
            .await
    }
}
