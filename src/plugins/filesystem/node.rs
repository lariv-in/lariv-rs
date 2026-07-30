//! VNode business logic — port of Go `p_filesystem` `models.go` / `db_fs.go`.
//!
//! Soft deletes are implemented explicitly (`deleted_at` filters on every query),
//! matching the blog plugin's style rather than relying on a SeaORM `SoftDelete` trait.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};

use super::entities::filesystem_node::{ActiveModel, Column, Entity as VNodeEntity};
use super::entities::VNode;
use super::storage::{DynFilestore, FilestoreError, human_readable_size};

#[derive(Debug)]
pub enum NodeError {
    Validation(String),
    Conflict,
    Db(DbErr),
    Store(FilestoreError),
}

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(msg) => write!(f, "{msg}"),
            Self::Conflict => write!(f, "an item with this name already exists here"),
            Self::Db(e) => write!(f, "{e}"),
            Self::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for NodeError {}

impl From<DbErr> for NodeError {
    fn from(value: DbErr) -> Self {
        Self::Db(value)
    }
}

/// Port of Go `sanitizeNodeName`: trims whitespace and strips any directory
/// components, rejecting `.`/`..`.
pub fn sanitize_node_name(name: &str) -> String {
    let trimmed = name.trim();
    let candidate = trimmed
        .rsplit(['/', '\\'])
        .find(|s| !s.is_empty())
        .unwrap_or("");
    if candidate == "." || candidate == ".." {
        String::new()
    } else {
        candidate.to_string()
    }
}

/// Port of Go `filepath.Ext`: `"a.txt"` → `".txt"`, `"a"` → `""`.
pub fn ext_of(filename: &str) -> String {
    std::path::Path::new(filename)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default()
}

pub fn item_type(node: &VNode) -> &'static str {
    if node.is_directory { "Directory" } else { "File" }
}

pub async fn get_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<VNode>, DbErr> {
    VNodeEntity::find_by_id(id)
        .filter(Column::DeletedAt.is_null())
        .one(db)
        .await
}

pub async fn list_children(
    db: &DatabaseConnection,
    parent_id: Option<i64>,
    only_directories: bool,
    name_filter: &str,
) -> Result<Vec<VNode>, DbErr> {
    let mut query = VNodeEntity::find().filter(Column::DeletedAt.is_null());
    query = match parent_id {
        Some(id) => query.filter(Column::ParentId.eq(id)),
        None => query.filter(Column::ParentId.is_null()),
    };
    if only_directories {
        query = query.filter(Column::IsDirectory.eq(true));
    }
    if !name_filter.is_empty() {
        query = query.filter(Column::Name.contains(name_filter));
    }
    query
        .order_by_desc(Column::IsDirectory)
        .order_by_asc(Column::Name)
        .all(db)
        .await
}

/// Soft-deletes every direct child of `parent_id` (used before replacing zip-uploaded contents).
pub async fn delete_direct_children(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
) -> Result<(), NodeError> {
    let children = list_children(db, parent_id, false, "").await?;
    for child in &children {
        delete_tree(db, store, child).await?;
    }
    Ok(())
}

/// Port of Go `EnsureDirectoryPath`: finds or creates each nested directory
/// segment under `parent_id`, returning the id of the deepest directory.
pub async fn ensure_directory_path(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
    segments: &[String],
) -> Result<Option<i64>, NodeError> {
    let mut current_parent = parent_id;
    for seg in segments {
        let name = sanitize_node_name(seg);
        if name.is_empty() {
            continue;
        }
        let mut query = VNodeEntity::find()
            .filter(Column::Name.eq(&name))
            .filter(Column::IsDirectory.eq(true))
            .filter(Column::DeletedAt.is_null());
        query = match current_parent {
            Some(id) => query.filter(Column::ParentId.eq(id)),
            None => query.filter(Column::ParentId.is_null()),
        };
        let existing = query.one(db).await?;
        current_parent = Some(match existing {
            Some(node) => node.id,
            None => {
                let parent_model = match current_parent {
                    Some(id) => get_by_id(db, id).await?,
                    None => None,
                };
                let created = create(db, store, name, true, None, parent_model.as_ref()).await?;
                created.id
            }
        });
    }
    Ok(current_parent)
}

pub async fn children_count(db: &DatabaseConnection, id: i64) -> Result<u64, DbErr> {
    VNodeEntity::find()
        .filter(Column::ParentId.eq(id))
        .filter(Column::DeletedAt.is_null())
        .count(db)
        .await
}

async fn exists_conflict(
    db: &DatabaseConnection,
    parent_id: Option<i64>,
    name: &str,
    is_directory: bool,
    exclude_id: Option<i64>,
) -> Result<bool, DbErr> {
    let mut query = VNodeEntity::find()
        .filter(Column::Name.eq(name))
        .filter(Column::IsDirectory.eq(is_directory))
        .filter(Column::DeletedAt.is_null());
    query = match parent_id {
        Some(id) => query.filter(Column::ParentId.eq(id)),
        None => query.filter(Column::ParentId.is_null()),
    };
    if let Some(id) = exclude_id {
        query = query.filter(Column::Id.ne(id));
    }
    Ok(query.count(db).await? > 0)
}

