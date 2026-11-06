//! Create / update / move / delete / access writes.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, sea_query::Expr,
};

use crate::entities::VNode;
use crate::entities::filesystem_node::{ActiveModel, Column, Entity as VNodeEntity};
use crate::permissions::NodePermissions;
use crate::storage::DynFilestore;

use super::error::{NodeError, NodeFile};

/// Soft-deletes every direct child of `parent_id` (used before replacing zip-uploaded contents).
pub async fn delete_direct_children(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
) -> Result<(), NodeError> {
    let children = super::query::list_children(db, parent_id, false, "").await?;
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
        let name = super::text::sanitize_node_name(seg);
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
                    Some(id) => super::query::get_by_id(db, id).await?,
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

async fn exists_conflict<C: ConnectionTrait>(
    db: &C,
    parent_id: Option<i64>,
    name: &str,
    is_directory: bool,
    exclude_id: Option<i64>,
) -> Result<bool, DbErr> {
    Ok(super::query::find_child(db, parent_id, name, is_directory)
        .await?
        .is_some_and(|n| exclude_id.is_none_or(|id| n.id != id)))
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
    name = super::text::sanitize_node_name(&name);
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
    name = super::text::sanitize_node_name(&name);
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
        current_parent_id = super::query::get_by_id(db, pid)
            .await?
            .and_then(|p| p.parent_id);
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
        let children = super::query::list_children(db, Some(parent_id), false, "").await?;
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
        let children = super::query::list_children(db, Some(node.id), false, "").await?;
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

#[cfg(test)]
mod tests {
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
}
