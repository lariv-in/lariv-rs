//! Merge a git tree into a directory's children.
//!
//! A matching directory is kept and its contents are merged. A matching file keeps
//! its vnode and receives new bytes. Anything else in the directory is deleted.
//! The directory itself is never deleted.

use std::collections::HashSet;

use sea_orm::DatabaseConnection;

use lariv_plugin_filesystem::entities::VNode;
use lariv_plugin_filesystem::node::{self, NodeError, NodeFile};
use lariv_plugin_filesystem::storage::DynFilestore;

use crate::fetch::{GitEntry, GitEntryKind};

fn check_names(entries: &[GitEntry]) -> Result<(), NodeError> {
    let mut names = HashSet::new();
    for entry in entries {
        let name = node::sanitize_node_name(&entry.name);
        if name.is_empty() || !names.insert(name) {
            return Err(NodeError::Validation(
                "git tree has an empty or duplicate name".into(),
            ));
        }
        if let GitEntryKind::Directory(children) = &entry.kind {
            check_names(children)?;
        }
    }
    Ok(())
}

fn find_entry<'a>(entries: &'a [GitEntry], name: &str) -> Option<&'a GitEntry> {
    entries
        .iter()
        .find(|entry| node::sanitize_node_name(&entry.name) == name)
}

async fn create_entry(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent: &VNode,
    entry: &GitEntry,
    owner_id: Option<i64>,
    role: Option<String>,
) -> Result<(), NodeError> {
    match &entry.kind {
        GitEntryKind::Directory(children) => {
            let created = node::create(
                db,
                store,
                entry.name.clone(),
                true,
                None,
                Some(parent),
                owner_id,
                role.clone(),
            )
            .await?;
            Box::pin(merge_into(db, store, &created, children, owner_id, role)).await
        }
        GitEntryKind::File(bytes) => {
            node::create(
                db,
                store,
                entry.name.clone(),
                false,
                Some(NodeFile::Bytes {
                    filename: entry.name.clone(),
                    data: bytes.clone(),
                }),
                Some(parent),
                owner_id,
                role,
            )
            .await?;
            Ok(())
        }
    }
}

async fn apply_match(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent: &VNode,
    child: VNode,
    entry: &GitEntry,
    owner_id: Option<i64>,
    role: Option<String>,
) -> Result<(), NodeError> {
    match (&entry.kind, child.is_directory) {
        (GitEntryKind::Directory(children), true) => {
            Box::pin(merge_into(db, store, &child, children, owner_id, role)).await
        }
        (GitEntryKind::File(bytes), false) => {
            let name = child.name.clone();
            node::update(
                db,
                store,
                child,
                name.clone(),
                Some(NodeFile::Bytes {
                    filename: name,
                    data: bytes.clone(),
                }),
            )
            .await?;
            Ok(())
        }
        _ => {
            node::delete_tree(db, store, &child).await?;
            create_entry(db, store, parent, entry, owner_id, role).await
        }
    }
}

