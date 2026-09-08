//! Text VNode helpers for `read_vnode` / `create_vnode` / `edit_vnode`.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::{
    llm_tools::ToolCtx,
    plugins::filesystem::{
        entities::VNode,
        node::{self, NodeError},
        zip::read_file_bytes,
    },
};

use super::{entities::session_vnode_read, gemini_file_cache};

/// Max bytes returned (or accepted) as VNode text content.
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;

pub fn require_session_id(ctx: &ToolCtx<'_>) -> Result<i64, String> {
    ctx.session_id
        .ok_or_else(|| "a chat session is required to read, create, or edit VNodes".to_string())
}

/// Split `/a/b/c.txt` into parent segments `["a","b"]` and filename `c.txt`.
pub fn split_vnode_path(raw: &str) -> Result<(Vec<String>, String), String> {
    let cleaned = raw.trim();
    if cleaned.is_empty() || cleaned == "/" {
        return Err("path is required".into());
    }
    let parts: Vec<&str> = cleaned.trim_matches('/').split('/').collect();
    let mut segments = Vec::new();
    for part in &parts {
        let name = node::sanitize_node_name(part);
        if name.is_empty() {
            return Err(format!("invalid path segment \"{part}\""));
        }
        segments.push(name);
    }
    let filename = segments
        .pop()
        .ok_or_else(|| "path is required".to_string())?;
    Ok((segments, filename))
}

/// UTF-8 text within [`MAX_TEXT_BYTES`].
pub fn decode_text(bytes: &[u8]) -> Result<&str, String> {
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(format!(
            "file is {} bytes; text reads are limited to {MAX_TEXT_BYTES} bytes. \
             Use attach_vnode_to_context for large or non-text files",
            bytes.len()
        ));
    }
    std::str::from_utf8(bytes).map_err(|_| {
        "file is not a text file (invalid UTF-8); use attach_vnode_to_context for binary files"
            .to_string()
    })
}

pub fn require_text_content_size(content: &str) -> Result<(), String> {
    if content.len() > MAX_TEXT_BYTES {
        return Err(format!(
            "content is {} bytes; text writes are limited to {MAX_TEXT_BYTES} bytes",
            content.len()
        ));
    }
    Ok(())
}

pub async fn resolve_file_vnode(db: &DatabaseConnection, path: &str) -> Result<VNode, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path is required".into());
    }
    let (node, _) = node::get_by_path(db, trimmed)
        .await
        .map_err(|e| e.to_string())?;
    let vnode = node.ok_or_else(|| format!("file not found at path \"{trimmed}\""))?;
    if vnode.is_directory {
        return Err(format!("path \"{trimmed}\" is a directory, not a file"));
    }
    Ok(vnode)
}

pub async fn record_read(
    db: &DatabaseConnection,
    session_id: i64,
    vnode: &VNode,
    bytes: &[u8],
) -> Result<(), String> {
    let now = Utc::now();
    let hash = gemini_file_cache::content_sha256(bytes);
    let existing = session_vnode_read::Entity::find()
        .filter(session_vnode_read::Column::SessionId.eq(session_id))
        .filter(session_vnode_read::Column::VnodeId.eq(vnode.id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;

    match existing {
        Some(row) => {
            let mut am: session_vnode_read::ActiveModel = row.into();
            am.updated_at = Set(Some(now));
            am.read_at = Set(now);
            am.vnode_updated_at = Set(vnode.updated_at);
            am.content_sha256 = Set(hash);
            am.update(db).await.map_err(|e| e.to_string())?;
        }
        None => {
            session_vnode_read::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                session_id: Set(session_id),
                vnode_id: Set(vnode.id),
                read_at: Set(now),
                vnode_updated_at: Set(vnode.updated_at),
                content_sha256: Set(hash),
            }
            .insert(db)
            .await
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Error unless this session has a last-read snapshot that still matches `vnode` + `bytes`.
pub async fn require_fresh_read(
    db: &DatabaseConnection,
    session_id: i64,
    vnode: &VNode,
    bytes: &[u8],
) -> Result<(), String> {
    let Some(row) = session_vnode_read::Entity::find()
        .filter(session_vnode_read::Column::SessionId.eq(session_id))
        .filter(session_vnode_read::Column::VnodeId.eq(vnode.id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("file has not yet been read in this session; call read_vnode first".into());
    };

    let hash = gemini_file_cache::content_sha256(bytes);
    let hash_changed = row.content_sha256 != hash;
    let updated_after_read = match (vnode.updated_at, row.vnode_updated_at) {
        (Some(current), Some(read_at)) => current > read_at,
        (Some(_), None) => true,
        _ => false,
    };
    if hash_changed || updated_after_read {
        return Err("file has been edited since the last read_vnode; call read_vnode again".into());
    }
    Ok(())
}

pub async fn load_text_file(ctx: &ToolCtx<'_>, vnode: &VNode) -> Result<(Vec<u8>, String), String> {
    let bytes = read_file_bytes(ctx.store.as_ref(), vnode)
        .await
        .map_err(|e: NodeError| e.to_string())?;
    let text = decode_text(&bytes)?.to_string();
    Ok((bytes, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_nested_path() {
        let (parents, name) = split_vnode_path("/notes/todo.md").unwrap();
        assert_eq!(parents, vec!["notes"]);
        assert_eq!(name, "todo.md");
    }

    #[test]
    fn split_root_file() {
        let (parents, name) = split_vnode_path("/todo.md").unwrap();
        assert!(parents.is_empty());
        assert_eq!(name, "todo.md");
    }

    #[test]
    fn split_strips_parent_refs() {
        let err = split_vnode_path("/a/../b.txt").unwrap_err();
        assert!(err.contains("invalid path segment"));
    }

    #[test]
    fn split_rejects_root_and_empty() {
        assert!(split_vnode_path("/").is_err());
        assert!(split_vnode_path("  ").is_err());
        assert!(split_vnode_path("").is_err());
    }

    #[test]
    fn decode_text_accepts_utf8() {
        assert_eq!(decode_text(b"hello").unwrap(), "hello");
        assert_eq!(decode_text("café".as_bytes()).unwrap(), "café");
    }

    #[test]
    fn decode_text_rejects_binary() {
        let err = decode_text(&[0xff, 0xfe, 0x00]).unwrap_err();
        assert!(err.contains("not a text file"));
    }

    #[test]
    fn decode_text_rejects_oversize() {
        let bytes = vec![b'a'; MAX_TEXT_BYTES + 1];
        let err = decode_text(&bytes).unwrap_err();
        assert!(err.contains("limited to"));
    }
}
