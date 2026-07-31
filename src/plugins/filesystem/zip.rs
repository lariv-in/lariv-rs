//! Zip archive helpers — download a directory subtree as a `.zip`, and replace a
//! directory's direct children with the contents of an uploaded `.zip` (Go
//! `p_filesystem` `views.go` `addVNodeToZip` / zip-upload handler).

use std::io::{Cursor, Write};

use sea_orm::DatabaseConnection;
use tokio::io::AsyncReadExt;
use zip::write::SimpleFileOptions;

use super::entities::VNode;
use super::node::{self, NodeError};
use super::storage::DynFilestore;

/// Recursively gather `(zip_path, file_bytes)` for every file under `parent_id`,
/// plus an empty-directory marker (`"name/"`, no bytes) for empty directories so
/// they round-trip through the archive.
async fn collect_files(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
    prefix: &str,
    out: &mut Vec<(String, Option<Vec<u8>>)>,
) -> Result<(), NodeError> {
    let children = node::list_children(db, parent_id, false, "").await?;
    for child in &children {
        if child.is_directory {
            let dir_path = format!("{prefix}{}/", child.name);
            out.push((dir_path.clone(), None));
            Box::pin(collect_files(db, store, Some(child.id), &dir_path, out)).await?;
        } else {
            let path = format!("{prefix}{}", child.name);
            let bytes = read_file_bytes(store, child).await?;
            out.push((path, Some(bytes)));
        }
    }
    Ok(())
}

/// Read a file VNode's bytes from the filestore (chat attachments, zip, etc.).
pub async fn read_file_bytes(
    store: &DynFilestore,
    node: &VNode,
) -> Result<Vec<u8>, NodeError> {
    let Some(path) = node.file_path.as_deref().filter(|p| !p.is_empty()) else {
        return Ok(Vec::new());
    };
    let mut download = store
        .open(path, &node.name)
        .await
        .map_err(NodeError::Store)?;
    let mut buf = Vec::new();
    download
        .reader
        .read_to_end(&mut buf)
        .await
        .map_err(|e| NodeError::Store(e.into()))?;
    Ok(buf)
}

/// Build a `.zip` archive of everything under `root` (`None` = filesystem root).
/// Returns `(zip_filename, bytes)`.
pub async fn build_zip(
    db: &DatabaseConnection,
    store: &DynFilestore,
    root: Option<&VNode>,
) -> Result<(String, Vec<u8>), NodeError> {
    let mut entries = Vec::new();
    collect_files(db, store, root.map(|n| n.id), "", &mut entries).await?;

    let filename = match root {
        Some(node) => format!("{}.zip", node.name),
        None => "filesystem.zip".to_string(),
    };

    let mut buf = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut buf));
        let options = SimpleFileOptions::default();
        for (path, bytes) in &entries {
            match bytes {
                Some(data) => {
                    writer
                        .start_file(path, options)
                        .map_err(zip_err_to_node_error)?;
                    writer.write_all(data).map_err(|e| {
                        NodeError::Store(super::storage::FilestoreError::Io(e))
                    })?;
                }
                None => {
                    writer
                        .add_directory(path, options)
                        .map_err(zip_err_to_node_error)?;
                }
            }
        }
        writer.finish().map_err(zip_err_to_node_error)?;
    }
    Ok((filename, buf))
}

fn zip_err_to_node_error(e: zip::result::ZipError) -> NodeError {
    NodeError::Store(super::storage::FilestoreError::Io(std::io::Error::other(
        e.to_string(),
    )))
}

/// Replace all direct children of `parent` (`None` = root) with the contents of
/// `zip_bytes`. Creates directories as needed; entries whose path contains a
/// `..` component are skipped (path traversal).
/// A single parsed zip entry, extracted with no borrows into the archive so the
/// data can be processed across `.await` points (`zip::read::ZipFile` is not `Send`).
struct ZipEntryData {
    /// Path segments excluding the final file name (empty for a top-level entry).
    dir_segments: Vec<String>,
    /// Empty for directory entries.
    file_name: String,
    is_dir: bool,
    bytes: Vec<u8>,
}

fn extract_zip_entries(zip_bytes: &[u8]) -> Result<Vec<ZipEntryData>, NodeError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(zip_err_to_node_error)?;
    let mut entries = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_err_to_node_error)?;
        let Some(name) = entry.enclosed_name() else {
            // `enclosed_name` returns None for absolute paths or paths containing `..`.
            continue;
        };
        let mut segments: Vec<String> = name
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if segments.is_empty() {
            continue;
        }
        let is_dir = entry.is_dir();
        let file_name = if is_dir {
            String::new()
        } else {
            segments.pop().unwrap_or_default()
        };
        let mut bytes = Vec::new();
        if !is_dir {
            std::io::Read::read_to_end(&mut entry, &mut bytes)
                .map_err(|e| NodeError::Store(super::storage::FilestoreError::Io(e)))?;
        }
        entries.push(ZipEntryData {
            dir_segments: segments,
            file_name,
            is_dir,
            bytes,
        });
    }
    Ok(entries)
}

pub async fn replace_children_from_zip(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent: Option<&VNode>,
    zip_bytes: &[u8],
) -> Result<(), NodeError> {
    let parent_id = parent.map(|p| p.id);
    node::delete_direct_children(db, store, parent_id).await?;

    let entries = extract_zip_entries(zip_bytes)?;
    for entry in entries {
        let dir_id = node::ensure_directory_path(db, store, parent_id, &entry.dir_segments).await?;
        if entry.is_dir || entry.file_name.is_empty() {
            continue;
        }
        let dir_model = match dir_id {
            Some(id) => node::get_by_id(db, id).await.map_err(NodeError::Db)?,
            None => None,
        };
        node::create(
            db,
            store,
            entry.file_name.clone(),
            false,
            Some(node::NodeFile::Bytes {
                filename: entry.file_name.clone(),
                data: entry.bytes,
            }),
            dir_model.as_ref(),
        )
        .await?;
    }
    Ok(())
}
