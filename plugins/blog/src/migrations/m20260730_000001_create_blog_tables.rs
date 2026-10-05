use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BlogTags::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BlogTags::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BlogTags::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(BlogTags::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(BlogTags::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(BlogTags::Name).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_blog_tags_deleted_at")
                    .table(BlogTags::Table)
                    .col(BlogTags::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_blog_tags_name")
                    .table(BlogTags::Table)
                    .col(BlogTags::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Blogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Blogs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Blogs::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Blogs::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Blogs::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Blogs::Title).text().not_null())
                    .col(ColumnDef::new(Blogs::Slug).string_len(255).not_null())
                    .col(ColumnDef::new(Blogs::Description).text())
                    .col(ColumnDef::new(Blogs::CreatedById).big_integer().not_null())
                    .col(ColumnDef::new(Blogs::Content).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_blogs_created_by_id")
                            .from(Blogs::Table, Blogs::CreatedById)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_blogs_deleted_at")
                    .table(Blogs::Table)
                    .col(Blogs::DeletedAt)
                    .to_owned(),
            )
            .await?;

        // SQLite cannot express partial unique indexes via SeaORM builder; full unique
        // on slug (soft-deleted rows still occupy the slug, matching users plugin style).
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_blogs_slug")
                    .table(Blogs::Table)
                    .col(Blogs::Slug)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_blogs_created_by_id")
                    .table(Blogs::Table)
                    .col(Blogs::CreatedById)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PBlogTags::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PBlogTags::BlogId).big_integer().not_null())
                    .col(
                        ColumnDef::new(PBlogTags::BlogTagId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(PBlogTags::BlogId)
                            .col(PBlogTags::BlogTagId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_p_blog_tags_blog_id")
                            .from(PBlogTags::Table, PBlogTags::BlogId)
                            .to(Blogs::Table, Blogs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_p_blog_tags_blog_tag_id")
                            .from(PBlogTags::Table, PBlogTags::BlogTagId)
                            .to(BlogTags::Table, BlogTags::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PBlogTags::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Blogs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(BlogTags::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum BlogTags {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
}

#[derive(Iden)]
enum Blogs {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Title,
    Slug,
    Description,
    CreatedById,
    Content,
}

#[derive(Iden)]
enum PBlogTags {
    Table,
    BlogId,
    BlogTagId,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
