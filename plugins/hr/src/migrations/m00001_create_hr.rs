use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrApplicants {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Mobile,
    Email,
}

#[derive(DeriveIden)]
enum HrProbations {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Mobile,
    Email,
    StartedAt,
}

#[derive(DeriveIden)]
enum HrEmployees {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Mobile,
    Email,
    HiredAt,
}

#[derive(DeriveIden)]
enum HrExEmployees {
    Table,
    Id,
    CreatedAt,
    Name,
    Mobile,
    Email,
    TerminatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrApplicants::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrApplicants::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrApplicants::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrApplicants::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrApplicants::Name).text().not_null())
                    .col(ColumnDef::new(HrApplicants::Mobile).text().not_null())
                    .col(ColumnDef::new(HrApplicants::Email).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrProbations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrProbations::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrProbations::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrProbations::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrProbations::Name).text().not_null())
                    .col(ColumnDef::new(HrProbations::Mobile).text().not_null())
                    .col(ColumnDef::new(HrProbations::Email).text().not_null())
                    .col(
                        ColumnDef::new(HrProbations::StartedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrEmployees::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrEmployees::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrEmployees::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrEmployees::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrEmployees::Name).text().not_null())
                    .col(ColumnDef::new(HrEmployees::Mobile).text().not_null())
                    .col(ColumnDef::new(HrEmployees::Email).text().not_null())
                    .col(
                        ColumnDef::new(HrEmployees::HiredAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrExEmployees::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrExEmployees::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrExEmployees::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrExEmployees::Name).text().not_null())
                    .col(ColumnDef::new(HrExEmployees::Mobile).text().not_null())
                    .col(ColumnDef::new(HrExEmployees::Email).text().not_null())
                    .col(
                        ColumnDef::new(HrExEmployees::TerminatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrExEmployees::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(HrEmployees::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(HrProbations::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(HrApplicants::Table).to_owned())
            .await
    }
}
