//! Drop the company stored on each stock.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum InventoryStocks {
    #[iden = "inventory_stocks"]
    Table,
    CompanyId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_inventory_stocks_company_id")
                    .table(InventoryStocks::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_inventory_stocks_company_id")
                    .table(InventoryStocks::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(InventoryStocks::Table)
                    .drop_column(InventoryStocks::CompanyId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "dropping the stock company is not reversible".into(),
        ))
    }
}
