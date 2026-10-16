//! HTTP handlers for the to-do page list and one page's checklist.

use axum::extract::{Path, Query};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};
use serde::Deserialize;

use crate::entities::todo::{self, Entity as TodoEntity};
use crate::entities::todo_page::{self, Entity as TodoPageEntity};
use crate::forms::TodoPageForm;
use crate::keys::{TodoPageCreateModalKey, TodoPageEditModalKey, TodoPageTableKey};
use crate::routes::{TodoListRouteTag, TodoPageDetailRouteTag};
use crate::state::TodoState;
use crate::sync;
use crate::templates::{
    TodoListPage, TodoPageCreateModalPage, TodoPageDeleteModalPage, TodoPageEditModalPage,
    TodoPageListPage, TodoPageRow,
};
use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, ModalFormQuery, QueryPage, QueryPageSize, html_built_page_or_app_layout,
    html_built_page_with_slots, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;

#[derive(Debug, Deserialize, Default)]
pub struct PageListQuery {
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

/// `GET /dashboard/todo` — table of pages this user created.
pub async fn list(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<PageListQuery>,
) -> maud::Markup {
    let (rows, number, total) = match load_pages(&state.db, ctx.user.id, &q).await {
        Ok(value) => value,
        Err(err) => {
            tracing::error!(error = %err, user_id = ctx.user.id, "todo: list pages");
            (Vec::new(), q.page.get(), 0)
        }
    };
    let page = TodoPageListPage {
        pages: ObjectList::from_page(rows, number, q.page_size.get(), total),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<TodoPageTableKey>() {
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

/// `GET /dashboard/todo/create` — title form for a new page.
pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = TodoPageCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// `POST /dashboard/todo/create` — store the submitted title and open the page.
pub async fn create_post(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<TodoPageForm>,
) -> Response {
    let title = form.title.trim().to_string();
    if title.is_empty() {
        let page = TodoPageCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            title: form.title,
            error: "Title is required.".to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let model = todo_page::ActiveModel {
        title: Set(title),
        created_by_id: Set(ctx.user.id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    match model.insert(&state.db).await {
        Ok(page) => respond_create_modal_done::<TodoPageCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &TodoPageDetailRouteTag::new(page.id).url(),
        ),
        Err(err) => {
            tracing::error!(error = %err, user_id = ctx.user.id, "todo: create page");
            let page = TodoPageCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                title: form.title,
                error: "Could not create a page.".to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// `GET /dashboard/todo/{id}/edit` — title form for a page this user created.
pub async fn edit_get(
    Path(id): Path<i64>,
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    let Some(page) = lariv_core::web::opt_or_log(
        sync::find_visible_page(&state.db, ctx.user.id, id).await,
        "todo: find page",
    ) else {
        return htmx.redirect(&TodoListRouteTag.url());
    };
    let view = TodoPageEditModalPage {
        id: page.id,
        form_name: q.form_name(),
        title: page.title,
        error: String::new(),
    };
    html_built_page_with_slots(&view, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// `POST /dashboard/todo/{id}/edit` — rename a page this user created.
pub async fn edit_post(
    Path(id): Path<i64>,
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<TodoPageForm>,
) -> Response {
    let Some(existing) = lariv_core::web::opt_or_log(
        sync::find_visible_page(&state.db, ctx.user.id, id).await,
        "todo: find page",
    ) else {
        return htmx.redirect(&TodoListRouteTag.url());
    };
    let title = form.title.trim().to_string();
    if title.is_empty() {
        let view = TodoPageEditModalPage {
            id,
            form_name: q.form_name(),
            title: form.title,
            error: "Title is required.".to_string(),
        };
        return html_built_page_with_slots(&view, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let mut model: todo_page::ActiveModel = existing.into();
    model.title = Set(title);
    model.updated_at = Set(Some(Utc::now()));
    match model.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<TodoPageEditModalKey>(
            &htmx,
            &TodoPageDetailRouteTag::new(id).url(),
        ),
        Err(err) => {
            tracing::error!(error = %err, id, "todo: edit page");
            let view = TodoPageEditModalPage {
                id,
                form_name: q.form_name(),
                title: form.title,
                error: "Could not save the page.".to_string(),
            };
            html_built_page_with_slots(&view, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// `GET /dashboard/todo/{id}/delete` — confirm deleting a page and its items.
pub async fn delete_get(
    Path(id): Path<i64>,
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    if lariv_core::web::opt_or_log(
        sync::find_visible_page(&state.db, ctx.user.id, id).await,
        "todo: find page",
    )
    .is_none()
    {
        return htmx.redirect(&TodoListRouteTag.url());
    }
    let page = TodoPageDeleteModalPage { id };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// `POST /dashboard/todo/{id}/delete` — delete a page this user created.
pub async fn delete_post(
    Path(id): Path<i64>,
    Cap(state): Cap<TodoState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    let Some(existing) = lariv_core::web::opt_or_log(
        sync::find_visible_page(&state.db, ctx.user.id, id).await,
        "todo: find page",
    ) else {
        return htmx.redirect(&TodoListRouteTag.url());
    };
    if let Err(err) = TodoEntity::delete_many()
        .filter(todo::Column::TodoPageId.eq(existing.id))
        .exec(&state.db)
        .await
    {
        tracing::error!(error = %err, id, "todo: delete page items");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not delete the page.",
        )
            .into_response();
    }
    match existing.delete(&state.db).await {
        Ok(_) => htmx.redirect(&TodoListRouteTag.url()),
        Err(err) => {
            tracing::error!(error = %err, id, "todo: delete page");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not delete the page.",
            )
                .into_response()
        }
    }
}

/// `GET /dashboard/todo/{id}` — checklist for one page this user created.
pub async fn detail(
    Path(id): Path<i64>,
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    let Some(page) = lariv_core::web::opt_or_log(
        sync::find_visible_page(&state.db, ctx.user.id, id).await,
        "todo: find page",
    ) else {
        return htmx.redirect(&TodoListRouteTag.url());
    };
    let view = TodoListPage {
        user_id: ctx.user.id,
        page_id: page.id,
        title: page.title,
        path_and_query: path_and_query(&uri),
    };
    html_built_page_or_app_layout(&view, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn load_pages(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    q: &PageListQuery,
) -> Result<(Vec<TodoPageRow>, u32, u64), sea_orm::DbErr> {
    let mut query = TodoPageEntity::find().filter(todo_page::Column::CreatedById.eq(user_id));
    query = match q.sort.as_deref().map(str::trim) {
        Some("Title DESC") => query
            .order_by_desc(todo_page::Column::Title)
            .order_by_desc(todo_page::Column::Id),
        Some("Title ASC") => query
            .order_by_asc(todo_page::Column::Title)
            .order_by_asc(todo_page::Column::Id),
        _ => query.order_by_asc(todo_page::Column::Id),
    };
    let number = q.page.get();
    let paginator = query.paginate(db, u64::from(q.page_size.get()));
    let total = paginator.num_items().await?;
    let models = paginator
        .fetch_page(u64::from(number).saturating_sub(1))
        .await?;
    let rows = models
        .into_iter()
        .map(|page| TodoPageRow {
            id: page.id,
            title: page.title,
        })
        .collect();
    Ok((rows, number, total))
}
