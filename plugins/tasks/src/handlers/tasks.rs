use axum::{
    extract::{Path, Query},
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};

use lariv_core::components::{
    ObjectList, SharedChromeFolder, SlotCtx, SwapKey, table_rows_instance_id,
};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::picker::respond_picker_select;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::{middleware::RequireAuth, roles::Superuser, state::AuthContext};

use crate::{
    color::{STATUS_TODO, status_name_from_slug},
    entities::task::{self, Entity as TaskEntity},
    forms::TaskForm,
    handlers::ModalNameQuery,
    keys::{
        TaskCreateModalKey, TaskDeleteModalKey, TaskEditModalKey, TaskSelectModalKey,
        TaskSelectTableKey, TaskTableKey,
    },
    logic::task::{TaskFields, delete_task, set_task_status, update_task, validate_parent},
    routes::{TaskDefaultRouteTag, TaskDetailRouteTag},
    scope::{
        apply_task_filters, apply_task_sort, effective_task_sort, find_status_by_name,
        find_task_scoped, load_status_choices, load_status_map, task_display_label,
        user_display_label, user_exists,
    },
    state::TasksState,
    templates::{
        ConfirmDeletePage, TaskCreateModalPage, TaskDetailPage, TaskEditModalPage, TaskListPage,
        TaskOption, TaskRow, TaskSelectPage, TaskSubtask,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaskHubQuery {
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(
        default,
        rename = "AssignedToID",
        alias = "AssignedToId",
        alias = "assigned_to_id"
    )]
    pub assigned_to_id: Option<String>,
    #[serde(default, rename = "StatusID", alias = "StatusId", alias = "status_id")]
    pub status_id: Option<String>,
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

fn parse_positive_id(raw: Option<&str>) -> Option<i64> {
    raw.and_then(|s| s.trim().parse().ok()).filter(|id| *id > 0)
}

/// A superuser may look across assignees. Every other role is limited to their own.
pub(crate) fn sees_every_task(role: &str) -> bool {
    Superuser::matches(role)
}

/// Superuser, or the user the task is assigned to.
pub(crate) fn may_set_status(role: &str, user_id: i64, assigned_to_id: i64) -> bool {
    sees_every_task(role) || user_id == assigned_to_id
}

/// Missing `AssignedToID` defaults to the current user when `default_to_current` is set.
/// An empty value means any user.
pub(crate) fn assigned_to_filter(
    raw: Option<&str>,
    current_user_id: i64,
    default_to_current: bool,
) -> Option<i64> {
    match raw {
        None if default_to_current => Some(current_user_id),
        None => None,
        Some(s) if s.trim().is_empty() => None,
        Some(s) => parse_positive_id(Some(s)),
    }
}

/// Assignee constraint for a task list. Non-superusers always see their own tasks.
pub(crate) fn visible_assignee_filter(role: &str, user_id: i64, raw: Option<&str>) -> Option<i64> {
    if sees_every_task(role) {
        assigned_to_filter(raw, user_id, false)
    } else {
        Some(user_id)
    }
}

/// Assignee the Clear button restores. Empty for a superuser.
pub(crate) fn default_assignee_fields(auth: &AuthContext) -> (String, String) {
    if sees_every_task(&auth.role) {
        (String::new(), String::new())
    } else {
        (auth.user.id.to_string(), auth.user.name.clone())
    }
}

fn parse_optional_parent(raw: &str) -> Result<Option<i64>, &'static str> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    match raw.parse::<i64>() {
        Ok(id) if id > 0 => Ok(Some(id)),
        _ => Err("invalid parent task"),
    }
}

fn parse_priority(s: &str) -> Result<i32, &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Err("priority is required");
    }
    s.parse::<i32>().ok().ok_or("invalid priority")
}

