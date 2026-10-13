//! Split stock movements into in, out, and process tables.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden, Clone, Copy)]
enum Header {
    Id,
    CreatedAt,
    UpdatedAt,
    Datetime,
    Number,
    BillToIndividual,
    CustomerIndividual,
    CustomerCompany,
    VehicleType,
    VehicleNumber,
    EwayBill,
    DriverId,
    DeliveryChallan,
}

#[derive(Iden, Clone, Copy)]
enum LineCol {
    Id,
    CreatedAt,
    UpdatedAt,
    StockId,
    MovementId,
    Qty,
    QtyUnit,
    QtyType,
    Side,
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

#[derive(Iden, Clone, Copy)]
enum LinesIn {
    #[iden = "inventory_stock_movement_in_lines"]
    Table,
}

#[derive(Iden, Clone, Copy)]
enum LinesOut {
    #[iden = "inventory_stock_movement_out_lines"]
    Table,
}

#[derive(Iden, Clone, Copy)]
enum LinesProcess {
    #[iden = "inventory_stock_movement_process_lines"]
    Table,
}

#[derive(Iden, Clone, Copy)]
enum OldMovements {
    #[iden = "inventory_stock_movements"]
    Table,
}

#[derive(Iden, Clone, Copy)]
enum OldLines {
    #[iden = "inventory_stock_movement_lines"]
    Table,
}

#[derive(Iden, Clone, Copy)]
enum Stocks {
    #[iden = "inventory_stocks"]
    Table,
    Id,
}

#[derive(Iden, Clone, Copy)]
enum CrmContacts {
    #[iden = "crm_contacts"]
    Table,
    Id,
}

#[derive(Iden, Clone, Copy)]
enum CrmCompanies {
    #[iden = "crm_companies"]
    Table,
    Id,
}

#[derive(Iden, Clone, Copy)]
enum FilesystemNodes {
    #[iden = "filesystem_nodes"]
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_party_table(manager, MovementsIn::Table, true).await?;
        create_party_table(manager, MovementsOut::Table, false).await?;
        create_process_table(manager).await?;
        create_line_table(manager, LinesIn::Table, MovementsIn::Table, false).await?;
        create_line_table(manager, LinesOut::Table, MovementsOut::Table, false).await?;
        create_line_table(manager, LinesProcess::Table, MovementsProcess::Table, true).await?;

        party_fks(manager, MovementsIn::Table, "inventory_stock_movements_in").await?;
        party_fks(
            manager,
            MovementsOut::Table,
            "inventory_stock_movements_out",
        )
        .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_inventory_stock_movements_in_delivery_challan")
                    .from(MovementsIn::Table, Header::DeliveryChallan)
                    .to(FilesystemNodes::Table, FilesystemNodes::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movements_in_delivery_challan")
                    .table(MovementsIn::Table)
                    .col(Header::DeliveryChallan)
                    .to_owned(),
            )
            .await?;

        line_fks(
            manager,
            LinesIn::Table,
            MovementsIn::Table,
            "inventory_stock_movement_in_lines",
        )
        .await?;
        line_fks(
            manager,
            LinesOut::Table,
            MovementsOut::Table,
            "inventory_stock_movement_out_lines",
        )
        .await?;
        line_fks(
            manager,
            LinesProcess::Table,
            MovementsProcess::Table,
            "inventory_stock_movement_process_lines",
        )
        .await?;

        bill_to_check(
            manager,
            "inventory_stock_movements_in",
            "ck_inventory_stock_movements_in_bill_to",
        )
        .await?;
        bill_to_check(
            manager,
            "inventory_stock_movements_out",
            "ck_inventory_stock_movements_out_bill_to",
        )
        .await?;

        copy_existing(manager).await?;
        reset_id_seq(manager, "inventory_stock_movements_in").await?;
        reset_id_seq(manager, "inventory_stock_movements_out").await?;
        reset_id_seq(manager, "inventory_stock_movement_in_lines").await?;
        reset_id_seq(manager, "inventory_stock_movement_out_lines").await?;