/// Port of Go `CreateVNode`. `file` is `(bytes, original_filename)`; the filename
/// supplies the stored extension and, when `name` is blank, the node name.
pub async fn create(
    db: &DatabaseConnection,
    store: &DynFilestore,
    mut name: String,
    is_directory: bool,
    file: Option<(&[u8], &str)>,
    parent: Option<&VNode>,
) -> Result<VNode, NodeError> {
    if let Some(p) = parent
        && !p.is_directory
    {
        return Err(NodeError::Validation(format!(
            "\"{}\" is not a directory",
            p.name
        )));
    }
    if !is_directory && file.is_none() {
        return Err(NodeError::Validation("file upload is required".into()));
    }
    if let Some((_, filename)) = file
        && name.trim().is_empty()
    {
        name = filename.to_string();
    }
    name = sanitize_node_name(&name);
    if name.is_empty() {
        return Err(NodeError::Validation("name is required".into()));
    }

    let parent_id = parent.map(|p| p.id);
    if exists_conflict(db, parent_id, &name, is_directory, None).await? {
        return Err(NodeError::Conflict);
    }

    let stored_path = match file {
        Some((bytes, filename)) => {
            let ext = ext_of(filename);
            Some(store.save(bytes, &ext).await.map_err(NodeError::Store)?)
        }
        None => None,
    };

    let now = Utc::now();
    let am = ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
        name: Set(name),
        is_directory: Set(is_directory),
        file_path: Set(stored_path.clone()),
        parent_id: Set(parent_id),
    };
    match am.insert(db).await {
        Ok(model) => Ok(model),
        Err(e) => {
            if let Some(path) = stored_path
                && let Err(del_err) = store.delete(&path).await
            {
                tracing::error!(path, error = %del_err, "filesystem: failed cleaning up stored file after create error");
            }
            Err(NodeError::Db(e))
        }
    }
}

/// Port of Go `VNode.Update`: rename, optionally replacing the backing file.
pub async fn update(
    db: &DatabaseConnection,
    store: &DynFilestore,
    node: VNode,
    mut name: String,
    file: Option<(&[u8], &str)>,
) -> Result<VNode, NodeError> {
    name = sanitize_node_name(&name);
    if name.is_empty() {
        return Err(NodeError::Validation("name is required".into()));
    }
    if exists_conflict(db, node.parent_id, &name, node.is_directory, Some(node.id)).await? {
        return Err(NodeError::Conflict);
    }
    if file.is_some() && node.is_directory {
        return Err(NodeError::Validation(
            "cannot upload a file for a directory".into(),
        ));
    }

    let old_path = node.file_path.clone();
    let new_path = match file {
        Some((bytes, filename)) => {
            let ext = ext_of(filename);
            Some(store.save(bytes, &ext).await.map_err(NodeError::Store)?)
        }
        None => old_path.clone(),
    };

    let mut am: ActiveModel = node.into();
    am.name = Set(name);
    am.file_path = Set(new_path.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(db).await {
        Ok(model) => {
            if new_path != old_path
                && let Some(old) = old_path
                && let Err(e) = store.delete(&old).await
            {
                tracing::error!(path = old, error = %e, "filesystem: failed deleting replaced stored file");
            }
            Ok(model)
        }
        Err(e) => {
            if new_path != old_path
                && let Some(new) = new_path
                && let Err(del_err) = store.delete(&new).await
            {
                tracing::error!(path = new, error = %del_err, "filesystem: failed cleaning up stored file after update error");
            }
            Err(NodeError::Db(e))
        }
    }
}

pub async fn is_descendant_of(
    db: &DatabaseConnection,
    node: &VNode,
    ancestor_id: i64,
) -> Result<bool, DbErr> {
    let mut current_parent_id = node.parent_id;
    loop {
        let Some(pid) = current_parent_id else {
            return Ok(false);
        };
        if pid == ancestor_id {
            return Ok(true);
        }
        current_parent_id = get_by_id(db, pid).await?.and_then(|p| p.parent_id);
    }
}

/// Port of Go `VNode.MoveToNode`.
pub async fn move_to(
    db: &DatabaseConnection,
    node: VNode,
    destination: Option<&VNode>,
) -> Result<VNode, NodeError> {
    let new_parent_id = match destination {
        Some(dest) => {
            if !dest.is_directory {
                return Err(NodeError::Validation(
                    "destination must be a directory".into(),
                ));
            }
            if dest.id == node.id {
                return Err(NodeError::Validation(
                    "cannot move an item into itself".into(),
                ));
            }
            if is_descendant_of(db, dest, node.id).await? {
                return Err(NodeError::Validation(
                    "cannot move an item into its descendants".into(),
                ));
            }
            Some(dest.id)
        }
        None => None,
    };

    let mut am: ActiveModel = node.into();
    am.parent_id = Set(new_parent_id);
    am.updated_at = Set(Some(Utc::now()));
    Ok(am.update(db).await?)
}

/// Port of Go `VNode.DeleteTree`: soft-deletes the node and all descendants
/// (children first), deleting each file node's backing blob along the way.
pub fn delete_tree<'a>(
    db: &'a DatabaseConnection,
    store: &'a DynFilestore,
    node: &'a VNode,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), NodeError>> + Send + 'a>> {
    Box::pin(async move {
        let children = list_children(db, Some(node.id), false, "").await?;
        for child in &children {
            delete_tree(db, store, child).await?;
        }

        let mut am: ActiveModel = node.clone().into();
        am.deleted_at = Set(Some(Utc::now()));
        am.update(db).await?;

        let path = node.file_path.clone().unwrap_or_default();
        if let Err(e) = store.delete(&path).await {
            tracing::error!(path, error = %e, "filesystem: failed deleting stored file after vnode delete");
        }
        Ok(())
    })
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

/// Port of Go `VNode.FileSizeDisplay`: `"-"` for directories/empty path, `"Missing"`
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
