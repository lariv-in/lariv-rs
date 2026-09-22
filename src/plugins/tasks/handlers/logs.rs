use axum::{
    extract::{Path, Query},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};

use crate::{
    components::{SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, respond_edit_modal_done,
    },
};

use crate::plugins::tasks::{
    entities::task_log::{self, Entity as TaskLogEntity},
    forms::{TaskLogForm, TaskLogQuickForm},
    handlers::ModalNameQuery,
    keys::{TASK_LOG_SAVED_EVENT, TaskLogDeleteModalKey, TaskLogEditModalKey},
    routes::TaskDetailRouteTag,
    scope::{find_log_scoped, find_task_scoped, scope_superuser},
    state::TasksState,
    templates::{
        ConfirmDeletePage, TaskLogDetailPage, TaskLogEditModalPage, TaskLogItem, TaskLogsPanel,
    },
};

fn task_url(task_id: i64) -> String {
    TaskDetailRouteTag::new(task_id).url()
}

pub(crate) async fn load_logs_panel(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    task_id: i64,
    can_edit: bool,
) -> TaskLogsPanel {
    let mut query = TaskLogEntity::find().filter(task_log::Column::TaskId.eq(task_id));
    query = scope_superuser(query, auth);
    let models = query
        .order_by_desc(task_log::Column::Datetime)
        .all(db)
        .await
        .unwrap_or_default();
    let items = models
        .into_iter()
        .map(|u| TaskLogItem {
            id: u.id,
            datetime: auth.format_datetime(u.datetime).into_string(),
            description: u.description,
        })
        .collect();
    TaskLogsPanel {
        task_id,
        items,
        can_edit,
        default_datetime: auth.datetime_local_input(Utc::now()).into_string(),
    }
}

pub async fn detail(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(log) = find_log_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/tasks").into_response();
    };
    let Some(task) = find_task_scoped(&state.db, log.task_id, &ctx).await else {
        return Redirect::to("/tasks").into_response();
    };
    let page = TaskLogDetailPage {
        id: log.id,
        task_id: log.task_id,
        task_title: task.title,
        datetime: ctx.format_datetime(log.datetime).into_string(),
        description: log.description,
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn add_post(
    Cap(state): Cap<TasksState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(task_id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<TaskLogQuickForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&task_url(task_id)).into_response();
    }
    if find_task_scoped(&state.db, task_id, &ctx).await.is_none() {
        return Redirect::to("/tasks").into_response();
    }
    let description = form.description.trim();
    if description.is_empty() {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    };
    let now = Utc::now();
    let model = task_log::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        task_id: Set(task_id),
        description: Set(description.to_string()),
        datetime: Set(datetime),
    };
    if model.insert(&state.db).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if !htmx.request {
        return Redirect::to(&task_url(task_id)).into_response();
    }
    let panel = load_logs_panel(&state.db, &ctx, task_id, true).await;
    let body = panel.render_list().into_string();
    let trigger = format!(r#"{{"{TASK_LOG_SAVED_EVENT}":true}}"#);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header("HX-Trigger", trigger)
        .body(body.into())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub async fn edit_get(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/tasks").into_response();
    }
    let Some(log) = find_log_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/tasks").into_response();
    };
    let page = TaskLogEditModalPage {
        id: log.id,
        form_name: q.form_name(),
        datetime: ctx.datetime_local_input(log.datetime).into_string(),
        description: log.description,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn edit_modal_error(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &TaskLogForm,
    error: &str,
) -> Response {
    let page = TaskLogEditModalPage {
        id,
        form_name: q.form_name(),
        datetime: form.datetime.clone(),
        description: form.description.clone(),
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TaskLogForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/tasks").into_response();
    }
    let Some(existing) = find_log_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/tasks").into_response();
    };
    if form.description.trim().is_empty() {
        return edit_modal_error(&chrome, &ctx, id, &q, &form, "description is required").await;
    }
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        return edit_modal_error(&chrome, &ctx, id, &q, &form, "invalid date & time").await;
    };
    let task_id = existing.task_id;
    let now = Utc::now();
    let mut am: task_log::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.datetime = Set(datetime);
    am.description = Set(form.description.trim().to_string());
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<TaskLogEditModalKey>(&htmx, &task_url(task_id)),
        Err(e) => edit_modal_error(&chrome, &ctx, id, &q, &form, &e.to_string()).await,
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: TaskLogDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this log?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_tasks.TaskLogDeleteForm".into()),
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
        return Redirect::to("/tasks").into_response();
    }
    let Some(log) = find_log_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/tasks").into_response();
    };
    let task_id = log.task_id;
    match TaskLogEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&task_url(task_id)),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete task log");
            let page = ConfirmDeletePage {
                modal_uid: TaskLogDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this log?".into(),
                form_name: "p_tasks.TaskLogDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
