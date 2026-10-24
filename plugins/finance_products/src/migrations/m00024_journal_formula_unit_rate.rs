//! Point default product journal formulas at `unit_rate`.
//!
//! `m00023` seeded `pre_tax` and `-pre_tax`. Databases that already applied
//! that seed rewrite those exact formulas. Fresh installs already store
//! `unit_rate`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
UPDATE product_journal_entry_item_templates
   SET formula = 'unit_rate', updated_at = NOW()
 WHERE formula = 'pre_tax';
UPDATE product_journal_entry_item_templates
   SET formula = '-unit_rate', updated_at = NOW()
 WHERE formula = '-pre_tax';
UPDATE product_preference_journal_items
   SET formula = 'unit_rate', updated_at = NOW()
 WHERE formula = 'pre_tax';
UPDATE product_preference_journal_items
   SET formula = '-unit_rate', updated_at = NOW()
 WHERE formula = '-pre_tax';
"#;

const DOWN: &str = r#"
UPDATE product_journal_entry_item_templates
   SET formula = 'pre_tax', updated_at = NOW()
 WHERE formula = 'unit_rate';
UPDATE product_journal_entry_item_templates
   SET formula = '-pre_tax', updated_at = NOW()
 WHERE formula = '-unit_rate';
UPDATE product_preference_journal_items
   SET formula = 'pre_tax', updated_at = NOW()
 WHERE formula = 'unit_rate';
UPDATE product_preference_journal_items
   SET formula = '-pre_tax', updated_at = NOW()
 WHERE formula = '-unit_rate';
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        Ok(())
    }
}
