use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Taxes {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
    Percentage,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Taxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Taxes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Taxes::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Taxes::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Taxes::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Taxes::Name).text().not_null())
                    .col(
                        ColumnDef::new(Taxes::Percentage)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_taxes_deleted_at")
                    .table(Taxes::Table)
                    .col(Taxes::DeletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Taxes::Table).to_owned())
            .await
    }
}
