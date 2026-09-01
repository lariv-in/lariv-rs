use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use crate::picker::respond_picker_select;
use crate::template::RenderAppPane;
use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        blog::{
            entities::{
                blog::Entity as BlogEntity,
                blog_tag::{self, Entity as BlogTagEntity},
            },
            forms::TagForm,
            keys::{
                TagCreateModalKey, TagDeleteModalKey, TagEditModalKey, TagSelectModalKey,
                TagSelectTableKey, TagTableKey,
            },
            routes::BlogTagsDetailRouteTag,
            state::BlogState,
            templates::{
                ConfirmDeletePage, TagCreateModalPage, TagDetailPage, TagEditModalPage,
                TagListPage, TagOption, TagRow, TagSelectPage,
            },
        },
        users::middleware::RequireAuth,
    },
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done_fk, respond_edit_modal_done,
    },
};

use super::ModalNameQuery;

#[derive(Debug, Deserialize, Default)]
pub struct TagListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    crate::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
}

async fn query_tags(
    db: &sea_orm::DatabaseConnection,
    q: &TagListQuery,
) -> (Vec<blog_tag::Model>, u32, u64) {
    let mut query = BlogTagEntity::find();
    let name = q.name.clone().unwrap_or_default();
    if !name.is_empty() {
        query = query.filter(blog_tag::Column::Name.contains(&name));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(blog_tag::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(blog_tag::Column::Name)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt DESC") => {
            query.order_by_desc(blog_tag::Column::UpdatedAt)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt ASC") || s.eq_ignore_ascii_case("UpdatedAt") => {
            query.order_by_asc(blog_tag::Column::UpdatedAt)
        }
        _ => query.order_by_desc(blog_tag::Column::Id),
    };

    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

async fn load_tags_page(
    db: &sea_orm::DatabaseConnection,
    q: &TagListQuery,
    tz: &str,
) -> ObjectList<TagRow> {
    let (models, page, total) = query_tags(db, q).await;
    let rows = models
        .into_iter()
        .map(|t| TagRow {
            id: t.id,
            name: t.name,
            updated_at: format_updated_at(t.updated_at, tz),
        })
        .collect();
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

async fn load_tag_options_page(
    db: &sea_orm::DatabaseConnection,
    q: &TagListQuery,
) -> ObjectList<TagOption> {
    let (models, page, total) = query_tags(db, q).await;
    let rows = models
        .into_iter()
        .map(|t| TagOption {
            id: t.id,
            name: t.name,
        })
        .collect();
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

/// Articles currently linked to `tag_id`.
async fn load_blogs_for_tag(db: &sea_orm::DatabaseConnection, tag_id: i64) -> Vec<(i64, String)> {
    let result = BlogTagEntity::find_by_id(tag_id)
        .find_with_related(BlogEntity)
        .all(db)
        .await
        .unwrap_or_default();
    result
        .into_iter()
        .flat_map(|(_, blogs)| blogs)
        .map(|b| (b.id, b.title))
        .collect()
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TagListQuery>,
) -> maud::Markup {
    let tags = load_tags_page(&state.db, &q, &ctx.timezone).await;
    let page = TagListPage {
        tags,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    if htmx.targets::<TagTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `select`.
pub async fn select(
    Cap(state): Cap<BlogState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TagListQuery>,
) -> maud::Markup {
    let tags = load_tag_options_page(&state.db, &q).await;
    let page = TagSelectPage {
        tags,
        filter_name: q.name.clone().unwrap_or_default(),
        target_input: q.target_input.clone().unwrap_or_else(|| "Tags".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<TagSelectTableKey, TagSelectModalKey, _>(&htmx, &page)
}

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(tag) = crate::web::opt_or_log(
        BlogTagEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/blog/tags/").into_response();
    };
    let blogs = load_blogs_for_tag(&state.db, id).await;
    let page = TagDetailPage {
        id: tag.id,
        name: tag.name,
        blogs,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = TagCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TagForm>,
) -> Response {
    let now = Utc::now();
    let model = blog_tag::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.clone()),
    };
    match model.insert(&state.db).await {
        Ok(tag) => respond_create_modal_done_fk::<TagCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &BlogTagsDetailRouteTag::new(tag.id).url(),
            tag.id,
            &tag.name,
            &q.target_input(),
        ),
        Err(e) => {
            let page = TagCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(tag) = crate::web::opt_or_log(
        BlogTagEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/blog/tags/").into_response();
    };
    let page = TagEditModalPage {
        id: tag.id,
        form_name: q.form_name(),
        name: tag.name,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TagForm>,
) -> Response {
    let Some(tag) = crate::web::opt_or_log(
        BlogTagEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/blog/tags/").into_response();
    };
    let mut am: blog_tag::ActiveModel = tag.into();
    am.name = Set(form.name.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<TagEditModalKey>(
            &htmx,
            &BlogTagsDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = TagEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `delete_get`.
pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: TagDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this tag?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_blog.TagDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<BlogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match BlogTagEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect("/blog/tags/"),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete blog tag");
            let page = ConfirmDeletePage {
                modal_uid: TagDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this tag?".into(),
                form_name: "p_blog.TagDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
