use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use crate::{
    entities::filesystem_node::{Column, Entity as VNodeEntity},
    keys::{V_NODE_SELECT_MODAL, V_NODE_SELECT_TABLE},
    node,
    permissions::AccessActor,
    routes::{
        VNodeFileSelectRouteTag, VNodeMoveSelectRouteTag, VNodeNodeSelectRouteTag,
        VNodeSelectRouteTag,
    },
    state::FilesystemState,
    templates::{VNodeOption, VNodeSelectPage},
};
use lariv_core::components::picker::respond_picker_select;
use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder};
use lariv_core::http::Cap;
use lariv_core::list_filters::{apply_filters, parse_list_filters};
use lariv_core::web::{Htmx, QueryI64, QueryPageSize, query_bool};
use lariv_plugin_users::{middleware::RequireAuth, state::AuthContext};

use super::{path_and_query, require_list_access, vnode_filter_fields};

#[derive(Debug, Deserialize, Default)]
pub struct VNodeSelectQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub target_input: Option<String>,
    #[serde(default)]
    pub exclude_id: QueryI64,
    /// When true, row clicks toggle via `fk-multi-select` and keep the modal open.
    #[serde(default, deserialize_with = "query_bool")]
    pub multi: Option<bool>,
}

#[allow(
    clippy::too_many_arguments,
    reason = "internal fan-in for select route handlers"
)]
async fn render_select(
    state: FilesystemState,
    auth: AuthContext,
    htmx: Htmx,
    uri: Uri,
    q: VNodeSelectQuery,
    parent_id: Option<i64>,
    browse_base: &str,
    default_target_input: &str,
    only_directories: bool,
    select_any: bool,
) -> Response {
    let parent = match parent_id {
        Some(id) => {
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        }
        None => None,
    };
    if let Err(response) = require_list_access(&state.db, parent_id, parent.as_ref(), &auth).await {
        return response;
    }
    let actor = AccessActor::from_auth(&auth);
    let fields = vnode_filter_fields();
    let clauses = parse_list_filters(uri.query().unwrap_or(""));
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let search_all = clauses.iter().any(|clause| clause.field == "name");
    let mut query = if search_all {
        VNodeEntity::find()
    } else {
        node::apply_listing_parent(VNodeEntity::find(), parent_id, "")
    };
    if only_directories {
        query = query.filter(Column::IsDirectory.eq(true));
    }
    let mut children = query
        .filter(applied.condition)
        .order_by_desc(Column::IsDirectory)
        .order_by_asc(Column::Name)
        .all(&state.db)
        .await
        .unwrap_or_default();
    children.retain(|child| node::authorize_view(child, &actor).is_ok());
    let sort = q.sort.as_deref().unwrap_or("").trim();
    if sort.eq_ignore_ascii_case("Name DESC") {
        children.sort_by(|a, b| b.name.cmp(&a.name));
    } else {
        children.sort_by(|a, b| a.name.cmp(&b.name));
    }
    let total = children.len() as u64;
    let options: Vec<VNodeOption> = children
        .into_iter()
        .map(|n| VNodeOption {
            id: n.id,
            name: n.name,
            is_directory: n.is_directory,
        })
        .collect();
    let page_size = (total.max(1) as u32).min(500);
    let items = ObjectList::from_page(options, 1, page_size, total);
    let current_path = match &parent {
        Some(p) => node::get_path(&state.db, p).await,
        None => "/".to_string(),
    };
    let page = VNodeSelectPage {
        items,
        fields,
        filters: FilterClauseView::from_clauses(&clauses, &applied.errors),
        filter_errors: applied.errors,
        target_input: q
            .target_input
            .clone()
            .unwrap_or_else(|| default_target_input.to_string()),
        browse_base: browse_base.to_string(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        up_parent_id: parent.as_ref().and_then(|p| p.parent_id).unwrap_or(0),
        current_path,
        exclude_id: q.exclude_id.or_zero(),
        only_directories,
        select_any,
        multi: q.multi.unwrap_or(false),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select(&htmx, V_NODE_SELECT_TABLE, V_NODE_SELECT_MODAL, &page).into_response()
}

/// HTTP handler: `select`.
pub async fn select(
    Cap(state): Cap<FilesystemState>,
    Cap(_chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
) -> Response {
    render_select(
        state,
        ctx,
        htmx,
        uri,
        q,
        None,
        &VNodeSelectRouteTag.path(),
        "ParentID",
        true,
        false,
    )
    .await
}

/// HTTP handler: `select_in`.
pub async fn select_in(
    Cap(state): Cap<FilesystemState>,
    Cap(_chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
    Path(parent_id): Path<i64>,
) -> Response {
    render_select(
        state,
        ctx,
        htmx,
        uri,
        q,
        Some(parent_id),
        &VNodeSelectRouteTag.path(),
        "ParentID",
        true,
        false,
    )
    .await
}

/// HTTP handler: `move_select`.
pub async fn move_select(
    Cap(state): Cap<FilesystemState>,
    Cap(_chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
) -> Response {
    render_select(
        state,
        ctx,
        htmx,
        uri,
        q,
        None,
        &VNodeMoveSelectRouteTag.path(),
        "DestinationID",
        true,
        false,
    )
    .await
}

/// HTTP handler: `move_select_in`.
pub async fn move_select_in(
    Cap(state): Cap<FilesystemState>,
    Cap(_chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
    Path(parent_id): Path<i64>,
) -> Response {
    render_select(
        state,
        ctx,
        htmx,
        uri,
        q,
        Some(parent_id),
        &VNodeMoveSelectRouteTag.path(),
        "DestinationID",
        true,
        false,
    )
    .await
}

/// HTTP handler: `file_select`.
pub async fn file_select(
    Cap(state): Cap<FilesystemState>,
    Cap(_chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
) -> Response {
    render_select(
        state,
        ctx,
        htmx,
        uri,
        q,
        None,
        &VNodeFileSelectRouteTag.path(),
        "PageID",
        false,
        false,
    )
    .await
}

/// HTTP handler: `file_select_in`.
pub async fn file_select_in(
    Cap(state): Cap<FilesystemState>,
    Cap(_chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
    Path(parent_id): Path<i64>,
) -> Response {
    render_select(
        state,
        ctx,
        htmx,
        uri,
        q,
        Some(parent_id),
        &VNodeFileSelectRouteTag.path(),
        "PageID",
        false,
        false,
    )
    .await
}

/// HTTP handler: `node_select`. Files and folders are both selectable.
pub async fn node_select(
    Cap(state): Cap<FilesystemState>,
    Cap(_chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
) -> Response {
    render_select(
        state,
        ctx,
        htmx,
        uri,
        q,
        None,
        &VNodeNodeSelectRouteTag.path(),
        "FolderVnodeID",
        false,
        true,
    )
    .await
}

/// HTTP handler: `node_select_in`.
pub async fn node_select_in(
    Cap(state): Cap<FilesystemState>,
    Cap(_chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
    Path(parent_id): Path<i64>,
) -> Response {
    render_select(
        state,
        ctx,
        htmx,
        uri,
        q,
        Some(parent_id),
        &VNodeNodeSelectRouteTag.path(),
        "FolderVnodeID",
        false,
        true,
    )
    .await
}
