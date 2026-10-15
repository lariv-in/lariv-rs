//! HTTP handlers for todo CRUD.

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

use crate::{
    entities::todo::{self, Entity as TodoEntity},
    forms::TodoForm,
    keys::{TodoCreateModalKey, TodoEditModalKey, TodoTableKey},
    routes::{TodoDetailRouteTag, TodoListRouteTag},
    state::TodoState,
    templates::{
        ConfirmDeletePage, TodoCreateModalPage, TodoDetailPage, TodoEditModalPage, TodoListPage,
        TodoRow,
    },
};
use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    FilterField, apply_filters, bool_column, parse_list_filters, text_column,
};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    opt_or_log, respond_create_modal_done_fk, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;

/// Modal opener query (`?name=…&refresh=table-id`).
pub use lariv_core::web::ModalFormQuery as ModalNameQuery;

#[derive(Debug, Deserialize, Default)]
pub struct TodoListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn todo_filter_fields() -> Vec<FilterField> {
    vec![
        text_column("title", "Title", todo::Column::Title),
        bool_column("checked", "Checked", todo::Column::Checked),
    ]
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    lariv_core::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
}

fn browser_path_and_query(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    rest.find('/')
        .and_then(|index| rest.get(index..))
        .map(str::to_string)
        .unwrap_or_else(|| TodoListRouteTag.path())
}

fn list_query_from_browser(htmx: &Htmx) -> (String, TodoListQuery) {
    let path_and_query = htmx
        .current_url
        .as_deref()
        .map(browser_path_and_query)
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| TodoListRouteTag.path());
    let query = path_and_query
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or("");
    let parsed = UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default();
    (path_and_query, parsed)
}

async fn find_owned(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    user_id: i64,
) -> Option<todo::Model> {
    opt_or_log(
        TodoEntity::find_by_id(id)
            .filter(todo::Column::UserId.eq(user_id))
            .one(db)
            .await,
        "find todo",
    )
}

async fn load_todos_page(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    q: &TodoListQuery,
    condition: sea_orm::Condition,
    tz: &str,
) -> ObjectList<TodoRow> {
    let query = TodoEntity::find()
        .filter(todo::Column::UserId.eq(user_id))
        .filter(condition);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Title DESC") => query.order_by_desc(todo::Column::Title),
        s if s.eq_ignore_ascii_case("Title ASC") || s.eq_ignore_ascii_case("Title") => {
            query.order_by_asc(todo::Column::Title)
        }
        s if s.eq_ignore_ascii_case("Checked DESC") => query.order_by_desc(todo::Column::Checked),
        s if s.eq_ignore_ascii_case("Checked ASC") || s.eq_ignore_ascii_case("Checked") => {
            query.order_by_asc(todo::Column::Checked)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt DESC") => {
            query.order_by_desc(todo::Column::UpdatedAt)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt ASC") || s.eq_ignore_ascii_case("UpdatedAt") => {
            query.order_by_asc(todo::Column::UpdatedAt)
        }
        _ => query.order_by_desc(todo::Column::Id),
    };

    let page = q.page.get();
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|item| TodoRow {
            id: item.id,
            title: item.title,
            checked: item.checked,
            updated_at: format_updated_at(item.updated_at, tz),
        })
        .collect();
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

