use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Products {
    Table,
    InventoryAccountId,
    CostOfSalesAccountId,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column(ColumnDef::new(Products::InventoryAccountId).big_integer())
                    .add_column(ColumnDef::new(Products::CostOfSalesAccountId).big_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_products_inventory_account_id")
                    .from(Products::Table, Products::InventoryAccountId)
                    .to(Accounts::Table, Accounts::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_products_cost_of_sales_account_id")
                    .from(Products::Table, Products::CostOfSalesAccountId)
                    .to(Accounts::Table, Accounts::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_products_inventory_account_id")
                    .table(Products::Table)
                    .col(Products::InventoryAccountId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_products_cost_of_sales_account_id")
                    .table(Products::Table)
                    .col(Products::CostOfSalesAccountId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_products_cost_of_sales_account_id")
                    .table(Products::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_products_inventory_account_id")
                    .table(Products::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::CostOfSalesAccountId)
                    .drop_column(Products::InventoryAccountId)
                    .to_owned(),
            )
            .await
    }
}