        manager
            .drop_table(Table::drop().table(OldLines::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(OldMovements::Table).to_owned())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_old_movement_table(manager).await?;
        create_old_line_table(manager).await?;
        exec_sql(
            manager,
            "INSERT INTO inventory_stock_movements (
                id, created_at, updated_at, datetime, movement_type, number,
                bill_to_individual, customer_individual, customer_company,
                vehicle_type, vehicle_number, eway_bill, driver_id
            )
            SELECT id, created_at, updated_at, datetime, 'in', number,
                bill_to_individual, customer_individual, customer_company,
                vehicle_type, vehicle_number, eway_bill, driver_id
            FROM inventory_stock_movements_in",
        )
        .await?;
        exec_sql(
            manager,
            "INSERT INTO inventory_stock_movements (
                id, created_at, updated_at, datetime, movement_type, number,
                bill_to_individual, customer_individual, customer_company,
                vehicle_type, vehicle_number, eway_bill, driver_id
            )
            SELECT id, created_at, updated_at, datetime, 'out', number,
                bill_to_individual, customer_individual, customer_company,
                vehicle_type, vehicle_number, eway_bill, driver_id
            FROM inventory_stock_movements_out",
        )
        .await?;
        exec_sql(
            manager,
            "INSERT INTO inventory_stock_movement_lines (
                id, created_at, updated_at, stock_id, stock_movement_id, qty, qty_unit, qty_type
            )
            SELECT id, created_at, updated_at, stock_id, movement_id, qty, qty_unit, qty_type
            FROM inventory_stock_movement_in_lines",
        )
        .await?;
        exec_sql(
            manager,
            "INSERT INTO inventory_stock_movement_lines (
                id, created_at, updated_at, stock_id, stock_movement_id, qty, qty_unit, qty_type
            )
            SELECT id, created_at, updated_at, stock_id, movement_id, qty, qty_unit, qty_type
            FROM inventory_stock_movement_out_lines",
        )
        .await?;
        old_fks(manager).await?;
        bill_to_check(
            manager,
            "inventory_stock_movements",
            "ck_inventory_stock_movements_bill_to",
        )
        .await?;
        reset_id_seq(manager, "inventory_stock_movements").await?;
        reset_id_seq(manager, "inventory_stock_movement_lines").await?;

        manager
            .drop_table(Table::drop().table(LinesProcess::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(LinesOut::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(LinesIn::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MovementsProcess::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MovementsOut::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MovementsIn::Table).to_owned())
            .await?;
        Ok(())
    }
}

fn pk_and_timestamps(mut table: TableCreateStatement) -> TableCreateStatement {
    table
        .col(
            ColumnDef::new(Header::Id)
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        .col(ColumnDef::new(Header::CreatedAt).timestamp_with_time_zone())
        .col(ColumnDef::new(Header::UpdatedAt).timestamp_with_time_zone())
        .col(
            ColumnDef::new(Header::Datetime)
                .timestamp_with_time_zone()
                .not_null(),
        )
        .col(ColumnDef::new(Header::Number).text().not_null())
        .to_owned()
}

async fn create_party_table(
    manager: &SchemaManager<'_>,
    table: impl Iden + 'static,
    challan: bool,
) -> Result<(), DbErr> {
    let mut stmt = pk_and_timestamps(Table::create().table(table).if_not_exists().to_owned());
    stmt.col(
        ColumnDef::new(Header::BillToIndividual)
            .boolean()
            .not_null(),
    )
    .col(ColumnDef::new(Header::CustomerIndividual).big_integer())
    .col(ColumnDef::new(Header::CustomerCompany).big_integer())
    .col(ColumnDef::new(Header::VehicleType).text())
    .col(ColumnDef::new(Header::VehicleNumber).text())
    .col(ColumnDef::new(Header::EwayBill).text())
    .col(ColumnDef::new(Header::DriverId).big_integer());
    if challan {
        stmt.col(ColumnDef::new(Header::DeliveryChallan).big_integer());
    }
    manager.create_table(stmt).await
}

async fn create_process_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(pk_and_timestamps(
            Table::create()
                .table(MovementsProcess::Table)
                .if_not_exists()
                .to_owned(),
        ))
        .await
}

