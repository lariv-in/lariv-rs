//! VNode business logic — go` / `db_fs.go`.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, sea_query::Expr,
};
use tokio::io::AsyncReadExt;

use lariv_core::html_form::UploadedFile;

use super::entities::VNode;
use super::entities::filesystem_node::{ActiveModel, Column, Entity as VNodeEntity};
use super::entities::filesystem_preferences::FilesystemPreferences;
use super::permissions::{AccessActor, NodePermissions, NodeRight};
use super::preferences;
use super::storage::{DynFilestore, FilestoreError, human_readable_size};

/// Max bytes loaded into the in-browser text editor (or accepted on save).
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;

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
    Forbidden,
    Db(DbErr),
    Store(FilestoreError),
}

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(msg) => write!(f, "{msg}"),
            Self::Conflict => write!(f, "an item with this name already exists here"),
            Self::Forbidden => write!(f, "you do not have access to this item"),
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

/// True for a file whose name ends with `.typ` (Typst source).
pub fn is_typst_file(name: &str, is_directory: bool) -> bool {
    !is_directory && name.to_ascii_lowercase().ends_with(".typ")
}

/// CodeMirror language key for a VNode filename.
pub fn editor_language(name: &str) -> &'static str {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" => "markdown",
        "js" | "mjs" | "cjs" => "javascript",
        "typ" => "typst",
        _ => "plaintext",
    }
}

