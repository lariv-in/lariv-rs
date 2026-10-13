//! Each stock movement out points at exactly one in movement or one process movement.

use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden, Clone, Copy)]
enum MovementsOut {
    #[iden = "inventory_stock_movements_out"]
    Table,
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
        for (name, col) in [
            (
                "idx_inventory_stock_movements_out_source_in_id",
                MovementsOut::SourceInId,
            ),
            (
                "idx_inventory_stock_movements_out_source_process_id",
                MovementsOut::SourceProcessId,
            ),
        ] {
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name(name)
                        .table(MovementsOut::Table)
                        .col(col)
                        .to_owned(),
                )
                .await?;
        }
        // SQLite cannot add a foreign key or a check to an existing table.
        // New writes still require exactly one source in the application.
        if is_postgres(manager) {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_inventory_stock_movements_out_source_in_id")
                        .from(MovementsOut::Table, MovementsOut::SourceInId)
                        .to(MovementsIn::Table, MovementsIn::Id)
                        .on_delete(ForeignKeyAction::Restrict)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_inventory_stock_movements_out_source_process_id")
                        .from(MovementsOut::Table, MovementsOut::SourceProcessId)
                        .to(MovementsProcess::Table, MovementsProcess::Id)
                        .on_delete(ForeignKeyAction::Restrict)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
            // NOT VALID so outs saved before a source was required stay readable.
            // Inserts and updates must name exactly one in movement or one process movement.
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements_out \
                 DROP CONSTRAINT IF EXISTS ck_inventory_stock_movements_out_source",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements_out \
                 ADD CONSTRAINT ck_inventory_stock_movements_out_source CHECK ( \
                   (source_in_id IS NOT NULL AND source_process_id IS NULL) \
                   OR (source_in_id IS NULL AND source_process_id IS NOT NULL) \
                 ) NOT VALID",
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements_out \
                 DROP CONSTRAINT IF EXISTS ck_inventory_stock_movements_out_source",
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
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_inventory_stock_movements_out_source_in_id")
                        .table(MovementsOut::Table)
                        .to_owned(),
                )
                .await?;
        }
        for name in [
            "idx_inventory_stock_movements_out_source_process_id",
            "idx_inventory_stock_movements_out_source_in_id",
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
                    .drop_column(MovementsOut::SourceProcessId)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MovementsOut::Table)
                    .drop_column(MovementsOut::SourceInId)
                    .to_owned(),
            )
            .await
    }
}