async fn create_line_table(
    manager: &SchemaManager<'_>,
    table: impl Iden + 'static,
    _parent: impl Iden + 'static,
    side: bool,
) -> Result<(), DbErr> {
    let mut stmt = Table::create()
        .table(table)
        .if_not_exists()
        .col(
            ColumnDef::new(LineCol::Id)
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        .col(ColumnDef::new(LineCol::CreatedAt).timestamp_with_time_zone())
        .col(ColumnDef::new(LineCol::UpdatedAt).timestamp_with_time_zone())
        .col(ColumnDef::new(LineCol::StockId).big_integer().not_null())
        .col(ColumnDef::new(LineCol::MovementId).big_integer().not_null())
        .col(ColumnDef::new(LineCol::Qty).decimal_len(19, 6).not_null())
        .col(ColumnDef::new(LineCol::QtyUnit).text().not_null())
        .col(ColumnDef::new(LineCol::QtyType).string_len(32).not_null())
        .to_owned();
    if side {
        stmt.col(ColumnDef::new(LineCol::Side).string_len(16).not_null());
    }
    manager.create_table(stmt).await
}

async fn party_fks(
    manager: &SchemaManager<'_>,
    table: impl Iden + Copy + 'static,
    prefix: &str,
) -> Result<(), DbErr> {
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name(format!("fk_{prefix}_customer_individual"))
                .from(table, Header::CustomerIndividual)
                .to(CrmContacts::Table, CrmContacts::Id)
                .on_delete(ForeignKeyAction::Restrict)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name(format!("fk_{prefix}_customer_company"))
                .from(table, Header::CustomerCompany)
                .to(CrmCompanies::Table, CrmCompanies::Id)
                .on_delete(ForeignKeyAction::Restrict)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name(format!("fk_{prefix}_driver_id"))
                .from(table, Header::DriverId)
                .to(CrmContacts::Table, CrmContacts::Id)
                .on_delete(ForeignKeyAction::Restrict)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    for (suffix, col) in [
        ("customer_individual", Header::CustomerIndividual),
        ("customer_company", Header::CustomerCompany),
        ("driver_id", Header::DriverId),
    ] {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name(format!("idx_{prefix}_{suffix}"))
                    .table(table)
                    .col(col)
                    .to_owned(),
            )
            .await?;
    }
    Ok(())
}

async fn line_fks(
    manager: &SchemaManager<'_>,
    table: impl Iden + Copy + 'static,
    parent: impl Iden + Copy + 'static,
    prefix: &str,
) -> Result<(), DbErr> {
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name(format!("fk_{prefix}_stock_id"))
                .from(table, LineCol::StockId)
                .to(Stocks::Table, Stocks::Id)
                .on_delete(ForeignKeyAction::Restrict)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name(format!("fk_{prefix}_movement_id"))
                .from(table, LineCol::MovementId)
                .to(parent, Header::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name(format!("idx_{prefix}_stock_id"))
                .table(table)
                .col(LineCol::StockId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name(format!("idx_{prefix}_movement_id"))
                .table(table)
                .col(LineCol::MovementId)
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn bill_to_check(manager: &SchemaManager<'_>, table: &str, name: &str) -> Result<(), DbErr> {
    if manager.get_database_backend() != sea_orm::DatabaseBackend::Postgres {
        return Ok(());
    }
    exec_sql(
        manager,
        &format!("ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {name}"),
    )
    .await?;
    exec_sql(
        manager,
        &format!(
            "ALTER TABLE {table} ADD CONSTRAINT {name} CHECK ( \
               (bill_to_individual AND customer_individual IS NOT NULL AND customer_company IS NULL) \
               OR (NOT bill_to_individual AND customer_company IS NOT NULL AND customer_individual IS NULL) \
             ) NOT VALID"
        ),
    )
    .await
}

async fn copy_existing(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    exec_sql(
        manager,
        "INSERT INTO inventory_stock_movements_in (
            id, created_at, updated_at, datetime, number, bill_to_individual,
            customer_individual, customer_company, vehicle_type, vehicle_number,
            eway_bill, driver_id, delivery_challan
        )
        SELECT id, created_at, updated_at, datetime, number, bill_to_individual,
            customer_individual, customer_company, vehicle_type, vehicle_number,
            eway_bill, driver_id, NULL
        FROM inventory_stock_movements
        WHERE movement_type = 'in'",
    )
    .await?;
    exec_sql(
        manager,
        "INSERT INTO inventory_stock_movements_out (
            id, created_at, updated_at, datetime, number, bill_to_individual,
            customer_individual, customer_company, vehicle_type, vehicle_number,
            eway_bill, driver_id
        )
        SELECT id, created_at, updated_at, datetime, number, bill_to_individual,
            customer_individual, customer_company, vehicle_type, vehicle_number,
            eway_bill, driver_id
        FROM inventory_stock_movements
        WHERE movement_type = 'out'",
    )
    .await?;
    exec_sql(
        manager,
        "INSERT INTO inventory_stock_movement_in_lines (
            id, created_at, updated_at, stock_id, movement_id, qty, qty_unit, qty_type
        )
        SELECT l.id, l.created_at, l.updated_at, l.stock_id, l.stock_movement_id,
            l.qty, l.qty_unit, l.qty_type
        FROM inventory_stock_movement_lines l
        INNER JOIN inventory_stock_movements m ON m.id = l.stock_movement_id
        WHERE m.movement_type = 'in'",
    )
    .await?;
    exec_sql(
        manager,
        "INSERT INTO inventory_stock_movement_out_lines (
            id, created_at, updated_at, stock_id, movement_id, qty, qty_unit, qty_type
        )
        SELECT l.id, l.created_at, l.updated_at, l.stock_id, l.stock_movement_id,
            l.qty, l.qty_unit, l.qty_type
        FROM inventory_stock_movement_lines l
        INNER JOIN inventory_stock_movements m ON m.id = l.stock_movement_id
        WHERE m.movement_type = 'out'",
    )
    .await?;
    Ok(())
}

