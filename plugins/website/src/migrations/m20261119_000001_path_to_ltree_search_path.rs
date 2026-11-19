use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !is_postgres(manager) {
            return Ok(());
        }
        // pg_dump restores with an empty search_path. An unqualified ltree cast
        // then fails while COPY fills the generated ltree_path column.
        exec_sql(
            manager,
            r#"
CREATE OR REPLACE FUNCTION path_to_ltree(p text) RETURNS public.ltree AS $$
DECLARE
    cleaned text;
BEGIN
    cleaned := trim(both '/' from p);
    IF cleaned = '' THEN
        RETURN 'root'::public.ltree;
    END IF;
    cleaned := replace(replace(cleaned, '/', '.'), '-', '_');
    cleaned := regexp_replace(cleaned, '[^a-zA-Z0-9_\.]', '', 'g');
    cleaned := regexp_replace(cleaned, '\.\.', '.', 'g');
    cleaned := trim(both '.' from cleaned);
    IF cleaned = '' THEN
        RETURN 'root'::public.ltree;
    END IF;
    RETURN cleaned::public.ltree;
EXCEPTION WHEN OTHERS THEN
    RETURN 'root'::public.ltree;
END;
$$ LANGUAGE plpgsql IMMUTABLE SET search_path = public;
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _ = manager;
        Ok(())
    }
}
