use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum ProductPreferencesTaxes {
    Table,
    ProductPreferencesId,
    TaxId,
}

#[derive(DeriveIden)]
enum ProductPreferences {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Taxes {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProductPreferencesTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProductPreferencesTaxes::ProductPreferencesId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProductPreferencesTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ProductPreferencesTaxes::ProductPreferencesId)
                            .col(ProductPreferencesTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_preferences_taxes_preferences_id")
                            .from(
                                ProductPreferencesTaxes::Table,
                                ProductPreferencesTaxes::ProductPreferencesId,
                            )
                            .to(ProductPreferences::Table, ProductPreferences::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_preferences_taxes_tax_id")
                            .from(
                                ProductPreferencesTaxes::Table,
                                ProductPreferencesTaxes::TaxId,
                            )
                            .to(Taxes::Table, Taxes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ProductPreferencesTaxes::Table)
                    .to_owned(),
            )
            .await
    }
}