async fn reset_id_seq(manager: &SchemaManager<'_>, table: &str) -> Result<(), DbErr> {
    if manager.get_database_backend() != sea_orm::DatabaseBackend::Postgres {
        return Ok(());
    }
    exec_sql(
        manager,
        &format!(
            "SELECT setval(pg_get_serial_sequence('{table}', 'id'), \
             COALESCE((SELECT MAX(id) FROM {table}), 1), \
             (SELECT MAX(id) FROM {table}) IS NOT NULL)"
        ),
    )
    .await
}

async fn create_old_movement_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(OldMovements::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Header::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(Header::CreatedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(Header::UpdatedAt).timestamp_with_time_zone())
                .col(
                    ColumnDef::new(Header::Datetime)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Alias::new("movement_type"))
                        .string_len(32)
                        .not_null(),
                )
                .col(ColumnDef::new(Header::Number).text().not_null())
                .col(
                    ColumnDef::new(Header::BillToIndividual)
                        .boolean()
                        .not_null(),
                )
                .col(ColumnDef::new(Header::CustomerIndividual).big_integer())
                .col(ColumnDef::new(Header::CustomerCompany).big_integer())
                .col(ColumnDef::new(Header::VehicleType).text())
                .col(ColumnDef::new(Header::VehicleNumber).text())
                .col(ColumnDef::new(Header::EwayBill).text())
                .col(ColumnDef::new(Header::DriverId).big_integer())
                .to_owned(),
        )
        .await
}

async fn create_old_line_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(OldLines::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(LineCol::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(LineCol::CreatedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(LineCol::UpdatedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(LineCol::StockId).big_integer().not_null())
                .col(
                    ColumnDef::new(Alias::new("stock_movement_id"))
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(LineCol::Qty).decimal_len(19, 6).not_null())
                .col(ColumnDef::new(LineCol::QtyUnit).text().not_null())
                .col(ColumnDef::new(LineCol::QtyType).string_len(32).not_null())
                .to_owned(),
        )
        .await
}

async fn old_fks(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_inventory_stock_movements_customer_individual")
                .from(OldMovements::Table, Header::CustomerIndividual)
                .to(CrmContacts::Table, CrmContacts::Id)
                .on_delete(ForeignKeyAction::Restrict)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_inventory_stock_movements_customer_company")
                .from(OldMovements::Table, Header::CustomerCompany)
                .to(CrmCompanies::Table, CrmCompanies::Id)
                .on_delete(ForeignKeyAction::Restrict)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_inventory_stock_movements_driver_id")
                .from(OldMovements::Table, Header::DriverId)
                .to(CrmContacts::Table, CrmContacts::Id)
                .on_delete(ForeignKeyAction::Restrict)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_inventory_stock_movement_lines_stock_id")
                .from(OldLines::Table, LineCol::StockId)
                .to(Stocks::Table, Stocks::Id)
                .on_delete(ForeignKeyAction::Restrict)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_inventory_stock_movement_lines_movement_id")
                .from(OldLines::Table, Alias::new("stock_movement_id"))
                .to(OldMovements::Table, Header::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    Ok(())
}
