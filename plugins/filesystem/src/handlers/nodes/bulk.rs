use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::{
    keys::V_NODE_BULK_DELETE_MODAL,
    node,
    permissions::AccessActor,
    routes::VNodeListRouteTag,
    state::FilesystemState,
    templates::{VNodeBulkMoveFormPage, VNodeConfirmBulkDeletePage},
    zip,
};
use lariv_core::components::SharedChromeFolder;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots};
use lariv_plugin_users::middleware::RequireAuth;

use super::download::zip_response;
use super::{forbidden, slot_ctx};

#[derive(Debug, Deserialize, Default)]
pub struct BulkIdsQuery {
    #[serde(default)]
    pub ids: Option<String>,
    #[serde(default, rename = "return")]
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct BulkIdsForm {
    #[serde(default)]
    pub ids: String,
    #[serde(default, rename = "return")]
    pub return_to: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct BulkMovePostForm {
    #[serde(default)]
    pub ids: String,
    #[serde(default, rename = "return")]
    pub return_to: String,
    #[serde(default, rename = "DestinationID")]
    pub destination_id: i64,
}

fn parse_bulk_ids(raw: &str) -> Vec<i64> {
    let mut ids: Vec<i64> = raw
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn under_list_path(candidate: &str, list_path: &str) -> bool {
    let path = candidate.split('?').next().unwrap_or(candidate);
    path == list_path || path.starts_with(&format!("{list_path}/"))
}

fn safe_return_to(raw: &str) -> String {
    let trimmed = raw.trim();
    let list_path = VNodeListRouteTag.path();
    if under_list_path(trimmed, &list_path) && !trimmed.starts_with("//") {
        trimmed.to_string()
    } else {
        VNodeListRouteTag.url()
    }
}

fn bulk_delete_message(count: usize) -> String {
    if count == 1 {
        "Are you sure you want to delete the selected item? Directories delete everything inside them.".into()
    } else {
        format!(
            "Are you sure you want to delete {count} selected items? Directories delete everything inside them."
        )
    }
}

fn ids_csv(ids: &[i64]) -> String {
    ids.iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// HTTP handler: `bulk_delete_get`.
pub async fn bulk_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkIdsQuery>,
) -> maud::Markup {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let return_to = safe_return_to(q.return_to.as_deref().unwrap_or(&VNodeListRouteTag.path()));
    let page = VNodeConfirmBulkDeletePage {
        modal_uid: V_NODE_BULK_DELETE_MODAL.id().to_string(),
        message: if ids.is_empty() {
            "Select at least one item to delete.".into()
        } else {
            bulk_delete_message(ids.len())
        },
        form_name: "p_filesystem.VNodeBulkDeleteForm".into(),
        ids: ids_csv(&ids),
        return_to,
        error: if ids.is_empty() {
            "No items selected.".into()
        } else {
            String::new()
        },
        can_submit: !ids.is_empty(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx))
}

/// HTTP handler: `bulk_delete_post`.
pub async fn bulk_delete_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<BulkIdsForm>,
) -> Response {
    let ids = parse_bulk_ids(&form.ids);
    let return_to = safe_return_to(&form.return_to);
    if ids.is_empty() {
        let page = VNodeConfirmBulkDeletePage {
            modal_uid: V_NODE_BULK_DELETE_MODAL.id().to_string(),
            message: "Select at least one item to delete.".into(),
            form_name: "p_filesystem.VNodeBulkDeleteForm".into(),
            ids: String::new(),
            return_to,
            error: "No items selected.".into(),
            can_submit: false,
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx)).into_response();
    }
    for id in &ids {
        let Some(n) =
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, *id).await, "get node by id")
        else {
            continue;
        };
        if let Err(e) = node::authorize_remove(&n, &AccessActor::from_auth(&ctx)) {
            let page = VNodeConfirmBulkDeletePage {
                modal_uid: V_NODE_BULK_DELETE_MODAL.id().to_string(),
                message: bulk_delete_message(ids.len()),
                form_name: "p_filesystem.VNodeBulkDeleteForm".into(),
                ids: ids_csv(&ids),
                return_to,
                error: e.to_string(),
                can_submit: true,
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx)).into_response();
        }
        if let Err(e) = node::delete_tree(&state.db, state.store.as_ref(), &n).await {
            tracing::error!(error = %e, id, "failed to bulk-delete vnode");
            let page = VNodeConfirmBulkDeletePage {
                modal_uid: V_NODE_BULK_DELETE_MODAL.id().to_string(),
                message: bulk_delete_message(ids.len()),
                form_name: "p_filesystem.VNodeBulkDeleteForm".into(),
                ids: ids_csv(&ids),
                return_to,
                error: format!("Failed to delete \"{}\": {e}", n.name),
                can_submit: true,
            };
            return html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx)).into_response();
        }
    }
    htmx.redirect(&return_to)
}

