//! Replace the single out-movement source with links to many in and process movements.

use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden, Clone, Copy)]
enum OutIns {
    #[iden = "inventory_stock_movement_out_ins"]
    Table,
    OutId,
    InId,
}

#[derive(Iden, Clone, Copy)]
enum OutProcesses {
    #[iden = "inventory_stock_movement_out_processes"]
    Table,
    OutId,
    ProcessId,
}

#[derive(Iden, Clone, Copy)]
enum MovementsOut {
    #[iden = "inventory_stock_movements_out"]
    Table,
    Id,
    SourceInId,
    SourceProcessId,
}

#[derive(Iden, Clone, Copy)]
enum MovementsIn {
    #[iden = "inventory_stock_movements_in"]
    Table,
    Id,
}

#[derive(Iden, Clone, Copy)]
enum MovementsProcess {
    #[iden = "inventory_stock_movements_process"]
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OutIns::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(OutIns::OutId).big_integer().not_null())
                    .col(ColumnDef::new(OutIns::InId).big_integer().not_null())
                    .primary_key(Index::create().col(OutIns::OutId).col(OutIns::InId))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(OutProcesses::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(OutProcesses::OutId).big_integer().not_null())
                    .col(
                        ColumnDef::new(OutProcesses::ProcessId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(OutProcesses::OutId)
                            .col(OutProcesses::ProcessId),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movement_out_ins_in_id")
                    .table(OutIns::Table)
                    .col(OutIns::InId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movement_out_processes_process_id")
                    .table(OutProcesses::Table)
                    .col(OutProcesses::ProcessId)
                    .to_owned(),
            )
            .await?;
        if is_postgres(manager) {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_inventory_stock_movement_out_ins_out_id")
                        .from(OutIns::Table, OutIns::OutId)
                        .to(MovementsOut::Table, MovementsOut::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_inventory_stock_movement_out_ins_in_id")
                        .from(OutIns::Table, OutIns::InId)
                        .to(MovementsIn::Table, MovementsIn::Id)
                        .on_delete(ForeignKeyAction::Restrict)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_inventory_stock_movement_out_processes_out_id")
                        .from(OutProcesses::Table, OutProcesses::OutId)
                        .to(MovementsOut::Table, MovementsOut::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_inventory_stock_movement_out_processes_process_id")
                        .from(OutProcesses::Table, OutProcesses::ProcessId)
                        .to(MovementsProcess::Table, MovementsProcess::Id)
                        .on_delete(ForeignKeyAction::Restrict)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements_out \
                 DROP CONSTRAINT IF EXISTS ck_inventory_stock_movements_out_source",
            )
            .await?;
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_inventory_stock_movements_out_source_in_id")
                        .table(MovementsOut::Table)
                        .to_owned(),
                )
                .await?;
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_inventory_stock_movements_out_source_process_id")
                        .table(MovementsOut::Table)
                        .to_owned(),
                )
                .await?;
        }
        exec_sql(
            manager,
            "INSERT INTO inventory_stock_movement_out_ins (out_id, in_id) \
             SELECT id, source_in_id FROM inventory_stock_movements_out \
             WHERE source_in_id IS NOT NULL",
        )
        .await?;
        exec_sql(
            manager,
            "INSERT INTO inventory_stock_movement_out_processes (out_id, process_id) \
             SELECT id, source_process_id FROM inventory_stock_movements_out \
             WHERE source_process_id IS NOT NULL",
        )
        .await?;
        for name in [
            "idx_inventory_stock_movements_out_source_in_id",
            "idx_inventory_stock_movements_out_source_process_id",
        ] {
            manager
                .drop_index(
                    Index::drop()
                        .name(name)
                        .table(MovementsOut::Table)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .alter_table(
                Table::alter()
                    .table(MovementsOut::Table)
                    .drop_column(MovementsOut::SourceInId)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MovementsOut::Table)
                    .drop_column(MovementsOut::SourceProcessId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MovementsOut::Table)
                    .add_column(
                        ColumnDef::new(MovementsOut::SourceInId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MovementsOut::Table)
                    .add_column(
                        ColumnDef::new(MovementsOut::SourceProcessId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        exec_sql(
            manager,
            "UPDATE inventory_stock_movements_out SET source_in_id = ( \
               SELECT in_id FROM inventory_stock_movement_out_ins \
               WHERE out_id = inventory_stock_movements_out.id LIMIT 1 \
             )",
        )
        .await?;
        exec_sql(
            manager,
            "UPDATE inventory_stock_movements_out SET source_process_id = ( \
               SELECT process_id FROM inventory_stock_movement_out_processes \
               WHERE out_id = inventory_stock_movements_out.id LIMIT 1 \
             ) WHERE source_in_id IS NULL",
        )
        .await?;
        manager
            .drop_table(Table::drop().table(OutProcesses::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(OutIns::Table).to_owned())
            .await
    }
}
