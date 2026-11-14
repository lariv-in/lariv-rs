use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TestCases::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestCases::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestCases::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(TestCases::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(TestCases::Name).text().not_null())
                    .col(ColumnDef::new(TestCases::Definition).text().not_null())
                    .col(ColumnDef::new(TestCases::Solution).text().not_null())
                    .col(
                        ColumnDef::new(TestCases::CreatedById)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_test_cases_created_by_id")
                            .from(TestCases::Table, TestCases::CreatedById)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_test_cases_name")
                    .table(TestCases::Table)
                    .col(TestCases::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_test_cases_created_by_id")
                    .table(TestCases::Table)
                    .col(TestCases::CreatedById)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TestCases::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum TestCases {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    Definition,
    Solution,
    CreatedById,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
