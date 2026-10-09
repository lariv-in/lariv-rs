use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum InventoryStocks {
    #[iden = "inventory_stocks"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    CompanyId,
    QtyType,
    QtyUnit,
}

#[derive(Iden)]
enum InventoryStockMovements {
    #[iden = "inventory_stock_movements"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Datetime,
    MovementType,
}

#[derive(Iden)]
enum InventoryStockMovementLines {
    #[iden = "inventory_stock_movement_lines"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    StockId,
    StockMovementId,
    Qty,
    QtyUnit,
    QtyType,
}

#[derive(Iden)]
enum CrmCompanies {
    #[iden = "crm_companies"]
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(InventoryStocks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InventoryStocks::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(InventoryStocks::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(InventoryStocks::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(InventoryStocks::Name).text().not_null())
                    .col(
                        ColumnDef::new(InventoryStocks::CompanyId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryStocks::QtyType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(InventoryStocks::QtyUnit).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_inventory_stocks_company_id")
                            .from(InventoryStocks::Table, InventoryStocks::CompanyId)
                            .to(CrmCompanies::Table, CrmCompanies::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stocks_company_id")
                    .table(InventoryStocks::Table)
                    .col(InventoryStocks::CompanyId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(InventoryStockMovements::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InventoryStockMovements::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovements::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovements::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovements::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovements::MovementType)
                            .string_len(32)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(InventoryStockMovementLines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InventoryStockMovementLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovementLines::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovementLines::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovementLines::StockId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovementLines::StockMovementId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovementLines::Qty)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovementLines::QtyUnit)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InventoryStockMovementLines::QtyType)
                            .string_len(32)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_inventory_stock_movement_lines_stock_id")
                            .from(
                                InventoryStockMovementLines::Table,
                                InventoryStockMovementLines::StockId,
                            )
                            .to(InventoryStocks::Table, InventoryStocks::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_inventory_stock_movement_lines_movement_id")
                            .from(
                                InventoryStockMovementLines::Table,
                                InventoryStockMovementLines::StockMovementId,
                            )
                            .to(InventoryStockMovements::Table, InventoryStockMovements::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movement_lines_stock_id")
                    .table(InventoryStockMovementLines::Table)
                    .col(InventoryStockMovementLines::StockId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movement_lines_movement_id")
                    .table(InventoryStockMovementLines::Table)
                    .col(InventoryStockMovementLines::StockMovementId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(InventoryStockMovementLines::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(InventoryStockMovements::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(InventoryStocks::Table).to_owned())
            .await
    }
}
