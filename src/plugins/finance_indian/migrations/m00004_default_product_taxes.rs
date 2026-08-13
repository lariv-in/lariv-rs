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
enum Taxes {
    Table,
    Id,
    Name,
}

/// Intra-state CGST+SGST (9%+9%) for the 18% GST slab (IGST is selected when needed).
const DEFAULT_PRODUCT_TAX_NAMES: &[&str] = &["CGST 9%", "SGST 9%"];

fn preference_tax_link_exists() -> SimpleExpr {
    Expr::exists(
        Query::select()
            .expr(Expr::val(1))
            .from(ProductPreferencesTaxes::Table)
            .and_where(
                Expr::col((
                    ProductPreferencesTaxes::Table,
                    ProductPreferencesTaxes::ProductPreferencesId,
                ))
                .eq(1),
            )
            .and_where(
                Expr::col((
                    ProductPreferencesTaxes::Table,
                    ProductPreferencesTaxes::TaxId,
                ))
                .eq(Expr::col((Taxes::Table, Taxes::Id))),
            )
            .to_owned(),
    )
    .not()
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let conn = manager.get_connection();

        for name in DEFAULT_PRODUCT_TAX_NAMES {
            let insert = Query::insert()
                .into_table(ProductPreferencesTaxes::Table)
                .columns([
                    ProductPreferencesTaxes::ProductPreferencesId,
                    ProductPreferencesTaxes::TaxId,
                ])
                .select_from(
                    Query::select()
                        .expr(Expr::val(1))
                        .column(Taxes::Id)
                        .from(Taxes::Table)
                        .and_where(Expr::col(Taxes::Name).eq(*name))
                        .and_where(preference_tax_link_exists())
                        .to_owned(),
                )
                .unwrap()
                .to_owned();
            conn.execute(backend.build(&insert)).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_connection().get_database_backend();
        let conn = manager.get_connection();

        let delete = Query::delete()
            .from_table(ProductPreferencesTaxes::Table)
            .cond_where(
                Condition::all()
                    .add(Expr::col(ProductPreferencesTaxes::ProductPreferencesId).eq(1))
                    .add(
                        Expr::col(ProductPreferencesTaxes::TaxId).in_subquery(
                            Query::select()
                                .column(Taxes::Id)
                                .from(Taxes::Table)
                                .and_where(
                                    Expr::col(Taxes::Name)
                                        .is_in(DEFAULT_PRODUCT_TAX_NAMES.iter().copied()),
                                )
                                .to_owned(),
                        ),
                    ),
            )
            .to_owned();
        conn.execute(backend.build(&delete)).await.map(|_| ())
    }
}
