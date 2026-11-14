//! Remap quotation service lines onto unified service products.
use crate::foreign_keys::FK_QUOTATION_SERVICE_LINE_SERVICE;
use lariv_core::db::migration_sql::exec_sql;

use lariv_plugin_finance_products::migrations::PRODUCT_SERVICE_ID_MAP;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            &format!(
                r#"DO $$
                BEGIN
                    IF to_regclass('public.{PRODUCT_SERVICE_ID_MAP}') IS NOT NULL THEN
                        UPDATE quotation_service_lines AS line
                           SET service_id = map.product_id
                          FROM {PRODUCT_SERVICE_ID_MAP} AS map
                         WHERE line.service_id = map.service_id;
                    END IF;
                END $$;"#
            ),
        )
        .await?;
        exec_sql(
            manager,
            &format!(
                "ALTER TABLE quotation_service_lines DROP CONSTRAINT IF EXISTS {}",
                FK_QUOTATION_SERVICE_LINE_SERVICE.constraint
            ),
        )
        .await?;
        exec_sql(
            manager,
            &format!(
                r#"ALTER TABLE quotation_service_lines
               ADD CONSTRAINT {}
               FOREIGN KEY (service_id) REFERENCES products(id)
               ON UPDATE CASCADE ON DELETE RESTRICT"#,
                FK_QUOTATION_SERVICE_LINE_SERVICE.constraint
            ),
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            &format!(
                "ALTER TABLE quotation_service_lines DROP CONSTRAINT IF EXISTS {}",
                FK_QUOTATION_SERVICE_LINE_SERVICE.constraint
            ),
        )
        .await?;
        exec_sql(
            manager,
            &format!(
                r#"ALTER TABLE quotation_service_lines
               ADD CONSTRAINT {}
               FOREIGN KEY (service_id) REFERENCES services(id)
               ON UPDATE CASCADE ON DELETE RESTRICT"#,
                FK_QUOTATION_SERVICE_LINE_SERVICE.constraint
            ),
        )
        .await?;
        Ok(())
    }
}
