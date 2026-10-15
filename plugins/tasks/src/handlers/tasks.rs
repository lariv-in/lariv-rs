use axum::{
    extract::{Path, Query},
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::Condition;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect,
};

use lariv_core::components::{
    FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx, SwapKey, table_rows_instance_id,
};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    FilterField, apply_filters, choice_i64_column, foreign_key_column, parse_list_filters,
    text_column, with_search_key,
};
use lariv_core::picker::respond_picker_select;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryI64, QueryPage, QueryPageSize, html_built_page_or_app_layout,
    html_built_page_with_slots, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::routes::UsersSelectRouteTag;
use lariv_plugin_users::{middleware::RequireAuth, roles::Superuser, state::AuthContext};

use crate::{
    color::{STATUS_TODO, status_name_from_slug},
    entities::task::{self, Entity as TaskEntity},
    forms::TaskForm,
    handlers::ModalNameQuery,
    keys::{
        TaskChildrenTableKey, TaskCreateModalKey, TaskDeleteModalKey, TaskEditModalKey,
        TaskSelectModalKey, TaskSelectTableKey, TaskTableKey,
    },
    logic::task::{
        TaskFields, delete_task, set_task_priority, set_task_status, stepped_priority, update_task,
        validate_parent,
    },
    routes::{TaskDefaultRouteTag, TaskDetailRouteTag},
    scope::{
        apply_task_sort, effective_task_sort, find_status_by_name, find_task_scoped,
        load_status_choices, load_status_map, task_display_label, user_display_label, user_exists,
    },
    state::TasksState,
    templates::{
        ConfirmDeletePage, TaskCreateModalPage, TaskDetailPage, TaskEditModalPage, TaskListPage,
        TaskOption, TaskRow, TaskSelectPage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaskCreateQuery {
    #[serde(flatten)]
    pub modal: ModalNameQuery,
    #[serde(default, rename = "ParentID", alias = "parent_id")]
    pub parent_id: QueryI64,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaskHubQuery {
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

/// The assignee cannot change the task through the edit form.
pub(crate) fn may_edit_task(user_id: i64, assigned_to_id: i64) -> bool {
    user_id != assigned_to_id
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

pub(crate) fn parse_optional_parent(raw: &str) -> Result<Option<i64>, &'static str> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    match raw.parse::<i64>() {
        Ok(id) if id > 0 => Ok(Some(id)),
        _ => Err("invalid parent task"),
    }
}

pub(crate) fn parse_priority(s: &str) -> Result<i32, &'static str> {
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
    parent_id: Option<i64>,
    condition: Condition,
    restrict_to_user: Option<i64>,
) -> (Vec<TaskRow>, u32, u64) {
    let sort = effective_task_sort(q.sort.as_deref());
    let page = q.page.get().max(1);
    let parents = visible_parent_map(db, restrict_to_user).await;
    let matches = load_scoped_tasks(db, condition, restrict_to_user).await;
    let stop_at = parent_id.filter(|id| *id > 0);
    let match_ids: Vec<i64> = matches.iter().map(|task| task.id).collect();
    let wanted = subtree_ids(&match_ids, &parents, stop_at);
    let mut by_id: std::collections::HashMap<i64, task::Model> = matches
        .into_iter()
        .filter(|task| wanted.contains(&task.id))
        .map(|task| (task.id, task))
        .collect();
    let missing: Vec<i64> = wanted
        .iter()
        .copied()
        .filter(|id| !by_id.contains_key(id))
        .collect();
    if !missing.is_empty() {
        for task in load_tasks_by_ids(db, &missing, restrict_to_user).await {
            by_id.insert(task.id, task);
        }
    }
    let status_map = load_status_map(db).await;
    let mut nodes: std::collections::HashMap<i64, TaskNode> = by_id
        .into_iter()
        .map(|(_, task)| (task.id, task_node(task, &status_map, auth)))
        .collect();
    fill_node_labels(db, &mut nodes).await;
    page_task_forest(nodes, &parents, &sort, page, page_size)
}

async fn visible_parent_map(
    db: &sea_orm::DatabaseConnection,
    restrict_to_user: Option<i64>,
) -> std::collections::HashMap<i64, Option<i64>> {
    let mut query = scope_allowed::<super::super::routes::TasksView, _>(TaskEntity::find());
    if let Some(uid) = restrict_to_user {
        query = query.filter(task::Column::AssignedToId.eq(uid));
    }
    let pairs: Vec<(i64, Option<i64>)> = query
        .select_only()
        .column(task::Column::Id)
        .column(task::Column::ParentId)
        .into_tuple()
        .all(db)
        .await
        .unwrap_or_default();
    pairs
        .into_iter()
        .map(|(id, parent)| (id, parent.filter(|parent_id| *parent_id > 0)))
        .collect()
}

async fn load_scoped_tasks(
    db: &sea_orm::DatabaseConnection,
    condition: Condition,
    restrict_to_user: Option<i64>,
) -> Vec<task::Model> {
    let mut query = scope_allowed::<super::super::routes::TasksView, _>(TaskEntity::find());
    query = query.filter(condition);
    if let Some(uid) = restrict_to_user {
        query = query.filter(task::Column::AssignedToId.eq(uid));
    }
    query.all(db).await.unwrap_or_default()
}

async fn load_tasks_by_ids(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
    restrict_to_user: Option<i64>,
) -> Vec<task::Model> {
    if ids.is_empty() {
        return Vec::new();
    }
    let mut query = scope_allowed::<super::super::routes::TasksView, _>(TaskEntity::find());
    query = query.filter(task::Column::Id.is_in(ids.iter().copied()));
    if let Some(uid) = restrict_to_user {
        query = query.filter(task::Column::AssignedToId.eq(uid));
    }
    query.all(db).await.unwrap_or_default()
}

struct TaskNode {
    row: TaskRow,
    due_at: chrono::DateTime<chrono::Utc>,
}

fn task_node(
    task: task::Model,
    status_map: &std::collections::HashMap<i64, (String, u32)>,
    auth: &AuthContext,
) -> TaskNode {
    let (status_name, status_color) = status_map
        .get(&task.status_id)
        .cloned()
        .unwrap_or_else(|| (format!("Status #{}", task.status_id), 0));
    TaskNode {
        due_at: task.due_datetime,
        row: TaskRow {
            id: task.id,
            title: task.title,
            assigned_to: String::new(),
            assigned_to_id: task.assigned_to_id,
            status: status_name,
            status_color,
            priority: task.priority,
            due_datetime: auth.format_datetime(task.due_datetime).into_string(),
            detail_href: TaskDetailRouteTag::new(task.id).url(),
            children: Vec::new(),
        },
    }
}

async fn fill_node_labels(
    db: &sea_orm::DatabaseConnection,
    nodes: &mut std::collections::HashMap<i64, TaskNode>,
) {
    let mut cache = std::collections::HashMap::<i64, String>::new();
    for node in nodes.values_mut() {
        let assignee = node.row.assigned_to_id;
        if let Some(label) = cache.get(&assignee) {
            node.row.assigned_to = label.clone();
            continue;
        }
        let label = user_display_label(db, assignee).await;
        node.row.assigned_to = label.clone();
        cache.insert(assignee, label);
    }
}

/// Match ids plus the visible ancestors needed to hang them in the tree.
///
/// `stop_at` is the detail-page task: it is not included, and matches that
/// do not descend from it are dropped.
fn subtree_ids(
    match_ids: &[i64],
    parents: &std::collections::HashMap<i64, Option<i64>>,
    stop_at: Option<i64>,
) -> std::collections::HashSet<i64> {
    let mut out = std::collections::HashSet::new();
    for &start in match_ids {
        if !parents.contains_key(&start) {
            continue;
        }
        if let Some(stop) = stop_at
            && (start == stop || !reaches_parent(start, stop, parents))
        {
            continue;
        }
        let mut cur = start;
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert(cur) || stop_at == Some(cur) || !parents.contains_key(&cur) {
                break;
            }
            out.insert(cur);
            let Some(parent) = parents.get(&cur).copied().flatten() else {
                break;
            };
            if stop_at == Some(parent) || !parents.contains_key(&parent) {
                break;
            }
            cur = parent;
        }
    }
    out
}

fn reaches_parent(
    start: i64,
    stop: i64,
    parents: &std::collections::HashMap<i64, Option<i64>>,
) -> bool {
    let mut cur = start;
    let mut seen = std::collections::HashSet::new();
    loop {
        if !seen.insert(cur) {
            return false;
        }
        if cur == stop {
            return true;
        }
        let Some(parent) = parents.get(&cur).copied().flatten() else {
            return false;
        };
        if parent != stop && !parents.contains_key(&parent) {
            return false;
        }
        cur = parent;
    }
}

fn page_task_forest(
    nodes: std::collections::HashMap<i64, TaskNode>,
    parents: &std::collections::HashMap<i64, Option<i64>>,
    sort: &str,
    page: u32,
    page_size: u32,
) -> (Vec<TaskRow>, u32, u64) {
    let (roots, child_ids) = sorted_child_lists(&nodes, parents, sort);
    let total = roots.len() as u64;
    let page_size = page_size.max(1) as usize;
    let start = (page.max(1) - 1) as usize * page_size;
    let page_roots: Vec<i64> = roots.into_iter().skip(start).take(page_size).collect();
    (
        assemble(&page_roots, &nodes, &child_ids),
        page.max(1),
        total,
    )
}

fn sorted_child_lists(
    nodes: &std::collections::HashMap<i64, TaskNode>,
    parents: &std::collections::HashMap<i64, Option<i64>>,
    sort: &str,
) -> (Vec<i64>, std::collections::HashMap<i64, Vec<i64>>) {
    let mut children: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    let mut roots = Vec::new();
    for &id in nodes.keys() {
        match nearest_loaded_parent(id, nodes, parents) {
            Some(parent) if parent != id && !chain_reaches(parent, id, parents) => {
                children.entry(parent).or_default().push(id);
            }
            _ => roots.push(id),
        }
    }
    sort_ids(&mut roots, nodes, sort);
    for kids in children.values_mut() {
        sort_ids(kids, nodes, sort);
    }
    (roots, children)
}

fn chain_reaches(
    start: i64,
    target: i64,
    parents: &std::collections::HashMap<i64, Option<i64>>,
) -> bool {
    let mut cur = start;
    let mut seen = std::collections::HashSet::new();
    while seen.insert(cur) {
        if cur == target {
            return true;
        }
        cur = match parents.get(&cur).copied().flatten() {
            Some(parent) => parent,
            None => return false,
        };
    }
    true
}

fn nearest_loaded_parent(
    id: i64,
    nodes: &std::collections::HashMap<i64, TaskNode>,
    parents: &std::collections::HashMap<i64, Option<i64>>,
) -> Option<i64> {
    let mut cur = parents.get(&id).copied().flatten()?;
    let mut seen = std::collections::HashSet::new();
    loop {
        if !seen.insert(cur) {
            return None;
        }
        if nodes.contains_key(&cur) {
            return Some(cur);
        }
        cur = parents.get(&cur).copied().flatten()?;
    }
}

fn sort_ids(ids: &mut [i64], nodes: &std::collections::HashMap<i64, TaskNode>, sort: &str) {
    let key = sort.split_whitespace().next().unwrap_or("");
    let desc = sort
        .split_whitespace()
        .last()
        .is_some_and(|dir| dir.eq_ignore_ascii_case("DESC"));
    ids.sort_by(|a, b| {
        let (Some(left), Some(right)) = (nodes.get(a), nodes.get(b)) else {
            return a.cmp(b);
        };
        let ord = if key.eq_ignore_ascii_case("Title") {
            left.row.title.cmp(&right.row.title)
        } else if key.eq_ignore_ascii_case("AssignedTo") {
            left.row.assigned_to.cmp(&right.row.assigned_to)
        } else if key.eq_ignore_ascii_case("Status") {
            left.row.status.cmp(&right.row.status)
        } else if key.eq_ignore_ascii_case("Priority") {
            left.row.priority.cmp(&right.row.priority)
        } else {
            left.due_at.cmp(&right.due_at)
        };
        if desc { ord.reverse() } else { ord }.then(a.cmp(b))
    });
}

fn assemble(
    ids: &[i64],
    nodes: &std::collections::HashMap<i64, TaskNode>,
    child_ids: &std::collections::HashMap<i64, Vec<i64>>,
) -> Vec<TaskRow> {
    fn walk(
        ids: &[i64],
        nodes: &std::collections::HashMap<i64, TaskNode>,
        child_ids: &std::collections::HashMap<i64, Vec<i64>>,
        seen: &mut std::collections::HashSet<i64>,
    ) -> Vec<TaskRow> {
        ids.iter()
            .filter_map(|id| {
                if !seen.insert(*id) {
                    return None;
                }
                let node = nodes.get(id)?;
                let mut row = node.row.clone();
                let kids = child_ids.get(id).map(Vec::as_slice).unwrap_or(&[]);
                row.children = walk(kids, nodes, child_ids, seen);
                Some(row)
            })
            .collect()
    }
    walk(ids, nodes, child_ids, &mut std::collections::HashSet::new())
}

fn query_string(path_and_query: &str) -> &str {
    path_and_query.split_once('?').map(|(_, q)| q).unwrap_or("")
}

fn task_fields(any_assignee: bool, status_choices: Vec<(String, String)>) -> Vec<FilterField> {
    let mut fields = vec![text_column("title", "Title", task::Column::Title)];
    if any_assignee {
        fields.push(with_search_key(
            foreign_key_column(
                "assigned_to_id",
                "Assigned To",
                task::Column::AssignedToId,
                format!("{}?filter.0.field=name", UsersSelectRouteTag.url()),
                "Any user…",
            ),
            "filter.0.value",
        ));
    }
    fields.push(choice_i64_column(
        "status_id",
        "Status",
        task::Column::StatusId,
        status_choices,
    ));
    fields
}

async fn fill_assignee_display(db: &sea_orm::DatabaseConnection, rows: &mut [FilterClauseView]) {
    for row in rows {
        if row.clause.field != "assigned_to_id" {
            continue;
        }
        let Ok(id) = row.clause.value().trim().parse::<i64>() else {
            continue;
        };
        if id > 0 {
            row.fk_display = user_display_label(db, id).await;
        }
    }
}

async fn task_list_page(
    db: &sea_orm::DatabaseConnection,
    ctx: &AuthContext,
    q: &TaskHubQuery,
    path_and_query: String,
    parent_id: Option<i64>,
) -> TaskListPage {
    let clauses = parse_list_filters(query_string(&path_and_query));
    let any_assignee = sees_every_task(&ctx.role);
    let fields = task_fields(any_assignee, load_status_choices(db).await);
    let applied = apply_filters(db.get_database_backend(), &fields, &clauses);
    let mut filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    fill_assignee_display(db, &mut filters).await;
    let restrict = if any_assignee {
        None
    } else {
        Some(ctx.user.id)
    };
    let (rows, page, total) = query_tasks(
        db,
        q,
        ctx,
        q.page_size.get(),
        parent_id,
        applied.condition,
        restrict,
    )
    .await;
    let tasks = ObjectList::from_page(rows, page, q.page_size.get(), total);
    TaskListPage {
        tasks,
        fields,
        filters,
        filter_errors: applied.errors,
        sort: effective_task_sort(q.sort.as_deref()),
        path_and_query,
        page_size: q.page_size.get(),
        parent_task_id: parent_id.filter(|id| *id > 0).unwrap_or(0),
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
    children: TaskListPage,
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
    TaskDetailPage {
        id: task.id,
        title: task.title,
        description: task.description,
        assigned_to: user_display_label(db, task.assigned_to_id).await,
        parent_title,
        parent_href,
        children,
        status: status_name,
        status_color,
        can_set_status: may_set_status(&ctx.role, ctx.user.id, task.assigned_to_id),
        can_edit: may_edit_task(ctx.user.id, task.assigned_to_id),
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
    let page = task_list_page(&state.db, &ctx, &q, path_and_query(&uri), None).await;
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
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<TaskHubQuery>,
) -> Response {
    let Some(task) = find_task_scoped(&state.db, id).await else {
        return Redirect::to(&TaskDefaultRouteTag.url()).into_response();
    };
    let children = task_list_page(&state.db, &ctx, &q, path_and_query(&uri), Some(task.id)).await;
    let page = task_detail_page(&state.db, &ctx, task, children).await;
    if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref()) {
        if TaskChildrenTableKey::matches_id(instance) {
            return page.children.render_table_rows(instance).into_response();
        }
    }
    if htmx.targets::<TaskChildrenTableKey>() {
        return page.children.render_table().into_response();
    }
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
    let parent_id = task.parent_id.filter(|id| *id > 0);
    let Some(status) = find_status_by_name(&state.db, name).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if task.status_id != status.id
        && let Err(e) = set_task_status(&state.db, task, status.id, &ctx).await
    {
        tracing::error!(error = %e, id, "failed to set task status");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    respond_after_inline_task_change(&state.db, &chrome, &ctx, &htmx, id, parent_id).await
}

pub async fn set_priority(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path((id, direction)): Path<(i64, String)>,
) -> Response {
    let Some(task) = find_task_scoped(&state.db, id).await else {
        return Redirect::to(&TaskDefaultRouteTag.url()).into_response();
    };
    let Some(next) = stepped_priority(task.priority, &direction) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let parent_id = task.parent_id.filter(|pid| *pid > 0);
    if next != task.priority
        && let Err(e) = set_task_priority(&state.db, task, next, &ctx).await
    {
        tracing::error!(error = %e, id, "failed to set task priority");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    respond_after_inline_task_change(&state.db, &chrome, &ctx, &htmx, id, parent_id).await
}

async fn respond_after_inline_task_change(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    htmx: &Htmx,
    task_id: i64,
    parent_id: Option<i64>,
) -> Response {
    if htmx.targets::<TaskTableKey>() {
        let (path_and_query, q) = list_query_from_browser(htmx);
        let page = task_list_page(db, ctx, &q, path_and_query, None).await;
        if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref()) {
            if TaskTableKey::matches_id(instance) {
                return page.render_table_rows(instance).into_response();
            }
        }
        return page.render_table().into_response();
    }
    if htmx.targets::<TaskChildrenTableKey>()
        && let Some(parent_id) = parent_id
    {
        let (path_and_query, q) = list_query_from_browser(htmx);
        let page = task_list_page(db, ctx, &q, path_and_query, Some(parent_id)).await;
        if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref()) {
            if TaskChildrenTableKey::matches_id(instance) {
                return page.render_table_rows(instance).into_response();
            }
        }
        return page.render_table().into_response();
    }
    if !htmx.request {
        return Redirect::to(&TaskDetailRouteTag::new(task_id).url()).into_response();
    }
    let Some(task) = find_task_scoped(db, task_id).await else {
        return Redirect::to(&TaskDefaultRouteTag.url()).into_response();
    };
    let children = task_list_page(
        db,
        ctx,
        &TaskHubQuery::default(),
        TaskDetailRouteTag::new(task_id).path(),
        Some(task_id),
    )
    .await;
    let page = task_detail_page(db, ctx, task, children).await;
    html_built_page_or_app_layout(&page, htmx, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<TasksState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<TaskCreateQuery>,
) -> maud::Markup {
    let (parent_id, parent_display) = match q.parent_id.positive() {
        Some(pid) => {
            let label = task_display_label(&state.db, pid).await;
            if label.is_empty() {
                (String::new(), String::new())
            } else {
                (pid.to_string(), label)
            }
        }
        None => (String::new(), String::new()),
    };
    let page = TaskCreateModalPage {
        form_name: q.modal.form_name(),
        refresh_table: q.modal.refresh_table(),
        title: String::new(),
        description: String::new(),
        assigned_to_id: ctx.user.id,
        assigned_to_display: ctx.user.name.clone(),
        parent_id,
        parent_display,
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
        Ok(saved) => {
            crate::push::notify_assignee(&state.db, &state.push, &saved).await;
            respond_create_modal_done::<TaskCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &TaskDetailRouteTag::new(saved.id).url(),
            )
        }
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
    if !may_edit_task(ctx.user.id, task.assigned_to_id) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
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
    if !may_edit_task(ctx.user.id, existing.assigned_to_id) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
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
    let previous_assignee = existing.assigned_to_id;
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
        Ok(saved) => {
            if previous_assignee != saved.assigned_to_id {
                crate::push::notify_assignee(&state.db, &state.push, &saved).await;
            }
            respond_edit_modal_done::<TaskEditModalKey>(&htmx, &TaskDetailRouteTag::new(id).url())
        }
        Err(e) => {
            task_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e.to_string()).await
        }
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaskSelectQuery {
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
    let clauses = parse_list_filters(uri.query().unwrap_or(""));
    let fields = vec![text_column("title", "Title", task::Column::Title)];
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    let mut query = scope_allowed::<super::super::routes::TasksView, _>(TaskEntity::find());
    query = query.filter(applied.condition);
    if let Some(uid) = visible_assignee_filter(&ctx.role, ctx.user.id, None) {
        query = query.filter(task::Column::AssignedToId.eq(uid));
    }
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
        fields,
        filters,
        filter_errors: applied.errors,
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
        assigned_to_filter, may_edit_task, may_set_status, sees_every_task, visible_assignee_filter,
    };
    use lariv_core::list_filters::{CmpOp, parse_list_filters};
    use lariv_plugin_users::roles::Superuser;

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
    fn the_assignee_cannot_edit_the_task() {
        assert!(!may_edit_task(7, 7));
        assert!(may_edit_task(1, 7));
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
    fn filter_query_accepts_the_indexed_field_name() {
        let clauses = parse_list_filters(
            "filter.0.field=assigned_to_id&filter.0.op=eq&filter.0.value=12&filter.1.field=status_id&filter.1.op=eq&filter.1.value=3",
        );
        assert_eq!(clauses[0].field, "assigned_to_id");
        assert_eq!(clauses[0].op, CmpOp::Eq);
        assert_eq!(clauses[0].value(), "12");
        assert_eq!(clauses[1].field, "status_id");
        assert_eq!(clauses[1].value(), "3");
    }

    fn sample_node(id: i64, title: &str, priority: i32) -> super::TaskNode {
        super::TaskNode {
            due_at: chrono::DateTime::from_timestamp(id, 0).unwrap_or_default(),
            row: super::TaskRow {
                id,
                title: title.into(),
                assigned_to: String::new(),
                assigned_to_id: 0,
                status: "Todo".into(),
                status_color: 0,
                priority,
                due_datetime: String::new(),
                detail_href: format!("/tasks/{id}"),
                children: Vec::new(),
            },
        }
    }

    #[test]
    fn forest_nests_descendants_and_pages_roots() {
        let mut parents = std::collections::HashMap::new();
        parents.insert(1, None);
        parents.insert(2, Some(1));
        parents.insert(3, Some(2));
        parents.insert(4, None);
        let wanted = super::subtree_ids(&[1, 2, 3, 4], &parents, None);
        assert_eq!(wanted.len(), 4);

        let nodes = [1, 2, 3, 4]
            .into_iter()
            .map(|id| {
                (
                    id,
                    sample_node(id, &format!("T{id}"), if id == 4 { 1 } else { 5 }),
                )
            })
            .collect();
        let (rows, page, total) =
            super::page_task_forest(nodes, &parents, "DueDatetime ASC", 1, 12);
        assert_eq!(page, 1);
        assert_eq!(total, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, 1);
        assert_eq!(rows[0].children.len(), 1);
        assert_eq!(rows[0].children[0].id, 2);
        assert_eq!(rows[0].children[0].children[0].id, 3);
        assert_eq!(rows[1].id, 4);

        let detail = super::subtree_ids(&[1, 2, 3, 4], &parents, Some(1));
        assert!(!detail.contains(&1));
        assert!(detail.contains(&2) && detail.contains(&3));
        assert!(!detail.contains(&4));

        let nodes = [2, 3]
            .into_iter()
            .map(|id| (id, sample_node(id, &format!("T{id}"), 0)))
            .collect();
        let (rows, _, total) = super::page_task_forest(nodes, &parents, "Title ASC", 1, 12);
        assert_eq!(total, 1);
        assert_eq!(rows[0].id, 2);
        assert_eq!(rows[0].children[0].id, 3);
    }

    #[test]
    fn a_matching_child_keeps_its_visible_ancestors() {
        let mut parents = std::collections::HashMap::new();
        parents.insert(1, None);
        parents.insert(2, Some(1));
        parents.insert(3, Some(2));
        let wanted = super::subtree_ids(&[3], &parents, None);
        assert_eq!(wanted, [1, 2, 3].into_iter().collect());
    }

    #[test]
    fn a_parent_cycle_does_not_nest_forever() {
        let mut parents = std::collections::HashMap::new();
        parents.insert(5, Some(6));
        parents.insert(6, Some(5));
        let nodes = [5, 6]
            .into_iter()
            .map(|id| (id, sample_node(id, &format!("T{id}"), 0)))
            .collect();
        let (rows, _, total) = super::page_task_forest(nodes, &parents, "Title ASC", 1, 12);
        assert_eq!(total, 2);
        assert!(rows.iter().all(|row| row.children.is_empty()));
    }
}
