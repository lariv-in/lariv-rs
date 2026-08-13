//! VNode business logic — go` / `db_fs.go`.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::html_form::UploadedFile;

use super::entities::VNode;
use super::entities::filesystem_node::{ActiveModel, Column, Entity as VNodeEntity};
use super::storage::{DynFilestore, FilestoreError, human_readable_size};

/// File payload for [`create`] / [`update`] — bytes or a spooled multipart upload.
pub enum NodeFile {
    Bytes { filename: String, data: Vec<u8> },
    Upload(UploadedFile),
}

impl NodeFile {
    pub fn filename(&self) -> &str {
        match self {
            Self::Bytes { filename, .. } => filename,
            Self::Upload(u) => u.filename(),
        }
    }

    async fn save_to_store(&self, store: &DynFilestore) -> Result<String, NodeError> {
        let ext = ext_of(self.filename());
        match self {
            Self::Bytes { data, .. } => store.save(data, &ext).await.map_err(NodeError::Store),
            Self::Upload(u) => {
                // Re-open path without dropping the UploadedFile yet.
                let mut reader = tokio::fs::File::open(u.path())
                    .await
                    .map_err(|e| NodeError::Validation(e.to_string()))?;
                store
                    .save_from_reader(&mut reader, &ext)
                    .await
                    .map_err(NodeError::Store)
            }
        }
    }
}

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

/// trims whitespace and strips any directory
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

/// `"a.txt"` → `".txt"`, `"a"` → `""`.
pub fn ext_of(filename: &str) -> String {
    std::path::Path::new(filename)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default()
}

pub fn item_type(node: &VNode) -> &'static str {
    if node.is_directory {
        "Directory"
    } else {
        "File"
    }
}

pub async fn get_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<VNode>, DbErr> {
    VNodeEntity::find_by_id(id).one(db).await
}

pub async fn list_children(
    db: &DatabaseConnection,
    parent_id: Option<i64>,
    only_directories: bool,
    name_filter: &str,
) -> Result<Vec<VNode>, DbErr> {
    let mut query = VNodeEntity::find();
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

/// finds or creates each nested directory
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
            .filter(Column::IsDirectory.eq(true));
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
    Ok(find_child(db, parent_id, name, is_directory)
        .await?
        .is_some_and(|n| exclude_id.is_none_or(|id| n.id != id)))
}

/// Find a child node by name under `parent_id` (`None` = filesystem root).
pub async fn find_child(
    db: &DatabaseConnection,
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

/// . The upload filename supplies the stored extension
/// and, when `name` is blank, the node name.
pub async fn create(
    db: &DatabaseConnection,
    store: &DynFilestore,
    mut name: String,
    is_directory: bool,
    file: Option<NodeFile>,
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
    if let Some(f) = file.as_ref()
        && name.trim().is_empty()
    {
        name = f.filename().to_string();
    }
    name = sanitize_node_name(&name);
    if name.is_empty() {
        return Err(NodeError::Validation("name is required".into()));
    }

    let parent_id = parent.map(|p| p.id);
    if exists_conflict(db, parent_id, &name, is_directory, None).await? {
        return Err(NodeError::Conflict);
    }

    let stored_path = match file.as_ref() {
        Some(f) => Some(f.save_to_store(store).await?),
        None => None,
    };

    let now = Utc::now();
    let am = ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
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

/// rename, optionally replacing the backing file.
pub async fn update(
    db: &DatabaseConnection,
    store: &DynFilestore,
    node: VNode,
    mut name: String,
    file: Option<NodeFile>,
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
    let new_path = match file.as_ref() {
        Some(f) => Some(f.save_to_store(store).await?),
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

/// .
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

/// Hard-deletes the node and all descendants
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

        let path = node.file_path.clone().unwrap_or_default();
        VNodeEntity::delete_by_id(node.id).exec(db).await?;

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
        let name = sanitize_node_name(part);
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
