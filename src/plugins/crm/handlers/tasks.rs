use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryFilter};

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

use crate::plugins::crm::{
    entities::{
        completed_task::Entity as CompletedTaskEntity,
        task::{self, Entity as TaskEntity},
    },
    forms::TaskForm,
    handlers::ModalNameQuery,
    keys::{TaskCreateModalKey, TaskDeleteModalKey, TaskEditModalKey, TaskTableKey},
    logic::task::{
        complete_task, completed_task_id_for, delete_uncompleted_task, err_if_task_completed,
    },
    routes::{CompletedTaskDetailRouteTag, TaskDetailRouteTag},
    scope::{
        apply_completed_task_filters, apply_completed_task_sort, apply_task_filters,
        apply_task_sort, find_completed_task_scoped, find_task_scoped, find_uncompleted_task,
        format_due_date, open_task_status, scope_superuser, sql_task_uncompleted,
        today_in_timezone, user_display_label, user_exists,
    },
    state::CrmState,
    templates::{
        CompletedTaskDetailPage, ConfirmDeletePage, TaskCreateModalPage, TaskDetailPage,
        TaskEditModalPage, TaskListPage, TaskRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaskHubQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(default, rename = "AssignedToId", alias = "assigned_to_id")]
    pub assigned_to_id: Option<String>,
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

fn opt_string(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

/// Missing AssignedToId defaults to the current user. Empty means any user.
fn assigned_to_filter(raw: Option<&str>, current_user_id: i64) -> Option<i64> {
    match raw {
        None => Some(current_user_id),
        Some(s) if s.trim().is_empty() => None,
        Some(s) => parse_user_id(s),
    }
}

fn parse_user_id(raw: &str) -> Option<i64> {
    raw.trim().parse().ok().filter(|id| *id > 0)
}

fn parse_due_date(s: &str) -> Result<Option<NaiveDate>, &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    crate::datetime::parse_date(s)
        .map(Some)
        .ok_or("invalid due date")
}

fn hub_tab(raw: Option<&str>) -> String {
    match raw.map(str::trim) {
        Some("completed") => "completed".to_string(),
        _ => "uncompleted".to_string(),
    }
}

async fn query_uncompleted_tasks(
    db: &sea_orm::DatabaseConnection,
    q: &TaskHubQuery,
    auth: &AuthContext,
    page_size: u32,
) -> (Vec<TaskRow>, u32, u64) {
    let assigned_to_id = assigned_to_filter(q.assigned_to_id.as_deref(), auth.user.id);
    let today = today_in_timezone(&auth.timezone);
    let mut query = TaskEntity::find().filter(sql_task_uncompleted());
    query = apply_task_filters(query, q.title.as_deref(), assigned_to_id);
    query = scope_superuser(query, auth);
    query = apply_task_sort(query, q.sort.as_deref(), today);
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|t| TaskRow {
            id: t.id,
            title: t.title,
            assigned_to: String::new(),
            assigned_to_id: t.assigned_to_id,
            due_date: format_due_date(t.due_date),
            status: open_task_status(t.due_date, today).to_string(),
            completed_at: String::new(),
            detail_href: TaskDetailRouteTag::new(t.id).url(),
        })
        .collect();
    (rows, page, total)
}

async fn query_completed_tasks(
    db: &sea_orm::DatabaseConnection,
    q: &TaskHubQuery,
    auth: &AuthContext,
    page_size: u32,
) -> (Vec<TaskRow>, u32, u64) {
    let assigned_to_id = assigned_to_filter(q.assigned_to_id.as_deref(), auth.user.id);
    let mut query = CompletedTaskEntity::find();
    query =
        apply_completed_task_filters(query, q.title.as_deref(), assigned_to_id, q.sort.as_deref());
    query = scope_superuser(query, auth);
    query = apply_completed_task_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let completed = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(completed.len());
    for c in completed {
        let task = crate::web::opt_or_log(
            TaskEntity::find_by_id(c.task_id).one(db).await,
            "find by id",
        );
        let (title, assigned_to_id, due_date) = match task {
            Some(t) => (t.title, t.assigned_to_id, format_due_date(t.due_date)),
            None => (format!("Task #{}", c.task_id), 0, String::new()),
        };
        rows.push(TaskRow {
            id: c.id,
            title,
            assigned_to: String::new(),
            assigned_to_id,
            due_date,
            status: "Completed".to_string(),
            completed_at: auth.format_datetime(c.completed_at).into_string(),
            detail_href: CompletedTaskDetailRouteTag::new(c.id).url(),
        });
    }
    (rows, page, total)
}

async fn fill_assigned_to_labels(db: &sea_orm::DatabaseConnection, rows: &mut [TaskRow]) {
    for row in rows {
        row.assigned_to = user_display_label(db, row.assigned_to_id).await;
    }
}

