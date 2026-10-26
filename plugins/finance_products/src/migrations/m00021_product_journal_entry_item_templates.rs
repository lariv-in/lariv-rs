//! Journal template lines point at the product. The header table goes away.

use lariv_core::db::migration_sql::{
    self, FK_PRODUCT_JOURNAL_ENTRY_TEMPLATES_JOURNAL, FK_PRODUCT_JOURNAL_ITEM_JOURNAL,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                ADD COLUMN IF NOT EXISTS product_id bigint NULL,
                ADD COLUMN IF NOT EXISTS journal_id bigint NULL"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"UPDATE product_journal_entry_item_templates AS item
                SET product_id = template.product_id,
                    journal_id = template.journal_id
                FROM product_journal_entry_templates AS template
                WHERE item.product_journal_entry_template_id = template.id
                  AND item.product_id IS NULL"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                DROP CONSTRAINT IF EXISTS product_journal_entry_item_templates_product_journal_entry_template_id_fkey"#,
        )
        .await?;
        conn.execute_unprepared(
            "DROP INDEX IF EXISTS idx_product_journal_entry_item_templates_template_id",
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                DROP COLUMN IF EXISTS product_journal_entry_template_id"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"DELETE FROM product_journal_entry_item_templates
                WHERE product_id IS NULL OR journal_id IS NULL"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                ALTER COLUMN product_id SET NOT NULL,
                ALTER COLUMN journal_id SET NOT NULL"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                DROP CONSTRAINT IF EXISTS fk_product_journal_entry_item_templates_product_id,
                DROP CONSTRAINT IF EXISTS fk_product_journal_entry_item_templates_journal_id"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                ADD CONSTRAINT fk_product_journal_entry_item_templates_product_id
                    FOREIGN KEY (product_id) REFERENCES products(id)
                    ON UPDATE CASCADE ON DELETE CASCADE"#,
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(manager, &[FK_PRODUCT_JOURNAL_ITEM_JOURNAL])
            .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_product_journal_entry_item_templates_product_id ON product_journal_entry_item_templates (product_id)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_product_journal_entry_item_templates_journal_id ON product_journal_entry_item_templates (journal_id)",
        )
        .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS product_journal_entry_templates")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS product_journal_entry_templates (
                id bigserial PRIMARY KEY,
                created_at timestamptz NULL,
                updated_at timestamptz NULL,
                product_id bigint NOT NULL UNIQUE
                    REFERENCES products(id) ON UPDATE CASCADE ON DELETE CASCADE,
                journal_id bigint NOT NULL
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"INSERT INTO product_journal_entry_templates (product_id, journal_id, created_at, updated_at)
                SELECT product_id, MIN(journal_id), NOW(), NOW()
                FROM product_journal_entry_item_templates
                GROUP BY product_id
                ON CONFLICT (product_id) DO NOTHING"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                ADD COLUMN IF NOT EXISTS product_journal_entry_template_id bigint NULL"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"UPDATE product_journal_entry_item_templates AS item
                SET product_journal_entry_template_id = template.id
                FROM product_journal_entry_templates AS template
                WHERE template.product_id = item.product_id"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                DROP CONSTRAINT IF EXISTS fk_product_journal_entry_item_templates_product_id,
                DROP CONSTRAINT IF EXISTS fk_product_journal_entry_item_templates_journal_id"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                DROP COLUMN IF EXISTS product_id,
                DROP COLUMN IF EXISTS journal_id"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                ALTER COLUMN product_journal_entry_template_id SET NOT NULL"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE product_journal_entry_item_templates
                ADD CONSTRAINT product_journal_entry_item_templates_product_journal_entry_template_id_fkey
                    FOREIGN KEY (product_journal_entry_template_id)
                    REFERENCES product_journal_entry_templates(id)
                    ON UPDATE CASCADE ON DELETE CASCADE"#,
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[FK_PRODUCT_JOURNAL_ENTRY_TEMPLATES_JOURNAL],
        )
        .await?;
        Ok(())
    }
}
