//! Party, vehicle, and number columns on stock movements, plus PDF preferences.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum InventoryStockMovements {
    #[iden = "inventory_stock_movements"]
    Table,
    Number,
    BillToIndividual,
    CustomerIndividual,
    CustomerCompany,
    VehicleType,
    VehicleNumber,
    DriverId,
}

#[derive(Iden)]
enum InventoryPreferences {
    #[iden = "inventory_preferences"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    MovementNumberFormat,
    CompanyName,
    CompanyAddress,
    CompanyPhone,
    CompanyEmail,
    CompanyGstin,
    TermsAndConditions,
    LogoVnodeId,
    SignatureVnodeId,
    MovementInTemplate,
    MovementOutTemplate,
}

#[derive(Iden)]
enum CrmContacts {
    #[iden = "crm_contacts"]
    Table,
    Id,
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
            .alter_table(
                Table::alter()
                    .table(InventoryStockMovements::Table)
                    .add_column(
                        ColumnDef::new(InventoryStockMovements::Number)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(InventoryStockMovements::BillToIndividual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .add_column(
                        ColumnDef::new(InventoryStockMovements::CustomerIndividual).big_integer(),
                    )
                    .add_column(
                        ColumnDef::new(InventoryStockMovements::CustomerCompany).big_integer(),
                    )
                    .add_column(ColumnDef::new(InventoryStockMovements::VehicleType).text())
                    .add_column(ColumnDef::new(InventoryStockMovements::VehicleNumber).text())
                    .add_column(ColumnDef::new(InventoryStockMovements::DriverId).big_integer())
                    .to_owned(),
            )
            .await?;

        exec_sql(
            manager,
            "UPDATE inventory_stock_movements SET number = 'SM-' || CAST(id AS TEXT) WHERE number = ''",
        )
        .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_inventory_stock_movements_customer_individual")
                    .from(
                        InventoryStockMovements::Table,
                        InventoryStockMovements::CustomerIndividual,
                    )
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
                    .from(
                        InventoryStockMovements::Table,
                        InventoryStockMovements::CustomerCompany,
                    )
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
                    .from(
                        InventoryStockMovements::Table,
                        InventoryStockMovements::DriverId,
                    )
                    .to(CrmContacts::Table, CrmContacts::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movements_customer_individual")
                    .table(InventoryStockMovements::Table)
                    .col(InventoryStockMovements::CustomerIndividual)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movements_customer_company")
                    .table(InventoryStockMovements::Table)
                    .col(InventoryStockMovements::CustomerCompany)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_inventory_stock_movements_driver_id")
                    .table(InventoryStockMovements::Table)
                    .col(InventoryStockMovements::DriverId)
                    .to_owned(),
            )
            .await?;

        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            // NOT VALID so movements created before a customer was required stay readable.
            // New writes and updates still must name exactly one party.
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements \
                 DROP CONSTRAINT IF EXISTS ck_inventory_stock_movements_bill_to",
            )
            .await?;
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements \
                 ADD CONSTRAINT ck_inventory_stock_movements_bill_to CHECK ( \
                   (bill_to_individual AND customer_individual IS NOT NULL AND customer_company IS NULL) \
                   OR (NOT bill_to_individual AND customer_company IS NOT NULL AND customer_individual IS NULL) \
                 ) NOT VALID",
            )
            .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(InventoryPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InventoryPreferences::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(InventoryPreferences::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(InventoryPreferences::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(InventoryPreferences::MovementNumberFormat).text())
                    .col(ColumnDef::new(InventoryPreferences::CompanyName).text())
                    .col(ColumnDef::new(InventoryPreferences::CompanyAddress).text())
                    .col(ColumnDef::new(InventoryPreferences::CompanyPhone).text())
                    .col(ColumnDef::new(InventoryPreferences::CompanyEmail).text())
                    .col(ColumnDef::new(InventoryPreferences::CompanyGstin).text())
                    .col(ColumnDef::new(InventoryPreferences::TermsAndConditions).text())
                    .col(ColumnDef::new(InventoryPreferences::LogoVnodeId).big_integer())
                    .col(ColumnDef::new(InventoryPreferences::SignatureVnodeId).big_integer())
                    .col(ColumnDef::new(InventoryPreferences::MovementInTemplate).text())
                    .col(ColumnDef::new(InventoryPreferences::MovementOutTemplate).text())
                    .to_owned(),
            )
            .await?;

        exec_sql(
            manager,
            "INSERT INTO inventory_preferences (id, movement_number_format) \
             SELECT 1, 'SM-{{YYYY}}-{{SEQ}}' \
             WHERE NOT EXISTS (SELECT 1 FROM inventory_preferences WHERE id = 1)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(InventoryPreferences::Table).to_owned())
            .await?;
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements \
                 DROP CONSTRAINT IF EXISTS ck_inventory_stock_movements_bill_to",
            )
            .await?;
        }
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_inventory_stock_movements_customer_individual")
                    .table(InventoryStockMovements::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_inventory_stock_movements_customer_company")
                    .table(InventoryStockMovements::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_inventory_stock_movements_driver_id")
                    .table(InventoryStockMovements::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(InventoryStockMovements::Table)
                    .drop_column(InventoryStockMovements::Number)
                    .drop_column(InventoryStockMovements::BillToIndividual)
                    .drop_column(InventoryStockMovements::CustomerIndividual)
                    .drop_column(InventoryStockMovements::CustomerCompany)
                    .drop_column(InventoryStockMovements::VehicleType)
                    .drop_column(InventoryStockMovements::VehicleNumber)
                    .drop_column(InventoryStockMovements::DriverId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
