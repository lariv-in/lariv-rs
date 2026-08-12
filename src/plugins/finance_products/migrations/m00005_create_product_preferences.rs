use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum ProductPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    InventoryAccountId,
    CostOfSalesAccountId,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProductPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProductPreferences::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ProductPreferences::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(ProductPreferences::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(ProductPreferences::DeletedAt).timestamp_with_time_zone(),
                    )
                    .col(ColumnDef::new(ProductPreferences::InventoryAccountId).big_integer())
                    .col(ColumnDef::new(ProductPreferences::CostOfSalesAccountId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_preferences_inventory_account_id")
                            .from(ProductPreferences::Table, ProductPreferences::InventoryAccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_preferences_cost_of_sales_account_id")
                            .from(
                                ProductPreferences::Table,
                                ProductPreferences::CostOfSalesAccountId,
                            )
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_product_preferences_deleted_at")
                    .table(ProductPreferences::Table)
                    .col(ProductPreferences::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_product_preferences_inventory_account_id")
                    .table(ProductPreferences::Table)
                    .col(ProductPreferences::InventoryAccountId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_product_preferences_cost_of_sales_account_id")
                    .table(ProductPreferences::Table)
                    .col(ProductPreferences::CostOfSalesAccountId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProductPreferences::Table).to_owned())
            .await
    }
}
