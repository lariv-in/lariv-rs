//! Optional vehicle departure time on in and out stock movements.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden, Clone, Copy)]
enum VehicleOut {
    VehicleOutDatetime,
}

#[derive(Iden, Clone, Copy)]
enum MovementsIn {
    #[iden = "inventory_stock_movements_in"]
    Table,
}

#[derive(Iden, Clone, Copy)]
enum MovementsOut {
    #[iden = "inventory_stock_movements_out"]
    Table,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_vehicle_out(manager, MovementsIn::Table).await?;
        add_vehicle_out(manager, MovementsOut::Table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_vehicle_out(manager, MovementsOut::Table).await?;
        drop_vehicle_out(manager, MovementsIn::Table).await
    }
}

async fn add_vehicle_out(
    manager: &SchemaManager<'_>,
    table: impl Iden + Copy + 'static,
) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .add_column(
                    ColumnDef::new(VehicleOut::VehicleOutDatetime)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .to_owned(),
        )
        .await
}

async fn drop_vehicle_out(
    manager: &SchemaManager<'_>,
    table: impl Iden + Copy + 'static,
) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .drop_column(VehicleOut::VehicleOutDatetime)
                .to_owned(),
        )
        .await
}
