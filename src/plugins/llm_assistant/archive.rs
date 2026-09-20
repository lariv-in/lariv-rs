//! Archive listing and extraction for Rune env bindings (compress-tools / libarchive).

use std::io::Cursor;
use std::path::{Component, Path};

use compress_tools::{
    ArchiveContents, ArchiveIteratorBuilder, ArchivePassword, stat as ArchiveStat,
};
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::plugins::filesystem::entities::VNode;
use crate::plugins::filesystem::node::{self, NodeFile};
use crate::plugins::filesystem::storage::DynFilestore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchiveListEntry {
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ExtractedVNode {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub bytes: usize,
}

impl ExtractedVNode {
    fn from_node(node: &VNode, path: String, bytes: usize) -> Self {
        Self {
            id: node.id,
            name: node.name.clone(),
            path,
            is_directory: node.is_directory,
            bytes,
        }
    }

    pub(crate) fn to_json(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "name": self.name,
            "path": self.path,
            "is_directory": self.is_directory,
        })
    }

    pub(crate) fn to_single_json(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "name": self.name,
            "path": self.path,
            "bytes": self.bytes,
        })
    }
}

struct CollectedEntry {
    path: String,
    size: u64,
    is_directory: bool,
    data: Vec<u8>,
}

fn archive_error(err: impl std::fmt::Display) -> String {
    format!("file is not a valid archive: {err}")
}

fn open_archive<'a>(
    bytes: &'a [u8],
    password: Option<&str>,
) -> Result<compress_tools::ArchiveIterator<Cursor<&'a [u8]>>, String> {
    let source = Cursor::new(bytes);
    let builder = ArchiveIteratorBuilder::new(source).mtree_format(false);
    let iter = if let Some(password) = password.filter(|s| !s.is_empty()) {
        let password = ArchivePassword::new(password).map_err(|e| e.to_string())?;
        builder
            .with_password(password)
            .build()
            .map_err(archive_error)?
    } else {
        builder.build().map_err(archive_error)?
    };
    Ok(iter)
}

fn stat_is_dir(stat: &ArchiveStat) -> bool {
    (stat.st_mode & libc::S_IFMT) == libc::S_IFDIR
}

fn stat_size(stat: &ArchiveStat) -> u64 {
    u64::try_from(stat.st_size).unwrap_or(0)
}

fn collect_entries(bytes: &[u8], password: Option<&str>) -> Result<Vec<CollectedEntry>, String> {
    let iter = open_archive(bytes, password)?;
    let mut entries = Vec::new();
    let mut current_path = None;
    let mut current_size = 0u64;
    let mut current_dir = false;
    let mut current_data = Vec::new();

    let flush = |path: Option<String>,
                 size: u64,
                 is_directory: bool,
                 data: Vec<u8>,
                 entries: &mut Vec<CollectedEntry>| {
        let Some(path) = path else {
            return;
        };
        if path.is_empty() {
            return;
        }
        entries.push(CollectedEntry {
            path,
            size,
            is_directory,
            data,
        });
    };

    for content in iter {
        match content {
            ArchiveContents::StartOfEntry(name, stat) => {
                flush(
                    current_path.take(),
                    current_size,
                    current_dir,
                    std::mem::take(&mut current_data),
                    &mut entries,
                );
                current_path = Some(name);
                current_size = stat_size(&stat);
                current_dir = stat_is_dir(&stat)
                    || current_path
                        .as_deref()
                        .is_some_and(|p| p.ends_with(['/', '\\']));
                current_data = Vec::new();
            }
            ArchiveContents::DataChunk(chunk) => {
                if !current_dir {
                    current_data.extend_from_slice(&chunk);
                }
            }
            ArchiveContents::EndOfEntry => {
                flush(
                    current_path.take(),
                    current_size,
                    current_dir,
                    std::mem::take(&mut current_data),
                    &mut entries,
                );
            }
            ArchiveContents::Err(err) => return Err(archive_error(err)),
        }
    }
    flush(
        current_path.take(),
        current_size,
        current_dir,
        current_data,
        &mut entries,
    );
    Ok(entries)
}

/// Split an archive path into VNode name segments, or `None` if it is unsafe
/// (absolute, empty, or contains `..`).
pub(crate) fn safe_archive_segments(name: &str) -> Option<Vec<String>> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = Path::new(trimmed);
    let mut segments = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                let part = part.to_string_lossy();
                let sanitized = node::sanitize_node_name(&part);
                if sanitized.is_empty() {
                    return None;
                }
                segments.push(sanitized);
            }
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return None,
        }
    }
    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

fn normalize_entry_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_matches('/')
        .trim_start_matches("./")
        .to_string()
}

fn vnode_child_path(parent_path: &str, name: &str) -> String {
    if parent_path == "/" {
        format!("/{name}")
    } else {
        format!("{parent_path}/{name}")
    }
}

async fn load_parent(
    db: &DatabaseConnection,
    parent_id: Option<i64>,
) -> Result<Option<VNode>, String> {
    match parent_id {
        Some(id) => node::get_by_id(db, id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "parent directory not found".to_string())
            .map(Some),
        None => Ok(None),
    }
}