async fn query_tasks(
    db: &sea_orm::DatabaseConnection,
    q: &TaskHubQuery,
    auth: &AuthContext,
    page_size: u32,
) -> (Vec<TaskRow>, u32, u64) {
    let assigned_to_id =
        visible_assignee_filter(&auth.role, auth.user.id, q.assigned_to_id.as_deref());
    let status_id = parse_positive_id(q.status_id.as_deref());
    let mut query = scope_allowed::<super::super::routes::TasksView, _>(TaskEntity::find());
    query = apply_task_filters(query, q.title.as_deref(), assigned_to_id, status_id);

    let sort = effective_task_sort(q.sort.as_deref());
    query = apply_task_sort(query, Some(sort.as_str()));
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let status_map = load_status_map(db).await;
    let rows = models
        .into_iter()
        .map(|t| {
            let (status_name, status_color) = status_map
                .get(&t.status_id)
                .cloned()
                .unwrap_or_else(|| (format!("Status #{}", t.status_id), 0));
            TaskRow {
                id: t.id,
                title: t.title,
                assigned_to: String::new(),
                assigned_to_id: t.assigned_to_id,
                status: status_name,
                status_color,
                can_set_status: may_set_status(&auth.role, auth.user.id, t.assigned_to_id),
                priority: t.priority,
                due_datetime: auth.format_datetime(t.due_datetime).into_string(),
                detail_href: TaskDetailRouteTag::new(t.id).url(),
            }
        })
        .collect();
    (rows, page, total)
}

async fn fill_assigned_to_labels(db: &sea_orm::DatabaseConnection, rows: &mut [TaskRow]) {
    for row in rows {
        row.assigned_to = user_display_label(db, row.assigned_to_id).await;
    }
}

async fn task_list_page(
    db: &sea_orm::DatabaseConnection,
    ctx: &AuthContext,
    q: &TaskHubQuery,
    path_and_query: String,
) -> TaskListPage {
    let (mut rows, page, total) = query_tasks(db, q, ctx, q.page_size.get()).await;
    fill_assigned_to_labels(db, &mut rows).await;
    let tasks = ObjectList::from_page(rows, page, q.page_size.get(), total);
    let default_assignee = default_assignee_fields(ctx);
    let filter_assigned_to_id =
        visible_assignee_filter(&ctx.role, ctx.user.id, q.assigned_to_id.as_deref());
    let filter_assigned_to_display = match filter_assigned_to_id {
        Some(id) if id == ctx.user.id => ctx.user.name.clone(),
        Some(id) => user_display_label(db, id).await,
        None => String::new(),
    };
    TaskListPage {
        tasks,
        filter_title: q.title.clone().unwrap_or_default(),
        filter_assigned_to_id: filter_assigned_to_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        filter_assigned_to_display,
        filter_status_id: q.status_id.clone().unwrap_or_default(),
        status_choices: load_status_choices(db).await,
        show_assignee_filter: sees_every_task(&ctx.role),
        default_assigned_to_id: default_assignee.0,
        default_assigned_to_display: default_assignee.1,
        sort: effective_task_sort(q.sort.as_deref()),
        path_and_query,
        page_size: q.page_size.get(),
    }
}

fn browser_path_and_query(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    rest.find('/')
        .map(|index| rest[index..].to_string())
        .unwrap_or_else(|| TaskDefaultRouteTag.path())
}

fn list_query_from_browser(htmx: &Htmx) -> (String, TaskHubQuery) {
    let path_and_query = htmx
        .current_url
        .as_deref()
        .map(browser_path_and_query)
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| TaskDefaultRouteTag.path());
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

async fn task_detail_page(
    db: &sea_orm::DatabaseConnection,
    ctx: &AuthContext,
    task: task::Model,
) -> TaskDetailPage {
    let status = lariv_core::web::opt_or_log(
        crate::entities::TaskStatusEntity::find_by_id(task.status_id)
            .one(db)
            .await,
        "find status by id",
    );
    let (status_name, status_color) = match status {
        Some(s) => (s.name, s.color),
        None => (format!("Status #{}", task.status_id), 0),
    };
    let (parent_title, parent_href) = match task.parent_id.filter(|id| *id > 0) {
        Some(pid) => {
            let title = task_display_label(db, pid).await;
            let title = if title.is_empty() {
                format!("Task #{pid}")
            } else {
                title
            };
            let href = if find_task_scoped(db, pid).await.is_some() {
                TaskDetailRouteTag::new(pid).url()
            } else {
                String::new()
            };
            (title, href)
        }
        None => (String::new(), String::new()),
    };
    let mut children_query =
        scope_allowed::<super::super::routes::TasksView, _>(TaskEntity::find())
            .filter(task::Column::ParentId.eq(task.id));
    if !sees_every_task(&ctx.role) {
        children_query = children_query.filter(task::Column::AssignedToId.eq(ctx.user.id));
    }
    let children = children_query
        .order_by_asc(task::Column::Title)
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|child| TaskSubtask {
            title: child.title,
            href: TaskDetailRouteTag::new(child.id).url(),
        })
        .collect();
    TaskDetailPage {
        id: task.id,
        title: task.title,
        description: task.description,
        assigned_to: user_display_label(db, task.assigned_to_id).await,
        parent_title,
        parent_href,
        subtasks: children,
        status: status_name,
        status_color,
        can_set_status: may_set_status(&ctx.role, ctx.user.id, task.assigned_to_id),
        priority: task.priority,
        due_datetime: ctx.format_datetime(task.due_datetime).into_string(),
    }
}

