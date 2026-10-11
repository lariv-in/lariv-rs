use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{DateTime, Utc};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, JoinType, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait,
};

use lariv_core::components::{
    ObjectList, SharedChromeFolder, SlotCtx, SwapKey, table_rows_instance_id,
};
use lariv_core::db::trigram::ci_contains;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    color::STATUS_TODO,
    entities::periodic_task::{self, Entity as PeriodicTaskEntity},
    forms::PeriodicTaskForm,
    handlers::{
        ModalNameQuery,
        tasks::{
            default_assignee_fields, parse_optional_parent, parse_priority, sees_every_task,
            visible_assignee_filter,
        },
    },
    keys::{
        PeriodicTaskCreateModalKey, PeriodicTaskDeleteModalKey, PeriodicTaskEditModalKey,
        PeriodicTaskTableKey,
    },
    logic::task::validate_parent,
    routes::{PeriodicTaskDefaultRouteTag, PeriodicTaskDetailRouteTag},
    scope::{
        find_periodic_scoped, find_status_by_name, load_status_choices, load_status_map,
        status_exists, task_display_label, user_display_label, user_exists,
    },
    state::TasksState,
    templates::{
        ConfirmDeletePage, PeriodicTaskCreateModalPage, PeriodicTaskDetailPage,
        PeriodicTaskEditModalPage, PeriodicTaskListPage, PeriodicTaskRow,
    },
};

const MIN_FREQUENCY_NS: i64 = 1_000_000_000;

#[derive(Debug, serde::Deserialize, Default)]
pub struct PeriodicListQuery {
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(
        default,
        rename = "AssignedToID",
        alias = "AssignedToId",
        alias = "assigned_to_id"
    )]
    pub assigned_to_id: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

