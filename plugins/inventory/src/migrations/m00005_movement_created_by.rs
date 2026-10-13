//! Who created a stock movement. `created_at` is already on each header.

use lariv_core::db::migration_sql::is_postgres;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden, Clone, Copy)]
enum CreatedBy {
    CreatedById,
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

#[derive(Iden, Clone, Copy)]
enum MovementsProcess {
    #[iden = "inventory_stock_movements_process"]
    Table,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_created_by(manager, MovementsIn::Table, "inventory_stock_movements_in").await?;
        add_created_by(
            manager,
            MovementsOut::Table,
            "inventory_stock_movements_out",
        )
        .await?;
        add_created_by(
            manager,
            MovementsProcess::Table,
            "inventory_stock_movements_process",
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_created_by(
            manager,
            MovementsProcess::Table,
            "inventory_stock_movements_process",
        )
        .await?;
        drop_created_by(
            manager,
            MovementsOut::Table,
            "inventory_stock_movements_out",
        )
        .await?;
        drop_created_by(manager, MovementsIn::Table, "inventory_stock_movements_in").await
    }
}

async fn add_created_by(
    manager: &SchemaManager<'_>,
    table: impl Iden + Copy + 'static,
    prefix: &str,
) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .add_column(ColumnDef::new(CreatedBy::CreatedById).big_integer().null())
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name(format!("idx_{prefix}_created_by_id"))
                .table(table)
                .col(CreatedBy::CreatedById)
                .to_owned(),
        )
        .await?;
    // SQLite cannot add a foreign key to an existing table.
    if is_postgres(manager) {
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name(format!("fk_{prefix}_created_by_id"))
                    .from(table, CreatedBy::CreatedById)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
    }
    Ok(())
}

async fn drop_created_by(
    manager: &SchemaManager<'_>,
    table: impl Iden + Copy + 'static,
    prefix: &str,
) -> Result<(), DbErr> {
    if is_postgres(manager) {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name(format!("fk_{prefix}_created_by_id"))
                    .table(table)
                    .to_owned(),
            )
            .await?;
    }
    manager
        .drop_index(
            Index::drop()
                .name(format!("idx_{prefix}_created_by_id"))
                .table(table)
                .to_owned(),
        )
        .await?;
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .drop_column(CreatedBy::CreatedById)
                .to_owned(),
        )
        .await
}