/// HTTP handler: `bulk_move_get`.
pub async fn bulk_move_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<BulkIdsQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let return_to = safe_return_to(q.return_to.as_deref().unwrap_or(&VNodeListRouteTag.path()));
    if ids.is_empty() {
        return Redirect::to(&return_to).into_response();
    }
    let page = VNodeBulkMoveFormPage {
        ids: ids_csv(&ids),
        count: ids.len(),
        destination_id: 0,
        destination_display: String::new(),
        return_to,
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
}

/// HTTP handler: `bulk_move_post`.
pub async fn bulk_move_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<BulkMovePostForm>,
) -> Response {
    let ids = parse_bulk_ids(&form.ids);
    let return_to = safe_return_to(&form.return_to);
    if ids.is_empty() {
        return Redirect::to(&return_to).into_response();
    }
    let destination = if form.destination_id == 0 {
        None
    } else {
        lariv_core::web::opt_or_log(
            node::get_by_id(&state.db, form.destination_id).await,
            "get node by id",
        )
    };
    for id in &ids {
        let Some(n) =
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, *id).await, "get node by id")
        else {
            continue;
        };
        let actor = AccessActor::from_auth(&ctx);
        if let Err(e) = node::authorize_remove(&n, &actor)
            .and(node::authorize_create_in(&state.db, destination.as_ref(), &actor).await)
        {
            let destination_display = destination
                .as_ref()
                .map(|d| d.name.clone())
                .unwrap_or_default();
            let page = VNodeBulkMoveFormPage {
                ids: ids_csv(&ids),
                count: ids.len(),
                destination_id: form.destination_id,
                destination_display,
                return_to,
                error: e.to_string(),
            };
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx))
                .into_response();
        }
        if let Err(e) = node::move_to(&state.db, n, destination.as_ref()).await {
            let destination_display = destination
                .as_ref()
                .map(|d| d.name.clone())
                .unwrap_or_default();
            let page = VNodeBulkMoveFormPage {
                ids: ids_csv(&ids),
                count: ids.len(),
                destination_id: form.destination_id,
                destination_display,
                return_to,
                error: e.to_string(),
            };
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx))
                .into_response();
        }
    }
    htmx.redirect(&return_to)
}

/// HTTP handler: `bulk_download`.
pub async fn bulk_download(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkIdsQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    if ids.is_empty() {
        return (StatusCode::BAD_REQUEST, "no items selected").into_response();
    }
    let mut nodes = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(n) =
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        else {
            continue;
        };
        nodes.push(n);
    }
    let actor = AccessActor::from_auth(&ctx);
    for n in &nodes {
        let check = if n.is_directory {
            node::authorize_open(n, &actor)
                .and(node::authorize_view_tree(&state.db, n, &actor).await)
        } else {
            node::authorize_view(n, &actor)
        };
        if let Err(e) = check {
            return forbidden(e);
        }
    }
    if nodes.is_empty() {
        return (StatusCode::NOT_FOUND, "no items found").into_response();
    }
    match zip::build_zip_from_nodes(&state.db, state.store.as_ref(), &nodes).await {
        Ok((filename, bytes)) => zip_response(&filename, bytes),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