async fn ensure_dirs(
    db: &DatabaseConnection,
    store: &DynFilestore,
    mut parent_id: Option<i64>,
    mut parent_path: String,
    segments: &[String],
    created: &mut Vec<ExtractedVNode>,
) -> Result<(Option<i64>, String), String> {
    for seg in segments {
        if let Some(file) = node::find_child(db, parent_id, seg, false)
            .await
            .map_err(|e| e.to_string())?
        {
            return Err(format!(
                "\"{}\" is a file, not a directory",
                vnode_child_path(&parent_path, &file.name)
            ));
        }
        let (dir, newly_created) = if let Some(existing) =
            node::find_child(db, parent_id, seg, true)
                .await
                .map_err(|e| e.to_string())?
        {
            (existing, false)
        } else {
            let parent = load_parent(db, parent_id).await?;
            (
                node::create(db, store, seg.clone(), true, None, parent.as_ref())
                    .await
                    .map_err(|e| e.to_string())?,
                true,
            )
        };
        parent_path = vnode_child_path(&parent_path, &dir.name);
        parent_id = Some(dir.id);
        if newly_created {
            created.push(ExtractedVNode::from_node(&dir, parent_path.clone(), 0));
        }
    }
    Ok((parent_id, parent_path))
}

async fn write_file(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
    parent_path: &str,
    name: String,
    data: Vec<u8>,
) -> Result<ExtractedVNode, String> {
    if let Some(dir) = node::find_child(db, parent_id, &name, true)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err(format!(
            "\"{}\" is a directory, not a file",
            vnode_child_path(parent_path, &dir.name)
        ));
    }
    let bytes = data.len();
    let payload = NodeFile::Bytes {
        filename: name.clone(),
        data,
    };
    let parent = load_parent(db, parent_id).await?;
    let node = if let Some(existing) = node::find_child(db, parent_id, &name, false)
        .await
        .map_err(|e| e.to_string())?
    {
        node::update(db, store, existing, name, Some(payload))
            .await
            .map_err(|e| e.to_string())?
    } else {
        node::create(db, store, name, false, Some(payload), parent.as_ref())
            .await
            .map_err(|e| e.to_string())?
    };
    Ok(ExtractedVNode::from_node(
        &node,
        vnode_child_path(parent_path, &node.name),
        bytes,
    ))
}

pub(crate) fn list_archive_bytes(
    bytes: &[u8],
    password: Option<&str>,
) -> Result<Vec<ArchiveListEntry>, String> {
    collect_entries(bytes, password).map(|entries| {
        entries
            .into_iter()
            .map(|entry| ArchiveListEntry {
                path: entry.path,
                size: if entry.size == 0 && !entry.is_directory {
                    u64::try_from(entry.data.len()).unwrap_or(0)
                } else {
                    entry.size
                },
                is_directory: entry.is_directory,
            })
            .collect()
    })
}

pub(crate) async fn extract_all(
    db: &DatabaseConnection,
    store: &DynFilestore,
    bytes: &[u8],
    password: Option<&str>,
    dest: Option<&VNode>,
    dest_path: &str,
) -> Result<Vec<ExtractedVNode>, String> {
    let entries = collect_entries(bytes, password)?;
    let dest_id = dest.map(|d| d.id);
    let mut items = Vec::new();
    for entry in entries {
        let Some(mut segments) = safe_archive_segments(&entry.path) else {
            continue;
        };
        if entry.is_directory {
            ensure_dirs(
                db,
                store,
                dest_id,
                dest_path.to_string(),
                &segments,
                &mut items,
            )
            .await?;
            continue;
        }
        let Some(name) = segments.pop() else {
            continue;
        };
        let (parent_id, parent_path) = ensure_dirs(
            db,
            store,
            dest_id,
            dest_path.to_string(),
            &segments,
            &mut items,
        )
        .await?;
        items.push(write_file(db, store, parent_id, &parent_path, name, entry.data).await?);
    }
    Ok(items)
}