pub async fn hub(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TaskHubQuery>,
) -> maud::Markup {
    let tab = hub_tab(q.tab.as_deref());
    let (mut rows, page, total) = if tab == "completed" {
        query_completed_tasks(&state.db, &q, &ctx, q.page_size.get()).await
    } else {
        query_uncompleted_tasks(&state.db, &q, &ctx, q.page_size.get()).await
    };
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
        tab,
        filter_title: q.title.clone().unwrap_or_default(),
        filter_assigned_to_id: filter_assigned_to_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        filter_assigned_to_display,
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
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(task) = find_task_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/tasks").into_response();
    };
    if find_uncompleted_task(&state.db, id, &ctx).await.is_none() {
        if let Some(completed_id) = completed_task_id_for(&state.db, id).await {
            return Redirect::to(&CompletedTaskDetailRouteTag::new(completed_id).url())
                .into_response();
        }
        return Redirect::to("/crm/tasks").into_response();
    }
    let page = TaskDetailPage {
        id: task.id,
        title: task.title,
        description: task.description.unwrap_or_default(),
        assigned_to: user_display_label(&state.db, task.assigned_to_id).await,
        due_date: format_due_date(task.due_date),
        status: open_task_status(task.due_date, today_in_timezone(&ctx.timezone)).to_string(),
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn completed_detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(completed) = find_completed_task_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/tasks?tab=completed").into_response();
    };
    let task = find_task_scoped(&state.db, completed.task_id, &ctx).await;
    let (title, description, assigned_to_id, due_date) = match task {
        Some(t) => (
            t.title,
            t.description.unwrap_or_default(),
            t.assigned_to_id,
            format_due_date(t.due_date),
        ),
        None => (
            format!("Task #{}", completed.task_id),
            String::new(),
            0,
            String::new(),
        ),
    };
    let page = CompletedTaskDetailPage {
        id: completed.id,
        title,
        description,
        assigned_to: user_display_label(&state.db, assigned_to_id).await,
        due_date,
        completed_at: ctx.format_datetime(completed.completed_at).into_string(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
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
        due_date: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

fn create_modal_page(
    q: &ModalNameQuery,
    form: &TaskForm,
    assigned_to_display: String,
    error: String,
) -> TaskCreateModalPage {
    TaskCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: form.title.clone(),
        description: form.description.clone(),
        assigned_to_id: form.assigned_to_id,
        assigned_to_display,
        due_date: form.due_date.clone(),
        error,
    }
}

pub async fn create_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TaskForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/tasks").into_response();
    }
    let assigned_to_id = form.assigned_to_id;
    let assigned_to_display = user_display_label(&state.db, assigned_to_id).await;
    if form.title.trim().is_empty() {
        let page = create_modal_page(&q, &form, assigned_to_display, "title is required".into());
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    if assigned_to_id <= 0 || !user_exists(&state.db, assigned_to_id).await {
        let page = create_modal_page(
            &q,
            &form,
            assigned_to_display,
            "assigned to is required".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let due_date = match parse_due_date(&form.due_date) {
        Ok(d) => d,
        Err(e) => {
            let page = create_modal_page(&q, &form, assigned_to_display, e.into());
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let now = Utc::now();
    let model = task::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(form.title.trim().to_string()),
        description: Set(opt_string(form.description.clone())),
        assigned_to_id: Set(assigned_to_id),
        due_date: Set(due_date),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<TaskCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &TaskDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = create_modal_page(&q, &form, assigned_to_display, e.to_string());
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/tasks").into_response();
    }
    let Some(task) = find_uncompleted_task(&state.db, id, &ctx).await else {
        if let Some(completed_id) = completed_task_id_for(&state.db, id).await {
            return Redirect::to(&CompletedTaskDetailRouteTag::new(completed_id).url())
                .into_response();
        }
        return Redirect::to("/crm/tasks").into_response();
    };
    let page = TaskEditModalPage {
        id: task.id,
        form_name: q.form_name(),
        title: task.title,
        description: task.description.unwrap_or_default(),
        assigned_to_id: task.assigned_to_id,
        assigned_to_display: user_display_label(&state.db, task.assigned_to_id).await,
        due_date: format_due_date(task.due_date),
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
        due_date: form.due_date.clone(),
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TaskForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/tasks").into_response();
    }
    let Some(existing) = find_uncompleted_task(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/tasks").into_response();
    };
    if let Err(e) = err_if_task_completed(&state.db, id).await {
        return task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e).await;
    }
    if form.title.trim().is_empty() {
        return task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, "title is required")
            .await;
    }
    let assigned_to_id = form.assigned_to_id;
    if assigned_to_id <= 0 || !user_exists(&state.db, assigned_to_id).await {
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
    let due_date = match parse_due_date(&form.due_date) {
        Ok(d) => d,
        Err(e) => {
            return task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, e).await;
        }
    };
    let now = Utc::now();
    let mut am: task::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.title = Set(form.title.trim().to_string());
    am.description = Set(opt_string(form.description.clone()));
    am.assigned_to_id = Set(assigned_to_id);
    am.due_date = Set(due_date);
    match am.update(&state.db).await {
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
            .unwrap_or_else(|| "p_crm.TaskDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/tasks").into_response();
    }
    match delete_uncompleted_task(&state.db, id, &ctx).await {
        Ok(()) => htmx.redirect("/crm/tasks"),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete task");
            let page = ConfirmDeletePage {
                modal_uid: TaskDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this task?".into(),
                form_name: "p_crm.TaskDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn complete_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/tasks").into_response();
    }
    match complete_task(&state.db, id, &ctx).await {
        Ok(completed_id) => {
            Redirect::to(&CompletedTaskDetailRouteTag::new(completed_id).url()).into_response()
        }
        Err(_) => {
            if let Some(completed_id) = completed_task_id_for(&state.db, id).await {
                Redirect::to(&CompletedTaskDetailRouteTag::new(completed_id).url()).into_response()
            } else {
                Redirect::to("/crm/tasks").into_response()
            }
        }
    }
}
