//! HTTP handlers for VNode list, detail, create, update, delete, and select.

use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use sea_orm::DatabaseConnection;

use crate::entities::filesystem_node::Column;
use crate::{entities::VNode, node, permissions::AccessActor, templates::VNodeDetailPage};
use lariv_core::components::SlotCtx;
use lariv_core::list_filters::{FilterField, text_column};
use lariv_plugin_users::state::AuthContext;

pub mod bulk;
pub mod create;
pub mod detail;
pub mod download;
pub mod edit;
pub mod list;
pub mod move_form;
pub mod permissions;
pub mod select;
pub mod upload;

pub use bulk::{bulk_delete_get, bulk_delete_post, bulk_download, bulk_move_get, bulk_move_post};
pub use create::{create_get, create_get_in, create_post, create_post_in};
pub use detail::{content_post, detail};
pub use download::{download, download_root};
pub use edit::{delete_get, delete_post, edit_get, edit_post};
pub use list::{browse, list};
pub use move_form::{move_get, move_post};
pub use permissions::{
    permissions_get, permissions_post, root_permissions_get, root_permissions_post,
};
pub use select::{
    file_select, file_select_in, move_select, move_select_in, node_select, node_select_in, select,
    select_in,
};
pub use upload::{
    upload_get, upload_get_in, upload_post, upload_post_in, zip_upload_get, zip_upload_get_in,
    zip_upload_post, zip_upload_post_in,
};

pub(crate) fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub(crate) fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    lariv_core::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
}

pub(crate) fn slot_ctx(ctx: &AuthContext) -> SlotCtx {
    SlotCtx::from_auth(ctx)
}

pub(crate) fn vnode_detail_page(
    data: &crate::layers::VNodeDetailData,
    tz: &str,
    text_content: Option<String>,
    save_error: String,
) -> VNodeDetailPage {
    VNodeDetailPage {
        id: data.node.id,
        name: data.node.name.clone(),
        is_directory: data.node.is_directory,
        item_type: node::item_type(&data.node).to_string(),
        size_display: data.size_display.clone(),
        items_display: data.items_display.clone(),
        path: data.path.clone(),
        updated_at: format_updated_at(data.node.updated_at, tz),
        text_content,
        save_error,
    }
}

pub(crate) fn forbidden(err: node::NodeError) -> Response {
    (StatusCode::FORBIDDEN, err.to_string()).into_response()
}

pub(crate) fn access_response(err: node::NodeError) -> Response {
    let status = match err {
        node::NodeError::Forbidden => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, err.to_string()).into_response()
}

pub(crate) async fn require_root_view(
    db: &DatabaseConnection,
    auth: &AuthContext,
) -> Result<(), Response> {
    node::authorize_view_root(db, &AccessActor::from_auth(auth))
        .await
        .map_err(access_response)
}

pub(crate) fn require_view(node: &VNode, auth: &AuthContext) -> Result<(), Response> {
    node::authorize_view(node, &AccessActor::from_auth(auth)).map_err(forbidden)
}

pub(crate) fn require_open(node: &VNode, auth: &AuthContext) -> Result<(), Response> {
    node::authorize_open(node, &AccessActor::from_auth(auth)).map_err(forbidden)
}

/// Root listing needs view on the root. A directory listing needs open, to enter
/// it, and view, to see what is inside.
pub(crate) async fn require_list_access(
    db: &DatabaseConnection,
    parent_id: Option<i64>,
    parent: Option<&VNode>,
    auth: &AuthContext,
) -> Result<(), Response> {
    if parent_id.is_none() {
        return require_root_view(db, auth).await;
    }
    if let Some(folder) = parent {
        require_open(folder, auth)?;
        require_view(folder, auth)?;
    }
    Ok(())
}

pub(crate) fn vnode_filter_fields() -> Vec<FilterField> {
    vec![text_column("name", "Name", Column::Name)]
}