struct PeriodicFields {
    title: String,
    description: String,
    assigned_to_id: i64,
    status_id: i64,
    parent_id: Option<i64>,
    priority: i32,
    due_datetime: DateTime<Utc>,
    frequency: i64,
    last_added_at: DateTime<Utc>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn list_url() -> String {
    PeriodicTaskDefaultRouteTag.url()
}

fn parse_frequency(raw: &str) -> Result<i64, String> {
    let nanos = lariv_core::duration::parse_duration(raw)?;
    if nanos < MIN_FREQUENCY_NS {
        return Err("frequency must be at least 1 second".into());
    }
    Ok(nanos)
}

async fn parse_fields(
    db: &sea_orm::DatabaseConnection,
    ctx: &lariv_plugin_users::state::AuthContext,
    form: &PeriodicTaskForm,
) -> Result<PeriodicFields, String> {
    if form.title.trim().is_empty() {
        return Err("title is required".into());
    }
    if form.assigned_to_id <= 0 || !user_exists(db, form.assigned_to_id).await {
        return Err("assigned to is required".into());
    }
    let status_id = form
        .status_id
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or("status is required")?;
    if !status_exists(db, status_id).await {
        return Err("status not found".into());
    }
    let parent_id = parse_optional_parent(&form.parent_id)?;
    validate_parent(db, None, parent_id).await?;
    let priority = parse_priority(&form.priority)?;
    let due_datetime = ctx
        .parse_datetime_local_input(&form.due_datetime)
        .ok_or("invalid due date & time")?;
    let frequency = parse_frequency(&form.frequency)?;
    let last_added_at = ctx
        .parse_datetime_local_input(&form.last_added_at)
        .ok_or("invalid last added at")?;
    Ok(PeriodicFields {
        title: form.title.trim().to_string(),
        description: form.description.clone(),
        assigned_to_id: form.assigned_to_id,
        status_id,
        parent_id,
        priority,
        due_datetime,
        frequency,
        last_added_at,
    })
}

fn apply_sort(
    mut query: sea_orm::Select<PeriodicTaskEntity>,
    sort: &str,
) -> sea_orm::Select<PeriodicTaskEntity> {
    let key = sort.split_whitespace().next().unwrap_or("");
    let desc = sort
        .split_whitespace()
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case("DESC"));
    if key.eq_ignore_ascii_case("AssignedTo") {
        query = query.join(
            JoinType::LeftJoin,
            periodic_task::Relation::AssignedTo.def(),
        );
    }
    match key {
        s if s.eq_ignore_ascii_case("Title") => {
            if desc {
                query.order_by_desc(periodic_task::Column::Title)
            } else {
                query.order_by_asc(periodic_task::Column::Title)
            }
        }
        s if s.eq_ignore_ascii_case("AssignedTo") => {
            if desc {
                query.order_by_desc(lariv_plugin_users::entities::user::Column::Name)
            } else {
                query.order_by_asc(lariv_plugin_users::entities::user::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Frequency") => {
            if desc {
                query.order_by_desc(periodic_task::Column::Frequency)
            } else {
                query.order_by_asc(periodic_task::Column::Frequency)
            }
        }
        s if s.eq_ignore_ascii_case("LastAddedAt") => {
            if desc {
                query.order_by_desc(periodic_task::Column::LastAddedAt)
            } else {
                query.order_by_asc(periodic_task::Column::LastAddedAt)
            }
        }
        _ => query.order_by_asc(periodic_task::Column::Title),
    }
}

async fn form_displays(
    db: &sea_orm::DatabaseConnection,
    form: &PeriodicTaskForm,
) -> (String, String) {
    let assigned = user_display_label(db, form.assigned_to_id).await;
    let parent = match parse_optional_parent(&form.parent_id) {
        Ok(Some(id)) => task_display_label(db, id).await,
        _ => String::new(),
    };
    (assigned, parent)
}

fn modal_from_form(
    form_name: String,
    refresh_table: String,
    form: &PeriodicTaskForm,
    assigned_to_display: String,
    parent_display: String,
    status_choices: Vec<(String, String)>,
    error: String,
) -> PeriodicTaskCreateModalPage {
    PeriodicTaskCreateModalPage {
        form_name,
        refresh_table,
        title: form.title.clone(),
        description: form.description.clone(),
        assigned_to_id: form.assigned_to_id,
        assigned_to_display,
        status_id: form.status_id.clone(),
        status_choices,
        parent_id: form.parent_id.clone(),
        parent_display,
        priority: form.priority.clone(),
        due_datetime: form.due_datetime.clone(),
        frequency: form.frequency.clone(),
        last_added_at: form.last_added_at.clone(),
        error,
    }
}

fn edit_modal_from_form(
    id: i64,
    form_name: String,
    form: &PeriodicTaskForm,
    assigned_to_display: String,
    parent_display: String,
    status_choices: Vec<(String, String)>,
    error: String,
) -> PeriodicTaskEditModalPage {
    PeriodicTaskEditModalPage {
        id,
        form_name,
        title: form.title.clone(),
        description: form.description.clone(),
        assigned_to_id: form.assigned_to_id,
        assigned_to_display,
        status_id: form.status_id.clone(),
        status_choices,
        parent_id: form.parent_id.clone(),
        parent_display,
        priority: form.priority.clone(),
        due_datetime: form.due_datetime.clone(),
        frequency: form.frequency.clone(),
        last_added_at: form.last_added_at.clone(),
        error,
    }
}

pub async fn list(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<PeriodicListQuery>,
) -> maud::Markup {
    let assigned_to_id =
        visible_assignee_filter(&ctx.role, ctx.user.id, q.assigned_to_id.as_deref());
    let mut query = scope_allowed::<super::super::routes::TasksView, _>(PeriodicTaskEntity::find());
    if let Some(title) = q.title.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(ci_contains(periodic_task::Column::Title, title));
    }
    if let Some(uid) = assigned_to_id {
        query = query.filter(periodic_task::Column::AssignedToId.eq(uid));
    }
    query = apply_sort(query, q.sort.as_deref().unwrap_or(""));
    let page = q.page.get();
    let paginator = query.paginate(&state.db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for row in models {
        rows.push(PeriodicTaskRow {
            title: row.title,
            assigned_to: user_display_label(&state.db, row.assigned_to_id).await,
            frequency: lariv_core::duration::format_duration(row.frequency),
            last_added_at: ctx.format_datetime(row.last_added_at).into_string(),
            detail_href: PeriodicTaskDetailRouteTag::new(row.id).url(),
        });
    }
    let default_assignee = default_assignee_fields(&ctx);
    let page = PeriodicTaskListPage {
        tasks: ObjectList::from_page(rows, page, q.page_size.get(), total),
        filter_title: q.title.clone().unwrap_or_default(),
        filter_assigned_to_id: assigned_to_id.map(|id| id.to_string()).unwrap_or_default(),
        filter_assigned_to_display: match assigned_to_id {
            Some(uid) if uid == ctx.user.id => ctx.user.name.clone(),
            Some(uid) => user_display_label(&state.db, uid).await,
            None => String::new(),
        },
        show_assignee_filter: sees_every_task(&ctx.role),
        default_assigned_to_id: default_assignee.0,
        default_assigned_to_display: default_assignee.1,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref()) {
        if PeriodicTaskTableKey::matches_id(instance) {
            return page.render_table_rows(instance);
        }
    }
    if htmx.targets::<PeriodicTaskTableKey>() {
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
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let choices = load_status_choices(&state.db).await;
    let status_id = find_status_by_name(&state.db, STATUS_TODO)
        .await
        .map(|s| s.id.to_string())
        .or_else(|| choices.first().map(|(id, _)| id.clone()))
        .unwrap_or_default();
    let now = ctx.datetime_local_input(Utc::now()).into_string();
    let page = PeriodicTaskCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: String::new(),
        description: String::new(),
        assigned_to_id: ctx.user.id,
        assigned_to_display: ctx.user.name.clone(),
        status_id,
        status_choices: choices,
        parent_id: String::new(),
        parent_display: String::new(),
        priority: "0".into(),
        due_datetime: now.clone(),
        frequency: String::new(),
        last_added_at: now,
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
    HtmlFormBody(form): HtmlFormBody<PeriodicTaskForm>,
) -> Response {
    let choices = load_status_choices(&state.db).await;
    let (assigned_to_display, parent_display) = form_displays(&state.db, &form).await;
    let fields = match parse_fields(&state.db, &ctx, &form).await {
        Ok(fields) => fields,
        Err(error) => {
            let page = modal_from_form(
                q.form_name(),
                q.refresh_table(),
                &form,
                assigned_to_display,
                parent_display,
                choices,
                error,
            );
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let now = Utc::now();
    let model = periodic_task::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(fields.title),
        description: Set(fields.description),
        assigned_to_id: Set(fields.assigned_to_id),
        status_id: Set(fields.status_id),
        parent_id: Set(fields.parent_id),
        priority: Set(fields.priority),
        due_datetime: Set(fields.due_datetime),
        frequency: Set(fields.frequency),
        last_added_at: Set(fields.last_added_at),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<PeriodicTaskCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &PeriodicTaskDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = modal_from_form(
                q.form_name(),
                q.refresh_table(),
                &form,
                assigned_to_display,
                parent_display,
                choices,
                e.to_string(),
            );
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(row) = find_periodic_scoped(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let status_map = load_status_map(&state.db).await;
    let (status, status_color) = status_map
        .get(&row.status_id)
        .cloned()
        .unwrap_or_else(|| (format!("Status #{}", row.status_id), 0));
    let parent_id = row.parent_id.unwrap_or(0);
    let parent_title = task_display_label(&state.db, parent_id).await;
    let page = PeriodicTaskDetailPage {
        id: row.id,
        title: row.title.clone(),
        description: row.description,
        assigned_to: user_display_label(&state.db, row.assigned_to_id).await,
        parent_title,
        parent_href: row
            .parent_id
            .filter(|id| *id > 0)
            .map(|id| crate::routes::TaskDetailRouteTag::new(id).url())
            .unwrap_or_default(),
        status,
        status_color,
        priority: row.priority,
        due_datetime: ctx.format_datetime(row.due_datetime).into_string(),
        frequency: lariv_core::duration::format_duration(row.frequency),
        last_added_at: ctx.format_datetime(row.last_added_at).into_string(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(row) = find_periodic_scoped(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let page = PeriodicTaskEditModalPage {
        id: row.id,
        form_name: q.form_name(),
        title: row.title,
        description: row.description,
        assigned_to_id: row.assigned_to_id,
        assigned_to_display: user_display_label(&state.db, row.assigned_to_id).await,
        status_id: row.status_id.to_string(),
        status_choices: load_status_choices(&state.db).await,
        parent_id: row.parent_id.map(|id| id.to_string()).unwrap_or_default(),
        parent_display: task_display_label(&state.db, row.parent_id.unwrap_or(0)).await,
        priority: row.priority.to_string(),
        due_datetime: ctx.datetime_local_input(row.due_datetime).into_string(),
        frequency: lariv_core::duration::format_duration(row.frequency),
        last_added_at: ctx.datetime_local_input(row.last_added_at).into_string(),
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
    HtmlFormBody(form): HtmlFormBody<PeriodicTaskForm>,
) -> Response {
    let Some(existing) = find_periodic_scoped(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let choices = load_status_choices(&state.db).await;
    let (assigned_to_display, parent_display) = form_displays(&state.db, &form).await;
    let fields = match parse_fields(&state.db, &ctx, &form).await {
        Ok(fields) => fields,
        Err(error) => {
            let page = edit_modal_from_form(
                id,
                q.form_name(),
                &form,
                assigned_to_display,
                parent_display,
                choices,
                error,
            );
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let mut am: periodic_task::ActiveModel = existing.into();
    am.updated_at = Set(Some(Utc::now()));
    am.title = Set(fields.title);
    am.description = Set(fields.description);
    am.assigned_to_id = Set(fields.assigned_to_id);
    am.status_id = Set(fields.status_id);
    am.parent_id = Set(fields.parent_id);
    am.priority = Set(fields.priority);
    am.due_datetime = Set(fields.due_datetime);
    am.frequency = Set(fields.frequency);
    am.last_added_at = Set(fields.last_added_at);
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<PeriodicTaskEditModalKey>(
            &htmx,
            &PeriodicTaskDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = edit_modal_from_form(
                id,
                q.form_name(),
                &form,
                assigned_to_display,
                parent_display,
                choices,
                e.to_string(),
            );
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
        modal_uid: PeriodicTaskDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this periodic task?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_tasks.PeriodicTaskDeleteForm".into()),
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
    let Some(_) = find_periodic_scoped(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    match PeriodicTaskEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&list_url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete periodic task");
            let page = ConfirmDeletePage {
                modal_uid: PeriodicTaskDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this periodic task?".into(),
                form_name: "p_tasks.PeriodicTaskDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
