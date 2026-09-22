use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::tasks::{
    color::{hex_to_u24, random_status_color, u24_to_hex},
    entities::{
        task::{self, Entity as TaskEntity},
        task_status::{self, Entity as TaskStatusEntity},
    },
    forms::TaskStatusForm,
    handlers::ModalNameQuery,
    keys::{
        TaskStatusCreateModalKey, TaskStatusDeleteModalKey, TaskStatusEditModalKey,
        TaskStatusTableKey, TaskStatusTasksTableKey,
    },
    logic::status::delete_status,
    routes::{TaskDetailRouteTag, TaskStatusDefaultRouteTag, TaskStatusDetailRouteTag},
    scope::{apply_status_sort, apply_task_sort, find_status_scoped, scope_superuser},
    state::TasksState,
    templates::{
        ConfirmDeletePage, StatusTaskRow, TaskStatusCreateModalPage, TaskStatusDetailPage,
        TaskStatusEditModalPage, TaskStatusListPage, TaskStatusRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct StatusListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct StatusDetailQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn statuses_list_url() -> String {
    TaskStatusDefaultRouteTag.url()
}

async fn query_statuses(
    db: &sea_orm::DatabaseConnection,
    q: &StatusListQuery,
    auth: &AuthContext,
) -> (Vec<task_status::Model>, u32, u64) {
    let mut query = TaskStatusEntity::find();
    let name = q.name.clone().unwrap_or_default();
    if !name.is_empty() {
        query = query.filter(task_status::Column::Name.contains(&name));
    }
    query = scope_superuser(query, auth);
    query = apply_status_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

pub async fn list(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<StatusListQuery>,
) -> maud::Markup {
    let (models, page, total) = query_statuses(&state.db, &q, &ctx).await;
    let rows = models
        .into_iter()
        .map(|s| TaskStatusRow {
            id: s.id,
            name: s.name,
            color: s.color,
        })
        .collect();
    let page = TaskStatusListPage {
        statuses: ObjectList::from_page(rows, page, q.page_size.get(), total),
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<TaskStatusTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = TaskStatusCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        color: u24_to_hex(random_status_color()),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TaskStatusForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&statuses_list_url()).into_response();
    }
    let name = form.name.trim().to_string();
    let color = hex_to_u24(&form.color);
    if name.is_empty() {
        let page = TaskStatusCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            name: form.name,
            color: u24_to_hex(color),
            error: "name is required".to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let model = task_status::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        color: Set(color),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<TaskStatusCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &TaskStatusDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = TaskStatusCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                name: form.name,
                color: u24_to_hex(color),
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<StatusDetailQuery>,
) -> Response {
    let Some(status) = find_status_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&statuses_list_url()).into_response();
    };
    let mut query = TaskEntity::find().filter(task::Column::StatusId.eq(id));
    query = scope_superuser(query, &ctx);
    query = apply_task_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(&state.db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows: Vec<StatusTaskRow> = models
        .into_iter()
        .map(|t| StatusTaskRow {
            id: t.id,
            title: t.title,
            due_datetime: ctx.format_datetime(t.due_datetime).into_string(),
            detail_href: TaskDetailRouteTag::new(t.id).url(),
        })
        .collect();
    let page = TaskStatusDetailPage {
        id: status.id,
        name: status.name,
        color: status.color,
        can_edit: ctx.user.is_superuser,
        tasks: ObjectList::from_page(rows, page, q.page_size.get(), total),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<TaskStatusTasksTableKey>() {
        return page.render_tasks_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&statuses_list_url()).into_response();
    }
    let Some(status) = find_status_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&statuses_list_url()).into_response();
    };
    let page = TaskStatusEditModalPage {
        id: status.id,
        form_name: q.form_name(),
        name: status.name,
        color: u24_to_hex(status.color),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TaskStatusForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&statuses_list_url()).into_response();
    }
    let Some(existing) = find_status_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&statuses_list_url()).into_response();
    };
    let name = form.name.trim().to_string();
    let color = hex_to_u24(&form.color);
    if name.is_empty() {
        let page = TaskStatusEditModalPage {
            id,
            form_name: q.form_name(),
            name: form.name,
            color: u24_to_hex(color),
            error: "name is required".to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let mut am: task_status::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    am.color = Set(color);
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<TaskStatusEditModalKey>(
            &htmx,
            &TaskStatusDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = TaskStatusEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                color: u24_to_hex(color),
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: TaskStatusDeleteModalKey::ID.to_string(),
        message:
            "Are you sure you want to delete this status? Tasks using it must be reassigned first."
                .into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_tasks.TaskStatusDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&statuses_list_url()).into_response();
    }
    match delete_status(&state.db, id, &ctx).await {
        Ok(()) => htmx.redirect(&statuses_list_url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete task status");
            let page = ConfirmDeletePage {
                modal_uid: TaskStatusDeleteModalKey::ID.to_string(),
                message:
                    "Are you sure you want to delete this status? Tasks using it must be reassigned first."
                        .into(),
                form_name: "p_tasks.TaskStatusDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
