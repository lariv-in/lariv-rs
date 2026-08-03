use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};
use serde::Deserialize;

use crate::{
    components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    http::{Cap},
    plugins::{
        blog::{
            entities::{
                blog::{self, Entity as BlogEntity},
                blog_tag::Entity as BlogTagEntity,
                blog_tag_link,
            },
            forms::BlogForm,
            keys::{BlogDeleteModalKey, BlogTableKey},
            routes::BlogDetailRouteTag,
            slug::resolve_blog_slug,
            state::BlogState,
            templates::{
                BlogDetailPage, BlogFormPage, BlogListPage, BlogRow, ConfirmDeletePage,
            },
        },
        users::{entities::user::Entity as UserEntity, middleware::RequireAuth},
    },
    web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots},
};
use crate::template::RenderAppPane;

use super::ModalNameQuery;

const PAGE_SIZE: u32 = 12;

#[derive(Debug, Deserialize, Default)]
pub struct BlogListQuery {
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>) -> String {
    dt.map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

async fn author_display(db: &sea_orm::DatabaseConnection, user_id: i64) -> String {
    UserEntity::find_by_id(user_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|u| u.name)
        .unwrap_or_default()
}

async fn query_blogs(
    db: &sea_orm::DatabaseConnection,
    q: &BlogListQuery,
) -> (Vec<blog::Model>, u32, u64) {
    let mut query = BlogEntity::find().filter(blog::Column::DeletedAt.is_null());
    let title = q.title.clone().unwrap_or_default();
    if !title.is_empty() {
        query = query.filter(blog::Column::Title.contains(&title));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Title DESC") => query.order_by_desc(blog::Column::Title),
        s if s.eq_ignore_ascii_case("Title ASC") || s.eq_ignore_ascii_case("Title") => {
            query.order_by_asc(blog::Column::Title)
        }
        _ => query.order_by_desc(blog::Column::Id),
    };

    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

async fn load_blogs_page(
    db: &sea_orm::DatabaseConnection,
    q: &BlogListQuery,
) -> ObjectList<BlogRow> {
    let (models, page, total) = query_blogs(db, q).await;
    let mut rows = Vec::with_capacity(models.len());
    for b in models {
        let author_name = author_display(db, b.created_by_id).await;
        rows.push(BlogRow {
            id: b.id,
            title: b.title,
            slug: b.slug,
            author_name,
            updated_at: format_updated_at(b.updated_at),
        });
    }
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

/// Tags currently linked to `blog_id`, as `(id, name)` pairs.
async fn load_tags_for_blog(db: &sea_orm::DatabaseConnection, blog_id: i64) -> Vec<(i64, String)> {
    let result = BlogEntity::find_by_id(blog_id)
        .find_with_related(BlogTagEntity)
        .all(db)
        .await
        .unwrap_or_default();
    result
        .into_iter()
        .flat_map(|(_, tags)| tags)
        .filter(|t| t.deleted_at.is_none())
        .map(|t| (t.id, t.name))
        .collect()
}

/// Tags currently linked to `blog_id`, as [`ManyToManyItem`]s for form pre-fill.
async fn load_tag_items_for_blog(
    db: &sea_orm::DatabaseConnection,
    blog_id: i64,
) -> Vec<ManyToManyItem> {
    load_tags_for_blog(db, blog_id)
        .await
        .into_iter()
        .map(|(id, name)| ManyToManyItem {
            key: id.to_string(),
            value: name,
        })
        .collect()
}

/// Resolve submitted tag ids to [`ManyToManyItem`]s for re-rendering a form on error.
async fn tag_items_from_ids(db: &sea_orm::DatabaseConnection, ids: &[i64]) -> Vec<ManyToManyItem> {
    if ids.is_empty() {
        return Vec::new();
    }
    let tags = BlogTagEntity::find()
        .filter(crate::plugins::blog::entities::blog_tag::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    ids.iter()
        .filter_map(|id| {
            tags.iter().find(|t| t.id == *id).map(|t| ManyToManyItem {
                key: t.id.to_string(),
                value: t.name.clone(),
            })
        })
        .collect()
}

/// Replace all `p_blog_tags` links for `blog_id` with `tag_ids`.
async fn sync_blog_tags(
    db: &sea_orm::DatabaseConnection,
    blog_id: i64,
    tag_ids: &[i64],
) -> Result<(), DbErr> {
    blog_tag_link::Entity::delete_many()
        .filter(blog_tag_link::Column::BlogId.eq(blog_id))
        .exec(db)
        .await?;
    for &tag_id in tag_ids {
        blog_tag_link::ActiveModel {
            blog_id: Set(blog_id),
            blog_tag_id: Set(tag_id),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<BlogListQuery>,
) -> maud::Markup
{
    let blogs = load_blogs_page(&state.db, &q).await;
    let page = BlogListPage {
        blogs,
        filter_title: q.title.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<BlogTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main();
    }
    if htmx.wants_app_layout() {
        return page.render_pane();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    let Some(blog) = BlogEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .filter(|b| b.deleted_at.is_none())
    else {
        return Redirect::to("/blog/").into_response();
    };
    let author_name = author_display(&state.db, blog.created_by_id).await;
    let tags = load_tags_for_blog(&state.db, id).await;
    let page = BlogDetailPage {
        id: blog.id,
        title: blog.title,
        slug: blog.slug,
        description: blog.description,
        author_name,
        tags,
        content: blog.content,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response
{
    let page = BlogFormPage {
        id: 0,
        title: String::new(),
        slug: String::new(),
        description: String::new(),
        created_by_id: ctx.user.id,
        author_display: ctx.user.name.clone(),
        tags: Vec::new(),
        content: String::new(),
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}


/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Form(form): Form<BlogForm>,
) -> Response
{
    let created_by_id = if form.created_by_id == 0 {
        ctx.user.id
    } else {
        form.created_by_id
    };
    let slug = resolve_blog_slug(&form.title, &form.slug);
    let now = Utc::now();
    let model = blog::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
        title: Set(form.title.clone()),
        slug: Set(slug.clone()),
        description: Set(form.description.clone()),
        created_by_id: Set(created_by_id),
        content: Set(form.content.clone()),
    };
    match model.insert(&state.db).await {
        Ok(saved) => {
            let _ = sync_blog_tags(&state.db, saved.id, &form.tags).await;
            htmx.redirect(&BlogDetailRouteTag::new(saved.id).url())
        }
        Err(e) => {
            let author_display = author_display(&state.db, created_by_id).await;
            let tag_items = tag_items_from_ids(&state.db, &form.tags).await;
            let page = BlogFormPage {
                id: 0,
                title: form.title,
                slug,
                description: form.description,
                created_by_id,
                author_display,
                tags: tag_items,
                content: form.content,
                error: e.to_string(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
    }
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    let Some(blog) = BlogEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/blog/").into_response();
    };
    let author_display = author_display(&state.db, blog.created_by_id).await;
    let tags = load_tag_items_for_blog(&state.db, id).await;
    let page = BlogFormPage {
        id: blog.id,
        title: blog.title,
        slug: blog.slug,
        description: blog.description,
        created_by_id: blog.created_by_id,
        author_display,
        tags,
        content: blog.content,
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<BlogForm>,
) -> Response
{
    let Some(blog) = BlogEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/blog/").into_response();
    };
    let created_by_id = if form.created_by_id == 0 {
        blog.created_by_id
    } else {
        form.created_by_id
    };
    let slug = resolve_blog_slug(&form.title, &form.slug);
    let mut am: blog::ActiveModel = blog.into();
    am.title = Set(form.title.clone());
    am.slug = Set(slug.clone());
    am.description = Set(form.description.clone());
    am.created_by_id = Set(created_by_id);
    am.content = Set(form.content.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => {
            let _ = sync_blog_tags(&state.db, id, &form.tags).await;
            htmx.redirect(&BlogDetailRouteTag::new(id).url())
        }
        Err(e) => {
            let author_display = author_display(&state.db, created_by_id).await;
            let tag_items = tag_items_from_ids(&state.db, &form.tags).await;
            let page = BlogFormPage {
                id,
                title: form.title,
                slug,
                description: form.description,
                created_by_id,
                author_display,
                tags: tag_items,
                content: form.content,
                error: e.to_string(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
    }
}

/// HTTP handler: `delete_get`.
pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup
{
    let page = ConfirmDeletePage {
        modal_uid: BlogDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this article?".into(),
        form_name: q.name
            .clone()
            .unwrap_or_else(|| "p_blog.BlogDeleteForm".into()),
        id,
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<BlogState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if let Some(blog) = BlogEntity::find_by_id(id).one(&state.db).await.ok().flatten() {
        let mut am: blog::ActiveModel = blog.into();
        am.deleted_at = Set(Some(Utc::now()));
        let _ = am.update(&state.db).await;
    }
    htmx.redirect("/blog/")
}
