use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Customers {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
    AddressLine1,
    AddressLine2,
    City,
    Pincode,
    State,
    Gstin,
    Pan,
    Phone,
    Email,
    Website,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Customers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Customers::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Customers::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Customers::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Customers::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Customers::Name).text().not_null())
                    .col(ColumnDef::new(Customers::AddressLine1).text())
                    .col(ColumnDef::new(Customers::AddressLine2).text())
                    .col(ColumnDef::new(Customers::City).text())
                    .col(ColumnDef::new(Customers::Pincode).text())
                    .col(ColumnDef::new(Customers::State).text())
                    .col(ColumnDef::new(Customers::Gstin).text())
                    .col(ColumnDef::new(Customers::Pan).text())
                    .col(ColumnDef::new(Customers::Phone).text())
                    .col(ColumnDef::new(Customers::Email).text())
                    .col(ColumnDef::new(Customers::Website).text())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_customers_deleted_at")
                    .table(Customers::Table)
                    .col(Customers::DeletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Customers::Table).to_owned())
            .await
    }
}
