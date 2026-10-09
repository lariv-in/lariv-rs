//! Optional e-way bill number on a stock movement.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum InventoryStockMovements {
    #[iden = "inventory_stock_movements"]
    Table,
    EwayBill,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(InventoryStockMovements::Table)
                    .add_column(ColumnDef::new(InventoryStockMovements::EwayBill).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(InventoryStockMovements::Table)
                    .drop_column(InventoryStockMovements::EwayBill)
                    .to_owned(),
            )
            .await
    }
}