/// UTF-8 text within [`MAX_TEXT_BYTES`].
pub fn decode_text(bytes: &[u8]) -> Option<&str> {
    if bytes.len() > MAX_TEXT_BYTES {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

/// Error unless `content` is within [`MAX_TEXT_BYTES`].
pub fn require_text_content_size(content: &str) -> Result<(), String> {
    if content.len() > MAX_TEXT_BYTES {
        return Err(format!(
            "content is {} bytes; text writes are limited to {MAX_TEXT_BYTES} bytes",
            content.len()
        ));
    }
    Ok(())
}

/// Load editor text for a file VNode, or `None` for directories / binary / oversized files.
///
/// Missing blobs and files with no stored path are treated as empty text so the editor
/// can create content.
pub async fn try_read_text(store: &DynFilestore, node: &VNode) -> Option<String> {
    if node.is_directory {
        return None;
    }
    let Some(path) = node.file_path.as_deref().filter(|p| !p.is_empty()) else {
        return Some(String::new());
    };
    match store.stored_size(path).await {
        Ok(size) if size as usize > MAX_TEXT_BYTES => return None,
        Err(e) if e.is_missing() => return Some(String::new()),
        Err(_) => return None,
        Ok(_) => {}
    }
    let mut download = store.open(path, &node.name).await.ok()?;
    let mut buf = Vec::new();
    download.reader.read_to_end(&mut buf).await.ok()?;
    decode_text(&buf).map(str::to_string)
}

pub fn item_type(node: &VNode) -> &'static str {
    if node.is_directory {
        "Directory"
    } else {
        "File"
    }
}

fn actor_allows(node: &VNode, actor: &AccessActor, right: NodeRight) -> bool {
    node.permissions.allows(
        right,
        actor.user_id.is_some_and(|id| node.owner_id == Some(id)),
        actor
            .role
            .as_deref()
            .is_some_and(|name| node.role.as_deref() == Some(name)),
        actor.role.as_deref(),
    )
}

fn require_right(node: &VNode, actor: &AccessActor, right: NodeRight) -> Result<(), NodeError> {
    if actor_allows(node, actor, right) {
        Ok(())
    } else {
        Err(NodeError::Forbidden)
    }
}

/// View on `node` itself. A listing includes `node` only when this succeeds.
/// Parent bits are not consulted. Opening a file uses this right.
pub fn authorize_view(node: &VNode, actor: &AccessActor) -> Result<(), NodeError> {
    require_right(node, actor, NodeRight::Read)
}

/// Open on `node`. Entering a directory requires this right.
pub fn authorize_open(node: &VNode, actor: &AccessActor) -> Result<(), NodeError> {
    require_right(node, actor, NodeRight::Execute)
}

/// View on `node` and every descendant, each on its own bits.
pub async fn authorize_view_tree(
    db: &DatabaseConnection,
    node: &VNode,
    actor: &AccessActor,
) -> Result<(), NodeError> {
    let mut pending = vec![node.clone()];
    while let Some(current) = pending.pop() {
        authorize_view(&current, actor)?;
        if current.is_directory {
            pending.extend(list_children(db, Some(current.id), false, "").await?);
        }
    }
    Ok(())
}

/// Change on the file itself.
pub fn authorize_change_file(node: &VNode, actor: &AccessActor) -> Result<(), NodeError> {
    require_right(node, actor, NodeRight::Write)
}

fn root_allows(root: &FilesystemPreferences, actor: &AccessActor, right: NodeRight) -> bool {
    root.permissions.allows(
        right,
        actor.user_id.is_some_and(|id| root.owner_id == Some(id)),
        actor
            .role
            .as_deref()
            .is_some_and(|name| root.role.as_deref() == Some(name)),
        actor.role.as_deref(),
    )
}

async fn require_root(
    db: &DatabaseConnection,
    actor: &AccessActor,
    rights: &[NodeRight],
) -> Result<(), NodeError> {
    let root = preferences::load(db).await?;
    for right in rights {
        if !root_allows(&root, actor, *right) {
            return Err(NodeError::Forbidden);
        }
    }
    Ok(())
}

/// View on the filesystem root. The root uses the global permissions row.
pub async fn authorize_view_root(
    db: &DatabaseConnection,
    actor: &AccessActor,
) -> Result<(), NodeError> {
    require_root(db, actor, &[NodeRight::Read]).await
}

/// Create inside `parent`. The filesystem root uses global permissions and needs
/// change and open. A folder needs those same rights on the folder itself.
pub async fn authorize_create_in(
    db: &DatabaseConnection,
    parent: Option<&VNode>,
    actor: &AccessActor,
) -> Result<(), NodeError> {
    let Some(parent) = parent else {
        return require_root(db, actor, &[NodeRight::Execute, NodeRight::Write]).await;
    };
    require_right(parent, actor, NodeRight::Execute)?;
    require_right(parent, actor, NodeRight::Write)
}

/// Owner or superuser may replace the filesystem root's owner, role, and access.
pub async fn authorize_set_root_access(
    db: &DatabaseConnection,
    actor: &AccessActor,
) -> Result<(), NodeError> {
    let root = preferences::load(db).await?;
    if actor
        .role
        .as_deref()
        .is_some_and(lariv_plugin_users::roles::Superuser::matches)
        || actor.user_id.is_some_and(|id| root.owner_id == Some(id))
    {
        Ok(())
    } else {
        Err(NodeError::Forbidden)
    }
}

/// Rename, delete, or move `node`. Change is required on the item itself.
pub fn authorize_remove(node: &VNode, actor: &AccessActor) -> Result<(), NodeError> {
    require_right(node, actor, NodeRight::Write)
}

/// Owner or superuser may replace owner, role, and access.
pub fn authorize_set_access(node: &VNode, actor: &AccessActor) -> Result<(), NodeError> {
    if actor
        .role
        .as_deref()
        .is_some_and(lariv_plugin_users::roles::Superuser::matches)
        || actor.user_id.is_some_and(|id| node.owner_id == Some(id))
    {
        Ok(())
    } else {
        Err(NodeError::Forbidden)
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
                let created = create(
                    db,
                    store,
                    name,
                    true,
                    None,
                    parent_model.as_ref(),
                    None,
                    None,
                )
                .await?;
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

/// Direct children `actor` is allowed to see. Hidden items stay out of the count.
pub async fn count_visible_children(
    db: &DatabaseConnection,
    id: i64,
    actor: &AccessActor,
) -> Result<u64, DbErr> {
    let children = list_children(db, Some(id), false, "").await?;
    Ok(children
        .iter()
        .filter(|child| authorize_view(child, actor).is_ok())
        .count() as u64)
}

async fn exists_conflict<C: ConnectionTrait>(
    db: &C,
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

/// . The upload filename supplies the stored extension
/// and, when `name` is blank, the node name.
/// Permissions follow [`NodePermissions::for_kind`].
pub async fn create<C: ConnectionTrait>(
    db: &C,
    store: &DynFilestore,
    name: String,
    is_directory: bool,
    file: Option<NodeFile>,
    parent: Option<&VNode>,
    owner_id: Option<i64>,
    role: Option<String>,
) -> Result<VNode, NodeError> {
    insert_node(
        db,
        store,
        name,
        is_directory,
        file,
        parent,
        owner_id,
        role,
        NodePermissions::for_kind(is_directory),
    )
    .await
}

/// Same as [`create`], owned by `owner_id` with [`NodePermissions::for_owner`].
/// Role is left empty so only the owner can view, change, and open the node.
pub async fn create_owned<C: ConnectionTrait>(
    db: &C,
    store: &DynFilestore,
    name: String,
    is_directory: bool,
    file: Option<NodeFile>,
    parent: Option<&VNode>,
    owner_id: i64,
) -> Result<VNode, NodeError> {
    insert_node(
        db,
        store,
        name,
        is_directory,
        file,
        parent,
        Some(owner_id),
        None,
        NodePermissions::for_owner(),
    )
    .await
}

async fn insert_node<C: ConnectionTrait>(
    db: &C,
    store: &DynFilestore,
    mut name: String,
    is_directory: bool,
    file: Option<NodeFile>,
    parent: Option<&VNode>,
    owner_id: Option<i64>,
    role: Option<String>,
    permissions: NodePermissions,
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
        owner_id: Set(owner_id),
        role: Set(role),
        permissions: Set(permissions),
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

/// Replace owner, role, and access on `node`.
/// When `recursive` is set and `node` is a directory, the same three fields are
/// written onto every descendant.
pub async fn set_access(
    db: &DatabaseConnection,
    node: VNode,
    owner_id: Option<i64>,
    role: Option<String>,
    permissions: NodePermissions,
    recursive: bool,
) -> Result<(), NodeError> {
    let id = node.id;
    let is_directory = node.is_directory;
    write_access(db, node, owner_id, role.clone(), permissions).await?;
    if recursive && is_directory {
        write_access_descendants(db, id, owner_id, role, permissions).await?;
    }
    Ok(())
}

/// Replace owner, role, and access on every vnode.
pub async fn set_access_all(
    db: &DatabaseConnection,
    owner_id: Option<i64>,
    role: Option<String>,
    permissions: NodePermissions,
) -> Result<(), NodeError> {
    VNodeEntity::update_many()
        .col_expr(Column::OwnerId, Expr::value(owner_id))
        .col_expr(Column::Role, Expr::value(role))
        .col_expr(Column::Permissions, Expr::value(permissions))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .exec(db)
        .await?;
    Ok(())
}

async fn write_access(
    db: &DatabaseConnection,
    node: VNode,
    owner_id: Option<i64>,
    role: Option<String>,
    permissions: NodePermissions,
) -> Result<(), NodeError> {
    let mut am: ActiveModel = node.into();
    am.owner_id = Set(owner_id);
    am.role = Set(role);
    am.permissions = Set(permissions);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await?;
    Ok(())
}

fn write_access_descendants<'a>(
    db: &'a DatabaseConnection,
    parent_id: i64,
    owner_id: Option<i64>,
    role: Option<String>,
    permissions: NodePermissions,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), NodeError>> + Send + 'a>> {
    Box::pin(async move {
        let children = list_children(db, Some(parent_id), false, "").await?;
        for child in children {
            let child_id = child.id;
            let child_is_directory = child.is_directory;
            write_access(db, child, owner_id, role.clone(), permissions).await?;
            if child_is_directory {
                write_access_descendants(db, child_id, owner_id, role.clone(), permissions).await?;
            }
        }
        Ok(())
    })
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

#[cfg(test)]
mod tests {
    use super::{
        MAX_TEXT_BYTES, decode_text, editor_language, is_typst_file, require_text_content_size,
    };

    #[test]
    fn typst_file_detects_typ_extension() {
        assert!(is_typst_file("notes.typ", false));
        assert!(is_typst_file("Notes.TYP", false));
        assert!(!is_typst_file("notes.typ", true));
        assert!(!is_typst_file("notes.txt", false));
        assert!(!is_typst_file("notes.typst", false));
    }

    #[test]
    fn editor_language_maps_known_extensions() {
        assert_eq!(editor_language("readme.md"), "markdown");
        assert_eq!(editor_language("README.MARKDOWN"), "markdown");
        assert_eq!(editor_language("app.js"), "javascript");
        assert_eq!(editor_language("mod.mjs"), "javascript");
        assert_eq!(editor_language("lib.cjs"), "javascript");
        assert_eq!(editor_language("notes.typ"), "typst");
        assert_eq!(editor_language("Notes.TYP"), "typst");
        assert_eq!(editor_language("notes.txt"), "plaintext");
        assert_eq!(editor_language("noext"), "plaintext");
    }

    #[test]
    fn decode_text_accepts_utf8() {
        assert_eq!(decode_text(b"hello").unwrap(), "hello");
        assert_eq!(decode_text("café".as_bytes()).unwrap(), "café");
        assert_eq!(decode_text(b"").unwrap(), "");
    }

    #[test]
    fn decode_text_rejects_binary() {
        assert!(decode_text(&[0xff, 0xfe, 0x00]).is_none());
    }

    #[test]
    fn decode_text_rejects_oversize() {
        let bytes = vec![b'a'; MAX_TEXT_BYTES + 1];
        assert!(decode_text(&bytes).is_none());
    }

    #[test]
    fn require_text_content_size_rejects_oversize() {
        let content = "a".repeat(MAX_TEXT_BYTES + 1);
        let err = require_text_content_size(&content).unwrap_err();
        assert!(err.contains("limited to"));
        assert!(require_text_content_size("ok").is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn set_access_recursive_updates_descendants_only_when_asked() {
        use chrono::Utc;
        use sea_orm::{
            ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, EntityTrait, Schema,
        };

        use crate::entities::filesystem_node::{self, Entity as VNodeEntity};
        use crate::permissions::NodePermissions;

        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        let schema = Schema::new(db.get_database_backend());
        db.execute(&schema.create_table_from_entity(VNodeEntity))
            .await
            .expect("create table");

        async fn insert(
            db: &sea_orm::DatabaseConnection,
            name: &str,
            is_directory: bool,
            parent_id: Option<i64>,
        ) -> filesystem_node::Model {
            let now = Utc::now();
            filesystem_node::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(name.into()),
                is_directory: Set(is_directory),
                file_path: Set(None),
                parent_id: Set(parent_id),
                owner_id: Set(None),
                role: Set(None),
                permissions: Set(NodePermissions::legacy()),
            }
            .insert(db)
            .await
            .expect("insert")
        }

        let folder = insert(&db, "docs", true, None).await;
        let child = insert(&db, "notes.txt", false, Some(folder.id)).await;
        let folder_id = folder.id;
        let child_id = child.id;
        let next = NodePermissions::for_file();
        super::set_access(&db, folder, Some(7), Some("3".into()), next, true)
            .await
            .expect("recursive");
        let child = VNodeEntity::find_by_id(child_id)
            .one(&db)
            .await
            .expect("child")
            .expect("child row");
        assert_eq!(child.owner_id, Some(7));
        assert_eq!(child.role.as_deref(), Some("3"));
        assert_eq!(child.permissions, next);
        let folder = VNodeEntity::find_by_id(folder_id)
            .one(&db)
            .await
            .expect("folder")
            .expect("folder row");
        assert_eq!(folder.owner_id, Some(7));
        assert_eq!(folder.permissions, next);

        let folder = insert(&db, "keep", true, None).await;
        let child = insert(&db, "stay.txt", false, Some(folder.id)).await;
        let child_id = child.id;
        super::set_access(&db, folder, Some(1), None, NodePermissions::empty(), false)
            .await
            .expect("one node");
        let child = VNodeEntity::find_by_id(child_id)
            .one(&db)
            .await
            .expect("child")
            .expect("child row");
        assert_eq!(child.owner_id, None);
        assert_eq!(child.role, None);
        assert_eq!(child.permissions, NodePermissions::legacy());

        let folder = insert(&db, "all", true, None).await;
        let child = insert(&db, "nested.txt", false, Some(folder.id)).await;
        let other = insert(&db, "root.txt", false, None).await;
        let next = NodePermissions::USER_READ;
        super::set_access_all(&db, Some(4), Some("6".into()), next)
            .await
            .expect("all nodes");
        for id in [folder.id, child.id, other.id] {
            let row = VNodeEntity::find_by_id(id)
                .one(&db)
                .await
                .expect("row")
                .expect("present");
            assert_eq!(row.owner_id, Some(4));
            assert_eq!(row.role.as_deref(), Some("6"));
            assert_eq!(row.permissions, next);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn authorize_checks_class_ancestors_root_and_owner() {
        use chrono::Utc;
        use sea_orm::{
            ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, EntityTrait, Schema,
        };

        use crate::entities::filesystem_node::{self, Entity as VNodeEntity};
        use crate::permissions::{AccessActor, NodePermissions};

        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        let schema = Schema::new(db.get_database_backend());
        db.execute(&schema.create_table_from_entity(VNodeEntity))
            .await
            .expect("create table");

        async fn insert(
            db: &sea_orm::DatabaseConnection,
            name: &str,
            is_directory: bool,
            parent_id: Option<i64>,
            owner_id: Option<i64>,
            role: Option<String>,
            permissions: NodePermissions,
        ) -> filesystem_node::Model {
            let now = Utc::now();
            filesystem_node::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(name.into()),
                is_directory: Set(is_directory),
                file_path: Set(None),
                parent_id: Set(parent_id),
                owner_id: Set(owner_id),
                role: Set(role),
                permissions: Set(permissions),
            }
            .insert(db)
            .await
            .expect("insert")
        }

        let other = AccessActor {
            user_id: Some(2),
            role: Some("8".into()),
        };
        let anon = AccessActor::anonymous();
        let shared = NodePermissions::OTHER_READ | NodePermissions::ALL_READ;
        let file = insert(
            &db,
            "shared.txt",
            false,
            None,
            Some(1),
            Some("9".into()),
            shared,
        )
        .await;
        assert!(super::authorize_view(&file, &other).is_ok());
        assert!(super::authorize_view(&file, &anon).is_ok());

        let closed = NodePermissions::empty();
        let mut locked: filesystem_node::ActiveModel = file.clone().into();
        locked.permissions = Set(closed);
        locked.update(&db).await.expect("close file");
        let file = VNodeEntity::find_by_id(file.id)
            .one(&db)
            .await
            .expect("reload")
            .expect("file");
        assert!(super::authorize_view(&file, &other).is_err());
        assert!(super::authorize_view(&file, &anon).is_err());

        let role_reader = AccessActor {
            user_id: Some(4),
            role: Some("3".into()),
        };
        let outsider = AccessActor {
            user_id: Some(5),
            role: Some("6".into()),
        };
        let grouped = insert(
            &db,
            "group.txt",
            false,
            None,
            Some(1),
            Some("3".into()),
            NodePermissions::ROLE_READ,
        )
        .await;
        assert!(super::authorize_view(&grouped, &role_reader).is_ok());
        assert!(super::authorize_view(&grouped, &outsider).is_err());

        let sealed = insert(
            &db,
            "sealed",
            true,
            None,
            Some(1),
            None,
            NodePermissions::OTHER_READ,
        )
        .await;
        let nested = insert(
            &db,
            "inside.txt",
            false,
            Some(sealed.id),
            None,
            None,
            NodePermissions::OTHER_READ,
        )
        .await;
        assert!(super::authorize_view(&nested, &anon).is_ok());
        assert!(super::authorize_remove(&nested, &anon).is_err());

        let open = NodePermissions::OTHER_READ | NodePermissions::OTHER_EXECUTE;
        let folder = insert(&db, "drop", true, None, Some(1), None, open).await;
        assert!(
            super::authorize_create_in(&db, Some(&folder), &anon)
                .await
                .is_err()
        );
        let writable = NodePermissions::OTHER_READ
            | NodePermissions::OTHER_WRITE
            | NodePermissions::OTHER_EXECUTE;
        let folder = insert(&db, "inbox", true, None, Some(1), None, writable).await;
        assert!(
            super::authorize_create_in(&db, Some(&folder), &anon)
                .await
                .is_ok()
        );

        let root_file = insert(
            &db,
            "root.txt",
            false,
            None,
            Some(1),
            None,
            NodePermissions::OTHER_READ,
        )
        .await;
        assert!(super::authorize_remove(&root_file, &other).is_err());
        let root_writable = insert(
            &db,
            "mine.txt",
            false,
            None,
            Some(2),
            None,
            NodePermissions::USER_WRITE,
        )
        .await;
        assert!(super::authorize_remove(&root_writable, &other).is_ok());

        let owned = insert(
            &db,
            "owned.txt",
            false,
            None,
            Some(1),
            None,
            NodePermissions::OTHER_WRITE,
        )
        .await;
        let owner = AccessActor {
            user_id: Some(1),
            role: Some("9".into()),
        };
        let changer = AccessActor {
            user_id: Some(2),
            role: Some("9".into()),
        };
        assert!(super::authorize_set_access(&owned, &owner).is_ok());
        assert!(super::authorize_set_access(&owned, &changer).is_err());
        let root = AccessActor {
            user_id: Some(99),
            role: Some(lariv_plugin_users::roles::Superuser::NAME.into()),
        };
        assert!(super::authorize_set_access(&owned, &root).is_ok());
        assert!(super::authorize_view(&file, &root).is_ok());
    }

    #[test]
    fn listing_uses_view_and_entering_a_directory_uses_open() {
        use crate::entities::VNode;
        use crate::permissions::{AccessActor, NodePermissions};

        fn sample(
            name: &str,
            is_directory: bool,
            owner_id: Option<i64>,
            role: Option<&str>,
            permissions: NodePermissions,
        ) -> VNode {
            VNode {
                id: 1,
                created_at: None,
                updated_at: None,
                name: name.into(),
                is_directory,
                file_path: None,
                parent_id: None,
                owner_id,
                role: role.map(str::to_string),
                permissions,
            }
        }

        let owner = AccessActor {
            user_id: Some(1),
            role: Some("admin".into()),
        };
        let other = AccessActor {
            user_id: Some(2),
            role: Some("hr".into()),
        };
        let hidden = sample(
            "secret.txt",
            false,
            Some(1),
            None,
            NodePermissions::USER_READ,
        );
        let shown = sample(
            "notes.txt",
            false,
            Some(1),
            None,
            NodePermissions::USER_READ | NodePermissions::OTHER_READ,
        );
        let children = [hidden, shown];
        let visible: Vec<_> = children
            .iter()
            .filter(|node| super::authorize_view(node, &other).is_ok())
            .map(|node| node.name.as_str())
            .collect();
        assert_eq!(visible, ["notes.txt"]);
        assert!(super::authorize_view(&children[0], &owner).is_ok());

        let listed = sample("plans", true, Some(1), None, NodePermissions::OTHER_READ);
        assert!(super::authorize_view(&listed, &other).is_ok());
        assert!(super::authorize_open(&listed, &other).is_err());

        let enterable = sample("plans", true, Some(1), None, NodePermissions::OTHER_EXECUTE);
        assert!(super::authorize_open(&enterable, &other).is_ok());
        assert!(super::authorize_view(&enterable, &other).is_err());

        let file = sample(
            "notes.txt",
            false,
            Some(1),
            None,
            NodePermissions::for_file(),
        );
        assert!(super::authorize_view(&file, &other).is_ok());
        assert!(super::authorize_open(&file, &other).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn root_permissions_gate_view_create_and_settings() {
        use sea_orm::{ConnectionTrait, Database, Schema};

        use crate::entities::FilesystemPreferencesEntity;
        use crate::permissions::{AccessActor, NodePermissions};
        use crate::preferences;

        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        let schema = Schema::new(db.get_database_backend());
        db.execute(&schema.create_table_from_entity(FilesystemPreferencesEntity))
            .await
            .expect("create table");

        let anon = AccessActor::anonymous();
        let outsider = AccessActor {
            user_id: Some(2),
            role: Some("8".into()),
        };
        assert!(super::authorize_view_root(&db, &anon).await.is_ok());
        assert!(
            super::authorize_create_in(&db, None, &outsider)
                .await
                .is_ok()
        );
        assert!(
            super::authorize_set_root_access(&db, &outsider)
                .await
                .is_err()
        );
        let superuser = AccessActor {
            user_id: Some(9),
            role: Some(lariv_plugin_users::roles::Superuser::NAME.into()),
        };
        assert!(
            super::authorize_set_root_access(&db, &superuser)
                .await
                .is_ok()
        );

        preferences::save(&db, Some(2), Some("8".into()), NodePermissions::USER_READ)
            .await
            .expect("save root");
        assert!(super::authorize_view_root(&db, &outsider).await.is_ok());
        assert!(super::authorize_view_root(&db, &anon).await.is_err());
        assert!(
            super::authorize_create_in(&db, None, &outsider)
                .await
                .is_err()
        );
        assert!(
            super::authorize_set_root_access(&db, &outsider)
                .await
                .is_ok()
        );
        assert!(
            super::authorize_create_in(&db, None, &superuser)
                .await
                .is_ok()
        );
    }
}
