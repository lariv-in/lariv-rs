use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_connection().get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(
            manager,
            "DELETE FROM product_preferences WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            "DELETE FROM products WHERE deleted_at IS NOT NULL",
        )
        .await?;

        for (index, table) in [
            ("idx_product_preferences_deleted_at", "product_preferences"),
            ("idx_products_deleted_at", "products"),
        ] {
            manager
                .drop_index(Index::drop().name(index).table(Alias::new(table)).to_owned())
                .await?;
        }

        for table in ["product_preferences", "products"] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new(table))
                        .drop_column(Alias::new("deleted_at"))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in ["products", "product_preferences"] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new(table))
                        .add_column(
                            ColumnDef::new(Alias::new("deleted_at")).timestamp_with_time_zone(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        for (index, table) in [
            ("idx_products_deleted_at", "products"),
            ("idx_product_preferences_deleted_at", "product_preferences"),
        ] {
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name(index)
                        .table(Alias::new(table))
                        .col(Alias::new("deleted_at"))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
