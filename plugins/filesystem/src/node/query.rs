//! Read / list / path helpers for VNodes.

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};

use crate::entities::VNode;
use crate::entities::filesystem_node::{Column, Entity as VNodeEntity};
use crate::permissions::AccessActor;
use crate::storage::{DynFilestore, human_readable_size};

use super::error::NodeError;

pub async fn get_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<VNode>, DbErr> {
    VNodeEntity::find_by_id(id).one(db).await
}

/// Limit a listing to direct children of `parent_id` (`None` = filesystem root).
///
/// A non-empty `name_filter` searches every level. Callers that pass an empty
/// filter still see only the current folder.
pub fn apply_listing_parent(
    query: sea_orm::Select<VNodeEntity>,
    parent_id: Option<i64>,
    name_filter: &str,
) -> sea_orm::Select<VNodeEntity> {
    if !name_filter.is_empty() {
        return query;
    }
    match parent_id {
        Some(id) => query.filter(Column::ParentId.eq(id)),
        None => query.filter(Column::ParentId.is_null()),
    }
}

pub async fn list_children(
    db: &DatabaseConnection,
    parent_id: Option<i64>,
    only_directories: bool,
    name_filter: &str,
) -> Result<Vec<VNode>, DbErr> {
    let mut query = apply_listing_parent(VNodeEntity::find(), parent_id, name_filter);
    if only_directories {
        query = query.filter(Column::IsDirectory.eq(true));
    }
    if !name_filter.is_empty() {
        query = query.filter(lariv_core::db::trigram::ci_contains(
            Column::Name,
            name_filter,
        ));
    }
    query
        .order_by_desc(Column::IsDirectory)
        .order_by_asc(Column::Name)
        .all(db)
        .await
}

pub async fn children_count(db: &DatabaseConnection, id: i64) -> Result<u64, DbErr> {
    VNodeEntity::find()
        .filter(Column::ParentId.eq(id))
        .count(db)
        .await
}

/// Direct children `actor` is allowed to see. Hidden items stay out of the count.
pub async fn count_visible_children(
    db: &DatabaseConnection,
    id: i64,
    actor: &AccessActor,
) -> Result<u64, DbErr> {
    let children = list_children(db, Some(id), false, "").await?;
    Ok(children
        .iter()
        .filter(|child| super::access::authorize_view(child, actor).is_ok())
        .count() as u64)
}

/// Find a child node by name under `parent_id` (`None` = filesystem root).
pub async fn find_child<C: ConnectionTrait>(
    db: &C,
    parent_id: Option<i64>,
    name: &str,
    is_directory: bool,
) -> Result<Option<VNode>, DbErr> {
    let mut query = VNodeEntity::find()
        .filter(Column::Name.eq(name))
        .filter(Column::IsDirectory.eq(is_directory));
    query = match parent_id {
        Some(id) => query.filter(Column::ParentId.eq(id)),
        None => query.filter(Column::ParentId.is_null()),
    };
    query.one(db).await
}

pub async fn get_path(db: &DatabaseConnection, node: &VNode) -> String {
    let mut segments = vec![node.name.clone()];
    let mut current_parent_id = node.parent_id;
    while let Some(pid) = current_parent_id {
        match get_by_id(db, pid).await {
            Ok(Some(parent)) => {
                segments.insert(0, parent.name.clone());
                current_parent_id = parent.parent_id;
            }
            _ => break,
        }
    }
    format!("/{}", segments.join("/"))
}

/// walk `/a/b/c` from root.
/// Returns `(node, normalized_path)`. Empty/`/` yields `(None, "/")`.
pub async fn get_by_path(
    db: &DatabaseConnection,
    raw_path: &str,
) -> Result<(Option<VNode>, String), NodeError> {
    let cleaned = raw_path.trim();
    if cleaned.is_empty() || cleaned == "/" {
        return Ok((None, "/".into()));
    }
    let parts: Vec<&str> = cleaned.trim_matches('/').split('/').collect();
    let mut current: Option<VNode> = None;
    let mut normalized = Vec::new();

    for (i, part) in parts.iter().enumerate() {
        let name = super::text::sanitize_node_name(part);
        if name.is_empty() {
            return Err(NodeError::Validation(format!(
                "invalid path segment \"{part}\""
            )));
        }
        let mut query = VNodeEntity::find().filter(Column::Name.eq(&name));
        query = match current.as_ref().map(|n| n.id) {
            Some(id) => query.filter(Column::ParentId.eq(id)),
            None => query.filter(Column::ParentId.is_null()),
        };
        let next = query.one(db).await?;
        let Some(next) = next else {
            let traversed = if i == 0 {
                "/".to_string()
            } else {
                format!("/{}", parts[..i].join("/"))
            };
            return Err(NodeError::Validation(format!(
                "path not found: \"{name}\" does not exist in \"{traversed}\""
            )));
        };
        normalized.push(name);
        current = Some(next);
    }

    Ok((current, format!("/{}", normalized.join("/"))))
}

/// `"-"` for directories/empty path, `"Missing"`
/// when the blob is absent, `"Error"` on other stat failures.
pub async fn file_size_display(store: &DynFilestore, node: &VNode) -> String {
    if node.is_directory {
        return "-".to_string();
    }
    let Some(path) = node.file_path.as_deref().filter(|p| !p.is_empty()) else {
        return "-".to_string();
    };
    match store.stored_size(path).await {
        Ok(size) => human_readable_size(size),
        Err(e) if e.is_missing() => "Missing".to_string(),
        Err(_) => "Error".to_string(),
    }
}