pub(crate) async fn extract_single(
    db: &DatabaseConnection,
    store: &DynFilestore,
    bytes: &[u8],
    password: Option<&str>,
    target_file: &str,
    dest: Option<&VNode>,
    dest_path: &str,
) -> Result<ExtractedVNode, String> {
    let target = normalize_entry_path(target_file);
    if target.is_empty() {
        return Err("target_file is required".into());
    }
    let entries = collect_entries(bytes, password)?;
    let entry = entries
        .into_iter()
        .find(|entry| normalize_entry_path(&entry.path) == target)
        .ok_or_else(|| format!("target file \"{target_file}\" not found in archive"))?;
    if entry.is_directory {
        return Err(format!("\"{target_file}\" is a directory, not a file"));
    }
    let Some(segments) = safe_archive_segments(&entry.path) else {
        return Err(format!("target file \"{target_file}\" has an unsafe path"));
    };
    let Some(name) = segments.last().cloned() else {
        return Err(format!(
            "target file \"{target_file}\" not found in archive"
        ));
    };
    write_file(db, store, dest.map(|d| d.id), dest_path, name, entry.data).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Arc;

    use sea_orm::{ConnectionTrait, Database, Schema};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use crate::plugins::filesystem::entities::filesystem_node;
    use crate::plugins::filesystem::storage::LocalFilestore;

    fn test_zip(entries: &[(&str, Option<&[u8]>)]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut buf));
            let options = SimpleFileOptions::default();
            for (path, data) in entries {
                if let Some(data) = data {
                    writer.start_file(*path, options).expect("start file");
                    writer.write_all(data).expect("write file");
                } else {
                    writer.add_directory(*path, options).expect("add dir");
                }
            }
            writer.finish().expect("finish zip");
        }
        buf
    }

    #[test]
    fn safe_archive_segments_rejects_traversal_and_absolute() {
        assert!(safe_archive_segments("../secret.txt").is_none());
        assert!(safe_archive_segments("/etc/passwd").is_none());
        assert!(safe_archive_segments("").is_none());
        assert!(safe_archive_segments(".").is_none());
        assert_eq!(
            safe_archive_segments("docs/readme.md").as_deref(),
            Some(["docs".to_string(), "readme.md".to_string()].as_slice())
        );
        assert_eq!(
            safe_archive_segments("docs/").as_deref(),
            Some(["docs".to_string()].as_slice())
        );
    }

    #[test]
    fn lists_zip_entries_with_sizes() {
        let bytes = test_zip(&[
            ("docs/", None),
            ("docs/readme.md", Some(b"hello")),
            ("a.txt", Some(b"abc")),
        ]);
        let files = list_archive_bytes(&bytes, None).expect("list");
        let readme = files
            .iter()
            .find(|e| e.path == "docs/readme.md" || e.path.ends_with("readme.md"))
            .expect("readme");
        assert_eq!(readme.size, 5);
        assert!(!readme.is_directory);
        let plain = files
            .iter()
            .find(|e| e.path == "a.txt" || e.path.ends_with("a.txt"))
            .expect("a.txt");
        assert_eq!(plain.size, 3);
        assert!(
            files
                .iter()
                .any(|e| e.is_directory && e.path.contains("docs")),
            "expected docs directory in {files:?}"
        );
    }

    #[test]
    fn rejects_non_archive_bytes() {
        let err = list_archive_bytes(b"not an archive", None).expect_err("non-archive");
        assert!(
            err.contains("not a valid archive"),
            "unexpected error: {err}"
        );
    }

    async fn setup_fs() -> (sea_orm::DatabaseConnection, Arc<DynFilestore>) {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        db.execute(&schema.create_table_from_entity(filesystem_node::Entity))
            .await
            .expect("create table");
        let dir = std::env::temp_dir().join(format!("lariv-archive-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let store: Arc<DynFilestore> = Arc::new(LocalFilestore::new(dir.to_string_lossy()));
        (db, store)
    }

    #[tokio::test]
    async fn extract_all_writes_nested_files_and_skips_traversal() {
        let (db, store) = setup_fs().await;
        let dest = node::create(&db, store.as_ref(), "out".into(), true, None, None)
            .await
            .expect("dest dir");
        let bytes = test_zip(&[
            ("docs/", None),
            ("docs/readme.md", Some(b"hello")),
            ("../escape.txt", Some(b"nope")),
        ]);
        let items = extract_all(&db, store.as_ref(), &bytes, None, Some(&dest), "/out")
            .await
            .expect("extract");
        assert!(
            items
                .iter()
                .any(|i| i.path == "/out/docs/readme.md" && !i.is_directory),
            "expected extracted file in {items:?}"
        );
        assert!(
            !items.iter().any(|i| i.name.contains("escape")),
            "traversal entry should be skipped: {items:?}"
        );
        let readme = node::get_by_path(&db, "/out/docs/readme.md")
            .await
            .expect("path")
            .0
            .expect("readme vnode");
        let data = crate::plugins::filesystem::zip::read_file_bytes(store.as_ref(), &readme)
            .await
            .expect("bytes");
        assert_eq!(data, b"hello");
    }

    #[tokio::test]
    async fn extract_single_writes_basename_into_output_dir() {
        let (db, store) = setup_fs().await;
        let dest = node::create(&db, store.as_ref(), "out".into(), true, None, None)
            .await
            .expect("dest dir");
        let bytes = test_zip(&[("docs/readme.md", Some(b"hello"))]);
        let item = extract_single(
            &db,
            store.as_ref(),
            &bytes,
            None,
            "docs/readme.md",
            Some(&dest),
            "/out",
        )
        .await
        .expect("extract single");
        assert_eq!(item.path, "/out/readme.md");
        assert_eq!(item.bytes, 5);
        let (node, _) = node::get_by_path(&db, "/out/readme.md")
            .await
            .expect("path");
        assert!(node.is_some());
        assert!(
            node::get_by_path(&db, "/out/docs")
                .await
                .ok()
                .and_then(|(n, _)| n)
                .is_none()
        );
    }
}
