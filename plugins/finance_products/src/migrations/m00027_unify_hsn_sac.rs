//! Products store one tax classification code: HSN for goods, SAC for services.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

const UP: &str = r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'products'
          AND column_name = 'sac_code'
    ) THEN
        UPDATE products
           SET hsn_code = sac_code
         WHERE sac_code IS NOT NULL
           AND hsn_code IS NULL;
        ALTER TABLE products DROP COLUMN sac_code;
    END IF;
END $$;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, UP).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
