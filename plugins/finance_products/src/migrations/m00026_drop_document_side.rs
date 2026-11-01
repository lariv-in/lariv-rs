//! One journal template per product; drop sale/purchase document_side.

use sea_orm_migration::prelude::*;

const UP: &str = r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'product_journal_entry_item_templates'
          AND column_name = 'document_side'
    ) THEN
        -- Prefer Sale rows when both sides exist on the same product.
        DELETE FROM product_journal_entry_item_templates AS purchase
         WHERE purchase.document_side = 'Purchase'
           AND EXISTS (
               SELECT 1
                 FROM product_journal_entry_item_templates AS sale
                WHERE sale.product_id = purchase.product_id
                  AND sale.document_side = 'Sale'
           );
        ALTER TABLE product_journal_entry_item_templates
            DROP COLUMN document_side;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'product_preference_journal_items'
          AND column_name = 'document_side'
    ) THEN
        DELETE FROM product_preference_journal_items AS purchase
         WHERE purchase.document_side = 'Purchase'
           AND EXISTS (
               SELECT 1
                 FROM product_preference_journal_items AS sale
                WHERE sale.product_preferences_id = purchase.product_preferences_id
                  AND sale.document_side = 'Sale'
           );
        ALTER TABLE product_preference_journal_items
            DROP COLUMN document_side;
    END IF;
END $$;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
