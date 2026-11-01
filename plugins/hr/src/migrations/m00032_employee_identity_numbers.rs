use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrEmployees {
    Table,
    PfNumber,
    AadharNumber,
    PanNumber,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .add_column(ColumnDef::new(HrEmployees::PfNumber).text().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .add_column(ColumnDef::new(HrEmployees::AadharNumber).text().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .add_column(ColumnDef::new(HrEmployees::PanNumber).text().null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .drop_column(HrEmployees::PanNumber)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .drop_column(HrEmployees::AadharNumber)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .drop_column(HrEmployees::PfNumber)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
