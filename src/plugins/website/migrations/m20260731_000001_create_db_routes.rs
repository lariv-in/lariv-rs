use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();

        if matches!(backend, sea_orm::DatabaseBackend::Postgres) {
            db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS ltree")
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(DbRoutes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DbRoutes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DbRoutes::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DbRoutes::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(DbRoutes::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(DbRoutes::Path)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(DbRoutes::PageId).big_integer().not_null())
                    .col(
                        ColumnDef::new(DbRoutes::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(DbRoutes::Theme)
                            .string_len(128)
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(DbRoutes::GrapesProject).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_db_routes_page_id")
                            .from(DbRoutes::Table, DbRoutes::PageId)
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_db_routes_deleted_at")
                    .table(DbRoutes::Table)
                    .col(DbRoutes::DeletedAt)
                    .to_owned(),
            )
            .await?;

        if matches!(backend, sea_orm::DatabaseBackend::Postgres) {
            db.execute_unprepared(
                r#"
CREATE OR REPLACE FUNCTION path_to_ltree(p text) RETURNS ltree AS $$
DECLARE
    cleaned text;
BEGIN
    cleaned := trim(both '/' from p);
    IF cleaned = '' THEN
        RETURN 'root'::ltree;
    END IF;
    cleaned := replace(replace(cleaned, '/', '.'), '-', '_');
    cleaned := regexp_replace(cleaned, '[^a-zA-Z0-9_\.]', '', 'g');
    cleaned := regexp_replace(cleaned, '\.\.', '.', 'g');
    cleaned := trim(both '.' from cleaned);
    IF cleaned = '' THEN
        RETURN 'root'::ltree;
    END IF;
    RETURN cleaned::ltree;
EXCEPTION WHEN OTHERS THEN
    RETURN 'root'::ltree;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
"#,
            )
            .await?;
            db.execute_unprepared(
                "ALTER TABLE db_routes ADD COLUMN IF NOT EXISTS ltree_path ltree GENERATED ALWAYS AS (path_to_ltree(path)) STORED",
            )
            .await?;
            db.execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_db_routes_ltree_path ON db_routes USING gist (ltree_path)",
            )
            .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(PWebsiteRouteReferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PWebsiteRouteReferences::DbRouteId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PWebsiteRouteReferences::VNodeId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(PWebsiteRouteReferences::DbRouteId)
                            .col(PWebsiteRouteReferences::VNodeId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_p_website_route_references_db_route_id")
                            .from(
                                PWebsiteRouteReferences::Table,
                                PWebsiteRouteReferences::DbRouteId,
                            )
                            .to(DbRoutes::Table, DbRoutes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_p_website_route_references_v_node_id")
                            .from(
                                PWebsiteRouteReferences::Table,
                                PWebsiteRouteReferences::VNodeId,
                            )
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(PWebsiteRouteReferences::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(DbRoutes::Table).to_owned())
            .await?;
        if matches!(
            manager.get_database_backend(),
            sea_orm::DatabaseBackend::Postgres
        ) {
            manager
                .get_connection()
                .execute_unprepared("DROP FUNCTION IF EXISTS path_to_ltree(text)")
                .await?;
        }
        Ok(())
    }
}

#[derive(Iden)]
enum DbRoutes {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Path,
    PageId,
    IsActive,
    Theme,
    GrapesProject,
}

#[derive(Iden)]
enum PWebsiteRouteReferences {
    Table,
    DbRouteId,
    VNodeId,
}

#[derive(Iden)]
enum FilesystemNodes {
    Table,
    Id,
}
