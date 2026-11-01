use axum::http::Uri;
use sea_orm::EntityTrait;

use crate::accounting_preferences_patch::{opt_i64_form_value, str_to_opt_i64};

pub fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub fn parse_i64(s: &str) -> Option<i64> {
    if s.is_empty() { None } else { s.parse().ok() }
}

pub fn optional_fk(raw: &str) -> Option<i64> {
    str_to_opt_i64(raw)
}

pub fn optional_fk_form_value(id: Option<i64>) -> String {
    opt_i64_form_value(id)
}

/// Display name for an optional filesystem vnode FK.
pub async fn vnode_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
    lariv_core::web::opt_or_log(VNodeEntity::find_by_id(id).one(db).await, "find vnode")
        .map(|n| n.name)
        .unwrap_or_else(|| format!("File #{id}"))
}

/// Detail URL and label for an optional filesystem vnode FK.
pub async fn vnode_link(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> (String, String) {
    let Some(id) = id.filter(|id| *id > 0) else {
        return (String::new(), String::new());
    };
    let label = vnode_display(db, Some(id)).await;
    (
        lariv_plugin_filesystem::routes::VNodeDetailRouteTag::new(id).url(),
        label,
    )
}

pub fn parse_i32(s: &str) -> Option<i32> {
    if s.is_empty() { None } else { s.parse().ok() }
}

pub fn checkbox_on(s: &str) -> bool {
    s == "on" || s == "true" || s == "1"
}

pub fn query_param(path_and_query: &str, key: &str) -> Option<String> {
    path_and_query
        .split('?')
        .nth(1)?
        .split('&')
        .find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            if k == key { Some(v.to_string()) } else { None }
        })
}