async fn build_list_page(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    q: &TodoListQuery,
    query: &str,
    path_and_query: String,
    tz: &str,
) -> TodoListPage {
    let fields = todo_filter_fields();
    let clauses = parse_list_filters(query);
    let applied = apply_filters(db.get_database_backend(), &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    let todos = load_todos_page(db, user_id, q, applied.condition, tz).await;
    TodoListPage {
        todos,
        fields,
        filters,
        filter_errors: applied.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query,
        page_size: q.page_size.get(),
    }
}

fn title_or_error(title: &str) -> Result<String, String> {
    let title = title.trim();
    if title.is_empty() {
        Err("Title is required.".into())
    } else {
        Ok(title.to_string())
    }
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TodoListQuery>,
) -> maud::Markup {
    let page = build_list_page(
        &state.db,
        ctx.user.id,
        &q,
        uri.query().unwrap_or(""),
        path_and_query(&uri),
        &ctx.timezone,
    )
    .await;
    if htmx.targets::<TodoTableKey>() {
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

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = TodoCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: String::new(),
        description: String::new(),
        checked: false,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TodoForm>,
) -> Response {
    let title = match title_or_error(&form.title) {
        Ok(title) => title,
        Err(error) => {
            let page = TodoCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                title: form.title,
                description: form.description,
                checked: form.checked,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let now = Utc::now();
    let model = todo::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(ctx.user.id),
        checked: Set(form.checked),
        title: Set(title),
        description: Set(form.description.trim().to_string()),
    };
    match model.insert(&state.db).await {
        Ok(item) => respond_create_modal_done_fk::<TodoCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &TodoDetailRouteTag::new(item.id).url(),
            item.id,
            &item.title,
            "",
        ),
        Err(e) => {
            tracing::error!(error = %e, "failed to create todo");
            let page = TodoCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                title: form.title,
                description: form.description,
                checked: form.checked,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(item) = find_owned(&state.db, id, ctx.user.id).await else {
        return Redirect::to(&TodoListRouteTag.url()).into_response();
    };
    let page = TodoDetailPage {
        id: item.id,
        title: item.title,
        description: item.description,
        checked: item.checked,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(item) = find_owned(&state.db, id, ctx.user.id).await else {
        return Redirect::to(&TodoListRouteTag.url()).into_response();
    };
    let page = TodoEditModalPage {
        id: item.id,
        form_name: q.form_name(),
        title: item.title,
        description: item.description,
        checked: item.checked,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TodoForm>,
) -> Response {
    let Some(item) = find_owned(&state.db, id, ctx.user.id).await else {
        return Redirect::to(&TodoListRouteTag.url()).into_response();
    };
    let title = match title_or_error(&form.title) {
        Ok(title) => title,
        Err(error) => {
            let page = TodoEditModalPage {
                id,
                form_name: q.form_name(),
                title: form.title,
                description: form.description,
                checked: form.checked,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let mut am: todo::ActiveModel = item.into();
    am.title = Set(title);
    am.description = Set(form.description.trim().to_string());
    am.checked = Set(form.checked);
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => {
            respond_edit_modal_done::<TodoEditModalKey>(&htmx, &TodoDetailRouteTag::new(id).url())
        }
        Err(e) => {
            tracing::error!(error = %e, id, "failed to update todo");
            let page = TodoEditModalPage {
                id,
                form_name: q.form_name(),
                title: form.title,
                description: form.description,
                checked: form.checked,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `delete_get`.
pub async fn delete_get(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let error = if find_owned(&state.db, id, ctx.user.id).await.is_none() {
        "To-do not found.".into()
    } else {
        String::new()
    };
    let page = ConfirmDeletePage {
        id,
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_todo.TodoDeleteForm".into()),
        error,
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<TodoState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if find_owned(&state.db, id, ctx.user.id).await.is_none() {
        return Redirect::to(&TodoListRouteTag.url()).into_response();
    }
    match TodoEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&TodoListRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete todo");
            let page = ConfirmDeletePage {
                id,
                form_name: "p_todo.TodoDeleteForm".into(),
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `toggle`.
pub async fn toggle(
    Cap(state): Cap<TodoState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(item) = find_owned(&state.db, id, ctx.user.id).await else {
        return Redirect::to(&TodoListRouteTag.url()).into_response();
    };
    let checked = !item.checked;
    let mut am: todo::ActiveModel = item.into();
    am.checked = Set(checked);
    am.updated_at = Set(Some(Utc::now()));
    if let Err(e) = am.update(&state.db).await {
        tracing::error!(error = %e, id, "failed to toggle todo");
        return Redirect::to(&TodoListRouteTag.url()).into_response();
    }
    if htmx.targets::<TodoTableKey>() {
        let (path_and_query, q) = list_query_from_browser(&htmx);
        let query = path_and_query
            .split_once('?')
            .map(|(_, query)| query.to_string())
            .unwrap_or_default();
        let page = build_list_page(
            &state.db,
            ctx.user.id,
            &q,
            &query,
            path_and_query,
            &ctx.timezone,
        )
        .await;
        return page.render_table().into_response();
    }
    Redirect::to(&TodoListRouteTag.url()).into_response()
}
