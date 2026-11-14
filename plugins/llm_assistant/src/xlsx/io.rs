//! VNode reads and writes for workbook files, images, themes, and VBA.

use std::path::Path;
use std::sync::Arc;

use rust_xlsxwriter::XlsxError;
use sea_orm::DatabaseConnection;

use lariv_plugin_filesystem::node::{self, NodeFile};
use lariv_plugin_filesystem::storage::DynFilestore;
use lariv_plugin_filesystem::zip::read_file_bytes;

use crate::vnode_text::split_vnode_path;

pub(crate) fn read_vnode(
    db: &DatabaseConnection,
    store: &Arc<DynFilestore>,
    path: &str,
) -> Result<Vec<u8>, String> {
    let db = db.clone();
    let store = Arc::clone(store);
    let path = path.to_string();
    lariv_core::rune_env::block_on_async(async move {
        let (vnode, normalized) = node::get_by_path(&db, &path)
            .await
            .map_err(|e| e.to_string())?;
        let Some(vnode) = vnode else {
            return Err(format!("item not found at path \"{normalized}\""));
        };
        if vnode.is_directory {
            return Err(format!("path \"{normalized}\" is a directory, not a file"));
        }
        read_file_bytes(store.as_ref(), &vnode)
            .await
            .map_err(|e| e.to_string())
    })
}

pub(crate) async fn save_vnode(
    db: &DatabaseConnection,
    store: &DynFilestore,
    path: &str,
    bytes: Vec<u8>,
    sheets: usize,
) -> Result<serde_json::Value, String> {
    let (parent_segments, filename) = split_vnode_path(path)?;
    let filename = spreadsheet_filename(filename);
    let parent_id = node::ensure_directory_path(db, store, None, &parent_segments)
        .await
        .map_err(|e| e.to_string())?;
    let stored_path = join_vnode_path(&parent_segments, &filename);
    if node::find_child(db, parent_id, &filename, false)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err(format!("file already exists at \"{stored_path}\""));
    }
    if node::find_child(db, parent_id, &filename, true)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err(format!("path \"{stored_path}\" is a directory, not a file"));
    }
    let parent = match parent_id {
        Some(id) => Some(
            node::get_by_id(db, id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "parent directory not found".to_string())?,
        ),
        None => None,
    };
    let len = bytes.len();
    let vnode = node::create(
        db,
        store,
        filename.clone(),
        false,
        Some(NodeFile::Bytes {
            filename: filename.clone(),
            data: bytes,
        }),
        parent.as_ref(),
        None,
        None,
    )
    .await
    .map_err(|e| format!("save xlsx: {e}"))?;
    let path = node::get_path(db, &vnode).await;
    Ok(serde_json::json!({
        "id": vnode.id,
        "name": vnode.name,
        "path": path,
        "bytes": len,
        "sheets": sheets,
    }))
}

pub(crate) fn with_temp_file<T>(
    bytes: &[u8],
    extension: &str,
    f: impl FnOnce(&Path) -> Result<T, XlsxError>,
) -> Result<T, String> {
    let path = std::env::temp_dir().join(format!("lariv-xlsx-{}{extension}", uuid::Uuid::new_v4()));
    std::fs::write(&path, bytes).map_err(|e| format!("temp file: {e}"))?;
    let outcome = f(&path).map_err(|e| e.to_string());
    match std::fs::remove_file(&path) {
        Ok(()) => outcome,
        Err(e) if outcome.is_ok() => Err(format!("remove temp file: {e}")),
        Err(_) => outcome,
    }
}

fn spreadsheet_filename(name: String) -> String {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".xlsx") || lower.ends_with(".xlsm") {
        name
    } else {
        format!("{name}.xlsx")
    }
}

fn join_vnode_path(parents: &[String], filename: &str) -> String {
    let mut path = String::new();
    for segment in parents {
        path.push('/');
        path.push_str(segment);
    }
    path.push('/');
    path.push_str(filename);
    path
}
