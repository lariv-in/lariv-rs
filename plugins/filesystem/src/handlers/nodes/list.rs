use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, sea_query::Condition};
use serde::Deserialize;

use crate::{
    entities::{
        VNode,
        filesystem_node::{Column, Entity as VNodeEntity},
    },
    keys::V_NODE_TABLE,
    node,
    permissions::AccessActor,
    routes::VNodeListRouteTag,
    state::FilesystemState,
    storage::DynFilestore,
    templates::{VNodeListPage, VNodeRow},
};
use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder};
use lariv_core::http::Cap;
use lariv_core::list_filters::{apply_filters, parse_list_filters};
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout};
use lariv_plugin_users::{middleware::RequireAuth, state::AuthContext};

use super::{
    format_updated_at, path_and_query, require_list_access, slot_ctx, vnode_filter_fields,
};

#[derive(Debug, Deserialize, Default)]
pub struct VNodeListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

async fn query_nodes(
    db: &DatabaseConnection,
    parent_id: Option<i64>,
    q: &VNodeListQuery,
    actor: &AccessActor,
    search_all: bool,
    condition: Condition,
) -> (Vec<VNode>, u32, u64) {
    let mut query = if search_all {
        VNodeEntity::find()
    } else {
        node::apply_listing_parent(VNodeEntity::find(), parent_id, "")
    };
    query = query.filter(condition);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query
            .order_by_desc(Column::IsDirectory)
            .order_by_desc(Column::Name),
        s if s.eq_ignore_ascii_case("Type DESC") => query.order_by_desc(Column::IsDirectory),
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(Column::IsDirectory)
        }
        s if s.eq_ignore_ascii_case("Modified DESC") => query
            .order_by_desc(Column::IsDirectory)
            .order_by_desc(Column::UpdatedAt),
        s if s.eq_ignore_ascii_case("Modified ASC") || s.eq_ignore_ascii_case("Modified") => query
            .order_by_desc(Column::IsDirectory)
            .order_by_asc(Column::UpdatedAt),
        // Name ASC / default: directories first, then name ascending.
        _ => query
            .order_by_desc(Column::IsDirectory)
            .order_by_asc(Column::Name),
    };

    let visible: Vec<VNode> = query
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|node| node::authorize_view(node, actor).is_ok())
        .collect();
    let page = q.page.get();
    let page_size = q.page_size.get().max(1) as usize;
    let total = visible.len() as u64;
    let start = (page as usize).saturating_sub(1).saturating_mul(page_size);
    let models = visible.into_iter().skip(start).take(page_size).collect();
    (models, page, total)
}

async fn load_list_page(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
    q: &VNodeListQuery,
    tz: &str,
    actor: &AccessActor,
    search_all: bool,
    condition: Condition,
) -> ObjectList<VNodeRow> {
    let (models, page, total) = query_nodes(db, parent_id, q, actor, search_all, condition).await;
    let mut rows = Vec::with_capacity(models.len());
    for n in models {
        let size_display = node::file_size_display(store, &n).await;
        let items_display = if n.is_directory {
            node::count_visible_children(db, n.id, actor)
                .await
                .unwrap_or(0)
                .to_string()
        } else {
            "-".to_string()
        };
        rows.push(VNodeRow {
            id: n.id,
            name: n.name.clone(),
            is_directory: n.is_directory,
            size_display,
            items_display,
            updated_at: format_updated_at(n.updated_at, tz),
        });
    }
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

async fn render_list_layered(
    state: FilesystemState,
    auth: AuthContext,
    chrome: SharedChromeFolder,
    htmx: Htmx,
    uri: Uri,
    q: VNodeListQuery,
    parent_id: Option<i64>,
) -> Response {
    // Equivalent to vnode_list/browse_layers(); avoid run_layers in Route handlers (rustc #100013).
    use crate::layers::VNodeListData;
    let parent = match parent_id {
        Some(id) => {
            match lariv_core::web::opt_or_log(
                node::get_by_id(&state.db, id).await,
                "get node by id",
            ) {
                Some(n) if n.is_directory => Some(n),
                _ => return Redirect::to(&VNodeListRouteTag.url()).into_response(),
            }
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
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    let items = load_list_page(
        &state.db,
        state.store.as_ref(),
        parent_id,
        &q,
        &auth.timezone,
        &actor,
        search_all,
        applied.condition,
    )
    .await;
    let list_page = VNodeListPage {
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_name: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        items,
        fields,
        filters,
        filter_errors: applied.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let _ = VNodeListData {
        parent_id: list_page.parent_id,
        parent_name: list_page.parent_name.clone(),
        items: list_page.items.clone(),
        filters: list_page.filters.clone(),
        filter_errors: list_page.filter_errors.clone(),
        sort: list_page.sort.clone(),
        path_and_query: list_page.path_and_query.clone(),
        page_size: list_page.page_size,
    };
    if htmx.targets(V_NODE_TABLE) {
        return list_page.render_table().into_response();
    }
    html_built_page_or_app_layout(&list_page, &htmx, &chrome, &slot_ctx(&auth)).into_response()
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeListQuery>,
) -> Response {
    render_list_layered(state, auth, chrome, htmx, uri, q, None).await
}

/// HTTP handler: `browse`.
pub async fn browse(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeListQuery>,
    Path(parent_id): Path<i64>,
) -> Response {
    render_list_layered(state, auth, chrome, htmx, uri, q, Some(parent_id)).await
}
