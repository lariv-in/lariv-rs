use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
    BaseCost,
    SalesPrice,
    HsnCode,
}

#[derive(DeriveIden)]
enum ProductTaxes {
    Table,
    ProductId,
    TaxId,
}

#[derive(DeriveIden)]
enum Taxes {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Products::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Products::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Products::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Products::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Products::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Products::Name).text().not_null())
                    .col(
                        ColumnDef::new(Products::BaseCost)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Products::SalesPrice)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Products::HsnCode).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_products_deleted_at")
                    .table(Products::Table)
                    .col(Products::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ProductTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProductTaxes::ProductId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ProductTaxes::TaxId).big_integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(ProductTaxes::ProductId)
                            .col(ProductTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_taxes_product_id")
                            .from(ProductTaxes::Table, ProductTaxes::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_taxes_tax_id")
                            .from(ProductTaxes::Table, ProductTaxes::TaxId)
                            .to(Taxes::Table, Taxes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProductTaxes::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Products::Table).to_owned())
            .await
    }
}
