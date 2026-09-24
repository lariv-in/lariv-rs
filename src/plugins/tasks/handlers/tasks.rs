use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait};

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
    entities::task::{self, Entity as TaskEntity},
    forms::TaskForm,
    handlers::{ModalNameQuery, logs::load_logs_panel},
    keys::{TaskCreateModalKey, TaskDeleteModalKey, TaskEditModalKey, TaskLogsKey, TaskTableKey},
    logic::task::{TaskFields, delete_task, update_task},
    routes::TaskDetailRouteTag,
    scope::{
        apply_task_filters, apply_task_sort, find_task_scoped, load_status_choices,
        load_status_map, scope_superuser, status_exists, user_display_label, user_exists,
    },
    state::TasksState,
    templates::{
        ConfirmDeletePage, TaskCreateModalPage, TaskDetailPage, TaskEditModalPage, TaskListPage,
        TaskRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaskHubQuery {
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(default, rename = "AssignedToId", alias = "assigned_to_id")]
    pub assigned_to_id: Option<String>,
    #[serde(default, rename = "StatusId", alias = "status_id")]
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

/// Missing AssignedToId defaults to the current user. Empty means any user.
fn assigned_to_filter(raw: Option<&str>, current_user_id: i64) -> Option<i64> {
    match raw {
        None => Some(current_user_id),
        Some(s) if s.trim().is_empty() => None,
        Some(s) => parse_positive_id(Some(s)),
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
    let assigned_to_id = assigned_to_filter(q.assigned_to_id.as_deref(), auth.user.id);
    let status_id = parse_positive_id(q.status_id.as_deref());
    let mut query = TaskEntity::find();
    query = apply_task_filters(query, q.title.as_deref(), assigned_to_id, status_id);
    query = scope_superuser(query, auth);
    query = apply_task_sort(query, q.sort.as_deref());
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

pub async fn hub(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TaskHubQuery>,
) -> maud::Markup {
    let (mut rows, page, total) = query_tasks(&state.db, &q, &ctx, q.page_size.get()).await;
    fill_assigned_to_labels(&state.db, &mut rows).await;
    let tasks = ObjectList::from_page(rows, page, q.page_size.get(), total);
    let filter_assigned_to_id = assigned_to_filter(q.assigned_to_id.as_deref(), ctx.user.id);
    let filter_assigned_to_display = match filter_assigned_to_id {
        Some(id) if id == ctx.user.id => ctx.user.name.clone(),
        Some(id) => user_display_label(&state.db, id).await,
        None => String::new(),
    };
    let page = TaskListPage {
        tasks,
        filter_title: q.title.clone().unwrap_or_default(),
        filter_assigned_to_id: filter_assigned_to_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        filter_assigned_to_display,
        filter_status_id: q.status_id.clone().unwrap_or_default(),
        status_choices: load_status_choices(&state.db).await,
        default_assigned_to_id: ctx.user.id.to_string(),
        default_assigned_to_display: ctx.user.name.clone(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
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
    let Some(task) = find_task_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/tasks").into_response();
    };
    let status = crate::web::opt_or_log(
        crate::plugins::tasks::entities::TaskStatusEntity::find_by_id(task.status_id)
            .one(&state.db)
            .await,
        "find status by id",
    );
    let (status_name, status_color) = match status {
        Some(s) => (s.name, s.color),
        None => (format!("Status #{}", task.status_id), 0),
    };
    let can_edit = ctx.user.is_superuser;
    let page = TaskDetailPage {
        id: task.id,
        title: task.title,
        description: task.description,
        assigned_to: user_display_label(&state.db, task.assigned_to_id).await,
        status: status_name,
        status_color,
        priority: task.priority,
        due_datetime: ctx.format_datetime(task.due_datetime).into_string(),
        can_edit,
        logs: load_logs_panel(&state.db, &ctx, task.id, can_edit).await,
    };
    if htmx.targets::<TaskLogsKey>() {
        return page.logs.render_list().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = TaskCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: String::new(),
        description: String::new(),
        assigned_to_id: ctx.user.id,
        assigned_to_display: ctx.user.name.clone(),
        status_id: String::new(),
        status_choices: load_status_choices(&state.db).await,
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
    status_choices: Vec<(String, String)>,
    error: String,
) -> TaskCreateModalPage {
    TaskCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: form.title.clone(),
        description: form.description.clone(),
        assigned_to_id: form.assigned_to_id,
        assigned_to_display,
        status_id: form.status_id.clone(),
        status_choices,
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
    if !ctx.user.is_superuser {
        return Redirect::to("/tasks").into_response();
    }
    let assigned_to_display = user_display_label(&state.db, form.assigned_to_id).await;
    let status_choices = load_status_choices(&state.db).await;
    if form.title.trim().is_empty() {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            status_choices,
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
            status_choices,
            "assigned to is required".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let Some(status_id) = parse_positive_id(Some(&form.status_id)) else {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            status_choices,
            "status is required".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    if !status_exists(&state.db, status_id).await {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            status_choices,
            "status is required".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let priority = match parse_priority(&form.priority) {
        Ok(p) => p,
        Err(e) => {
            let page = create_modal_page(&q, &form, assigned_to_display, status_choices, e.into());
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let Some(due_datetime) = ctx.parse_datetime_local_input(&form.due_datetime) else {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            status_choices,
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
        status_id: Set(status_id),
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
                status_choices,
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
    if !ctx.user.is_superuser {
        return Redirect::to("/tasks").into_response();
    }
    let Some(task) = find_task_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/tasks").into_response();
    };
    let page = TaskEditModalPage {
        id: task.id,
        form_name: q.form_name(),
        title: task.title,
        description: task.description,
        assigned_to_id: task.assigned_to_id,
        assigned_to_display: user_display_label(&state.db, task.assigned_to_id).await,
        status_id: task.status_id.to_string(),
        status_choices: load_status_choices(&state.db).await,
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
        status_id: form.status_id.clone(),
        status_choices: load_status_choices(db).await,
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
    if !ctx.user.is_superuser {
        return Redirect::to("/tasks").into_response();
    }
    let Some(existing) = find_task_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/tasks").into_response();
    };
    if form.title.trim().is_empty() {
        return task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, "title is required")
            .await;
    }
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
    let Some(status_id) = parse_positive_id(Some(&form.status_id)) else {
        return task_edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            "status is required",
        )
        .await;
    };
    if !status_exists(&state.db, status_id).await {
        return task_edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            "status is required",
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
    match update_task(
        &state.db,
        existing,
        TaskFields {
            title: form.title.trim().to_string(),
            description: form.description.clone(),
            assigned_to_id: form.assigned_to_id,
            status_id,
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
    if !ctx.user.is_superuser {
        return Redirect::to("/tasks").into_response();
    }
    match delete_task(&state.db, id, &ctx).await {
        Ok(()) => htmx.redirect("/tasks"),
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
