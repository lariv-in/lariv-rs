//! Optional customer on a process movement. Existing rows stay empty.

use lariv_core::db::migration_sql::is_postgres;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden, Clone, Copy)]
enum MovementsProcess {
    #[iden = "inventory_stock_movements_process"]
    Table,
    CustomerId,
}

#[derive(Iden, Clone, Copy)]
enum CrmCustomers {
    #[iden = "crm_customers"]
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MovementsProcess::Table)
                    .add_column(
                        ColumnDef::new(MovementsProcess::CustomerId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movements_process_customer_id")
                    .table(MovementsProcess::Table)
                    .col(MovementsProcess::CustomerId)
                    .to_owned(),
            )
            .await?;
        // SQLite cannot add a foreign key to an existing table.
        if is_postgres(manager) {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_inventory_stock_movements_process_customer_id")
                        .from(MovementsProcess::Table, MovementsProcess::CustomerId)
                        .to(CrmCustomers::Table, CrmCustomers::Id)
                        .on_delete(ForeignKeyAction::Restrict)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_inventory_stock_movements_process_customer_id")
                        .table(MovementsProcess::Table)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_inventory_stock_movements_process_customer_id")
                    .table(MovementsProcess::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MovementsProcess::Table)
                    .drop_column(MovementsProcess::CustomerId)
                    .to_owned(),
            )
            .await
    }
}
