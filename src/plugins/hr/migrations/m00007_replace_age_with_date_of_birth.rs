use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrApplicants {
    Table,
    Age,
    DateOfBirth,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrApplicants::Table)
                    .drop_column(HrApplicants::Age)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrApplicants::Table)
                    .add_column(
                        ColumnDef::new(HrApplicants::DateOfBirth)
                            .timestamp_with_time_zone()
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
                    .table(HrApplicants::Table)
                    .drop_column(HrApplicants::DateOfBirth)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrApplicants::Table)
                    .add_column(ColumnDef::new(HrApplicants::Age).big_integer().null())
                    .to_owned(),
            )
            .await
    }
}
