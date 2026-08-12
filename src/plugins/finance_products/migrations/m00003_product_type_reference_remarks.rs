use sea_orm_migration::prelude::extension::postgres::PgExpr;
use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
    ProductType,
    Reference,
    Remarks,
}

#[derive(DeriveIden)]
enum ProductType {
    #[sea_orm(iden = "product_type")]
    Enum,
    #[sea_orm(iden = "Goods")]
    Goods,
    #[sea_orm(iden = "Services")]
    Services,
    #[sea_orm(iden = "Both")]
    Both,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(ProductType::Enum)
                    .values([ProductType::Goods, ProductType::Services, ProductType::Both])
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column(
                        ColumnDef::new(Products::ProductType)
                            .custom(ProductType::Enum)
                            .not_null()
                            .default("Goods"),
                    )
                    .add_column(ColumnDef::new(Products::Reference).text())
                    .add_column(ColumnDef::new(Products::Remarks).text())
                    .to_owned(),
            )
            .await?;

        let backend = manager.get_connection().get_database_backend();
        let update = Query::update()
            .table(Products::Table)
            .value(
                Products::Reference,
                Expr::val("LEGACY-")
                    .concat(Expr::col(Products::Id).cast_as(Alias::new("TEXT"))),
            )
            .and_where(Expr::col(Products::Reference).is_null())
            .to_owned();
        manager.get_connection().execute(backend.build(&update)).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .modify_column(ColumnDef::new(Products::Reference).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uq_products_reference")
                    .table(Products::Table)
                    .col(Products::Reference)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("uq_products_reference")
                    .table(Products::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::Remarks)
                    .drop_column(Products::Reference)
                    .drop_column(Products::ProductType)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_type(Type::drop().name(ProductType::Enum).to_owned())
            .await
    }
}
