//! Access checks for nodes and the filesystem root.

use sea_orm::DatabaseConnection;

use crate::entities::VNode;
use crate::entities::filesystem_preferences::FilesystemPreferences;
use crate::permissions::{AccessActor, NodeRight};
use crate::preferences;

use super::error::NodeError;

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
            pending.extend(super::query::list_children(db, Some(current.id), false, "").await?);
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

#[cfg(test)]
mod tests {
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