pub async fn hub(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TaskHubQuery>,
) -> maud::Markup {
    let page = task_list_page(&state.db, &ctx, &q, path_and_query(&uri)).await;
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref()) {
        if TaskTableKey::matches_id(instance) {
            return page.render_table_rows(instance);
        }
    }
    if htmx.targets::<TaskTableKey>() {
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

pub async fn detail(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(task) = find_task_scoped(&state.db, id).await else {
        return Redirect::to(&TaskDefaultRouteTag.url()).into_response();
    };
    let page = task_detail_page(&state.db, &ctx, task).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn set_status(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path((id, slug)): Path<(i64, String)>,
) -> Response {
    let Some(name) = status_name_from_slug(&slug) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(task) = find_task_scoped(&state.db, id).await else {
        return Redirect::to(&TaskDefaultRouteTag.url()).into_response();
    };
    if !may_set_status(&ctx.role, ctx.user.id, task.assigned_to_id) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(status) = find_status_by_name(&state.db, name).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if task.status_id != status.id
        && let Err(e) = set_task_status(&state.db, task, status.id, &ctx).await
    {
        tracing::error!(error = %e, id, "failed to set task status");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if htmx.targets::<TaskTableKey>() {
        let (path_and_query, q) = list_query_from_browser(&htmx);
        let page = task_list_page(&state.db, &ctx, &q, path_and_query).await;
        if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref()) {
            if TaskTableKey::matches_id(instance) {
                return page.render_table_rows(instance).into_response();
            }
        }
        return page.render_table().into_response();
    }
    if !htmx.request {
        return Redirect::to(&TaskDetailRouteTag::new(id).url()).into_response();
    }
    let Some(task) = find_task_scoped(&state.db, id).await else {
        return Redirect::to(&TaskDefaultRouteTag.url()).into_response();
    };
    let page = task_detail_page(&state.db, &ctx, task).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(_state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = TaskCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: String::new(),
        description: String::new(),
        assigned_to_id: ctx.user.id,
        assigned_to_display: ctx.user.name.clone(),
        parent_id: String::new(),
        parent_display: String::new(),
        priority: "0".to_string(),
        due_datetime: ctx.datetime_local_input(Utc::now()).into_string(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

fn create_modal_page(
    q: &ModalNameQuery,
    form: &TaskForm,
    assigned_to_display: String,
    parent_display: String,
    error: String,
) -> TaskCreateModalPage {
    TaskCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: form.title.clone(),
        description: form.description.clone(),
        assigned_to_id: form.assigned_to_id,
        assigned_to_display,
        parent_id: form.parent_id.clone(),
        parent_display,
        priority: form.priority.clone(),
        due_datetime: form.due_datetime.clone(),
        error,
    }
}

pub async fn create_post(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TaskForm>,
) -> Response {
    let assigned_to_display = user_display_label(&state.db, form.assigned_to_id).await;
    let parent_id = match parse_optional_parent(&form.parent_id) {
        Ok(id) => id,
        Err(e) => {
            let page = create_modal_page(&q, &form, assigned_to_display, String::new(), e.into());
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let parent_display = match parent_id {
        Some(id) => task_display_label(&state.db, id).await,
        None => String::new(),
    };
    if form.title.trim().is_empty() {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            parent_display,
            "title is required".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    if form.assigned_to_id <= 0 || !user_exists(&state.db, form.assigned_to_id).await {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            parent_display,
            "assigned to is required".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    if let Err(e) = validate_parent(&state.db, None, parent_id).await {
        let page = create_modal_page(&q, &form, assigned_to_display, parent_display, e);
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let Some(status) = find_status_by_name(&state.db, STATUS_TODO).await else {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            parent_display,
            "To Do status is not configured".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    let priority = match parse_priority(&form.priority) {
        Ok(p) => p,
        Err(e) => {
            let page = create_modal_page(&q, &form, assigned_to_display, parent_display, e.into());
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let Some(due_datetime) = ctx.parse_datetime_local_input(&form.due_datetime) else {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            parent_display,
            "invalid due date & time".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    let now = Utc::now();
    let model = task::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(form.title.trim().to_string()),
        description: Set(form.description.clone()),
        assigned_to_id: Set(form.assigned_to_id),
        status_id: Set(status.id),
        parent_id: Set(parent_id),
        priority: Set(priority),
        due_datetime: Set(due_datetime),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<TaskCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &TaskDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = create_modal_page(
                &q,
                &form,
                assigned_to_display,
                parent_display,
                e.to_string(),
            );
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(task) = find_task_scoped(&state.db, id).await else {
        return Redirect::to(&TaskDefaultRouteTag.url()).into_response();
    };
    let page = TaskEditModalPage {
        id: task.id,
        form_name: q.form_name(),
        title: task.title,
        description: task.description,
        assigned_to_id: task.assigned_to_id,
        assigned_to_display: user_display_label(&state.db, task.assigned_to_id).await,
        parent_id: task.parent_id.map(|id| id.to_string()).unwrap_or_default(),
        parent_display: task_display_label(&state.db, task.parent_id.unwrap_or(0)).await,
        priority: task.priority.to_string(),
        due_datetime: ctx.datetime_local_input(task.due_datetime).into_string(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn task_edit_modal_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &TaskForm,
    error: &str,
) -> Response {
    let page = TaskEditModalPage {
        id,
        form_name: q.form_name(),
        title: form.title.clone(),
        description: form.description.clone(),
        assigned_to_id: form.assigned_to_id,
        assigned_to_display: user_display_label(db, form.assigned_to_id).await,
        parent_id: form.parent_id.clone(),
        parent_display: match parse_optional_parent(&form.parent_id) {
            Ok(Some(id)) => task_display_label(db, id).await,
            _ => String::new(),
        },
        priority: form.priority.clone(),
        due_datetime: form.due_datetime.clone(),
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
    HtmlFormBody(form): HtmlFormBody<TaskForm>,
) -> Response {
    let Some(existing) = find_task_scoped(&state.db, id).await else {
        return Redirect::to(&TaskDefaultRouteTag.url()).into_response();
    };
    if form.title.trim().is_empty() {
        return task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, "title is required")
            .await;
    }
    let parent_id = match parse_optional_parent(&form.parent_id) {
        Ok(id) => id,
        Err(e) => {
            return task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, e).await;
        }
    };
    if form.assigned_to_id <= 0 || !user_exists(&state.db, form.assigned_to_id).await {
        return task_edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            "assigned to is required",
        )
        .await;
    }
    let priority = match parse_priority(&form.priority) {
        Ok(p) => p,
        Err(e) => {
            return task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, e).await;
        }
    };
    let Some(due_datetime) = ctx.parse_datetime_local_input(&form.due_datetime) else {
        return task_edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            "invalid due date & time",
        )
        .await;
    };
    let status_id = existing.status_id;
    match update_task(
        &state.db,
        existing,
        TaskFields {
            title: form.title.trim().to_string(),
            description: form.description.clone(),
            assigned_to_id: form.assigned_to_id,
            status_id,
            parent_id,
            priority,
            due_datetime,
        },
        &ctx,
    )
    .await
    {
        Ok(_) => {
            respond_edit_modal_done::<TaskEditModalKey>(&htmx, &TaskDetailRouteTag::new(id).url())
        }
        Err(e) => {
            task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e.to_string()).await
        }
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaskSelectQuery {
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub target_input: Option<String>,
}

pub async fn select(
    Cap(state): Cap<TasksState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TaskSelectQuery>,
) -> maud::Markup {
    let assigned_to_id = visible_assignee_filter(&ctx.role, ctx.user.id, None);
    let mut query = scope_allowed::<super::super::routes::TasksView, _>(TaskEntity::find());
    query = apply_task_filters(query, q.title.as_deref(), assigned_to_id, None);
    let sort = effective_task_sort(q.sort.as_deref());
    query = apply_task_sort(query, Some(sort.as_str()));
    let page = q.page.get();
    let page_size = q.page_size.get();
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let tasks = ObjectList::from_page(
        models
            .into_iter()
            .map(|t| TaskOption {
                id: t.id,
                title: t.title,
            })
            .collect(),
        page,
        page_size,
        total,
    );
    let page = TaskSelectPage {
        tasks,
        filter_title: q.title.clone().unwrap_or_default(),
        sort,
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "ParentID".into()),
        page_size,
    };
    respond_picker_select::<TaskSelectTableKey, TaskSelectModalKey, _>(&htmx, &page)
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: TaskDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this task?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_tasks.TaskDeleteForm".into()),
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
    match delete_task(&state.db, id).await {
        Ok(()) => htmx.redirect(&TaskDefaultRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete task");
            let page = ConfirmDeletePage {
                modal_uid: TaskDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this task?".into(),
                form_name: "p_tasks.TaskDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TaskHubQuery, assigned_to_filter, may_set_status, sees_every_task, visible_assignee_filter,
    };
    use crate::handlers::statuses::StatusDetailQuery;
    use lariv_core::html_form::UrlencodedFields;
    use lariv_plugin_users::roles::Superuser;

    fn parse<T: serde::de::DeserializeOwned>(query: &str) -> T {
        UrlencodedFields::parse(query.as_bytes())
            .unwrap()
            .deserialize()
            .unwrap()
    }

    #[test]
    fn missing_assignee_defaults_to_the_current_user() {
        assert_eq!(assigned_to_filter(None, 7, true), Some(7));
    }

    #[test]
    fn missing_assignee_is_unfiltered_for_a_superuser() {
        assert!(sees_every_task(Superuser::NAME));
        assert_eq!(assigned_to_filter(None, 7, false), None);
        assert_eq!(visible_assignee_filter(Superuser::NAME, 7, None), None);
    }

    #[test]
    fn other_users_only_see_tasks_assigned_to_them() {
        assert!(!sees_every_task("unassigned"));
        assert_eq!(visible_assignee_filter("unassigned", 7, None), Some(7));
        assert_eq!(
            visible_assignee_filter("unassigned", 7, Some("12")),
            Some(7)
        );
        assert_eq!(visible_assignee_filter("unassigned", 7, Some("")), Some(7));
    }

    #[test]
    fn status_changes_follow_the_assignee() {
        assert!(may_set_status(Superuser::NAME, 1, 9));
        assert!(may_set_status("unassigned", 7, 7));
        assert!(!may_set_status("unassigned", 7, 9));
    }

    #[test]
    fn empty_assignee_means_any_user() {
        assert_eq!(assigned_to_filter(Some(""), 7, true), None);
        assert_eq!(assigned_to_filter(Some("  "), 7, true), None);
    }

    #[test]
    fn chosen_assignee_filters_to_that_user() {
        assert_eq!(assigned_to_filter(Some("12"), 7, false), Some(12));
        assert_eq!(assigned_to_filter(Some("0"), 7, true), None);
    }

    #[test]
    fn filter_query_accepts_the_form_field_name() {
        let q: TaskHubQuery = parse("AssignedToID=12&StatusID=3");
        assert_eq!(q.assigned_to_id.as_deref(), Some("12"));
        assert_eq!(q.status_id.as_deref(), Some("3"));
        let q: TaskHubQuery = parse("AssignedToId=9&StatusId=4");
        assert_eq!(q.assigned_to_id.as_deref(), Some("9"));
        assert_eq!(q.status_id.as_deref(), Some("4"));
        let q: StatusDetailQuery = parse("AssignedToID=12");
        assert_eq!(q.assigned_to_id.as_deref(), Some("12"));
    }
}