/// Merge `entries` into the children of `parent`.
pub async fn merge_into(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent: &VNode,
    entries: &[GitEntry],
    owner_id: Option<i64>,
    role: Option<String>,
) -> Result<(), NodeError> {
    check_names(entries)?;
    let children = node::list_children(db, Some(parent.id), false, "").await?;
    let mut matched = HashSet::new();
    for child in children {
        let name = node::sanitize_node_name(&child.name);
        let Some(entry) = find_entry(entries, &name) else {
            node::delete_tree(db, store, &child).await?;
            continue;
        };
        if name.is_empty() || !matched.insert(name) {
            node::delete_tree(db, store, &child).await?;
            continue;
        }
        apply_match(db, store, parent, child, entry, owner_id, role.clone()).await?;
    }
    for entry in entries {
        let name = node::sanitize_node_name(&entry.name);
        if matched.contains(&name) {
            continue;
        }
        create_entry(db, store, parent, entry, owner_id, role.clone()).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, Schema};

    use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
    use lariv_plugin_filesystem::node::{self, NodeFile};
    use lariv_plugin_filesystem::storage::{DynFilestore, LocalFilestore};
    use lariv_plugin_filesystem::zip::read_file_bytes;

    use super::merge_into;
    use crate::fetch::{GitEntry, GitEntryKind};

    fn file(name: &str, bytes: &[u8]) -> GitEntry {
        GitEntry {
            name: name.into(),
            kind: GitEntryKind::File(bytes.to_vec()),
        }
    }

    fn dir(name: &str, children: Vec<GitEntry>) -> GitEntry {
        GitEntry {
            name: name.into(),
            kind: GitEntryKind::Directory(children),
        }
    }

    #[tokio::test]
    async fn merge_keeps_matching_nodes_and_replaces_the_rest() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        let schema = Schema::new(db.get_database_backend());
        db.execute(&schema.create_table_from_entity(VNodeEntity))
            .await
            .expect("table");
        let tmp = tempfile::tempdir().expect("store");
        let store = LocalFilestore::new(tmp.path().display().to_string());
        let store_ref: &DynFilestore = &store;

        let parent = node::create(
            &db,
            store_ref,
            "docs".into(),
            true,
            None,
            None,
            Some(3),
            Some("admin".into()),
        )
        .await
        .expect("parent");
        let keep = node::create(
            &db,
            store_ref,
            "keep.txt".into(),
            false,
            Some(NodeFile::Bytes {
                filename: "keep.txt".into(),
                data: b"old".to_vec(),
            }),
            Some(&parent),
            Some(3),
            Some("admin".into()),
        )
        .await
        .expect("keep");
        let folder = node::create(
            &db,
            store_ref,
            "sub".into(),
            true,
            None,
            Some(&parent),
            Some(3),
            Some("admin".into()),
        )
        .await
        .expect("sub");
        let inner = node::create(
            &db,
            store_ref,
            "inner.txt".into(),
            false,
            Some(NodeFile::Bytes {
                filename: "inner.txt".into(),
                data: b"stay".to_vec(),
            }),
            Some(&folder),
            Some(3),
            Some("admin".into()),
        )
        .await
        .expect("inner");
        node::create(
            &db,
            store_ref,
            "gone.txt".into(),
            false,
            Some(NodeFile::Bytes {
                filename: "gone.txt".into(),
                data: b"bye".to_vec(),
            }),
            Some(&parent),
            Some(3),
            Some("admin".into()),
        )
        .await
        .expect("gone");

        let entries = vec![
            file("keep.txt", b"new"),
            dir(
                "sub",
                vec![file("inner.txt", b"stay"), file("added.txt", b"add")],
            ),
            file("fresh.txt", b"fresh"),
        ];
        merge_into(
            &db,
            store_ref,
            &parent,
            &entries,
            parent.owner_id,
            parent.role.clone(),
        )
        .await
        .expect("merge");

        let keep_after = node::find_child(&db, Some(parent.id), "keep.txt", false)
            .await
            .expect("find keep")
            .expect("keep remains");
        assert_eq!(keep_after.id, keep.id);
        let keep_bytes = read_file_bytes(store_ref, &keep_after)
            .await
            .expect("read keep");
        assert_eq!(keep_bytes, b"new");

        let folder_after = node::find_child(&db, Some(parent.id), "sub", true)
            .await
            .expect("find sub")
            .expect("sub remains");
        assert_eq!(folder_after.id, folder.id);
        let inner_after = node::find_child(&db, Some(folder.id), "inner.txt", false)
            .await
            .expect("find inner")
            .expect("inner remains");
        assert_eq!(inner_after.id, inner.id);

        assert!(
            node::find_child(&db, Some(parent.id), "gone.txt", false)
                .await
                .expect("find gone")
                .is_none()
        );
        let fresh = node::find_child(&db, Some(parent.id), "fresh.txt", false)
            .await
            .expect("find fresh")
            .expect("fresh created");
        let fresh_bytes = read_file_bytes(store_ref, &fresh)
            .await
            .expect("read fresh");
        assert_eq!(fresh_bytes, b"fresh");
        assert_eq!(fresh.owner_id, Some(3));
        assert_eq!(fresh.role.as_deref(), Some("admin"));

        let parent_after = node::get_by_id(&db, parent.id)
            .await
            .expect("parent")
            .expect("parent remains");
        assert_eq!(parent_after.id, parent.id);
        assert!(parent_after.is_directory);
    }
}
