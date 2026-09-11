//! Rune sandbox bindings for filesystem and chat-attachment helpers.

use crate::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers assistant filesystem helpers onto the Rune environment.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl RuneEnvRegistrar for Hook {
    fn register_rune_env(self, rune_env: &mut RuneEnvCapability) {
        register(rune_env);
    }
}

fn register(rune_env: &mut RuneEnvCapability) {
    use std::sync::Arc;

    use crate::rune_env::NativeBinding;

    rune_env.register_contextual(
        "move_vnode",
        "move_vnode(#{ path: string, destination: string }) -> #{ id: int, name: string, path: string, is_directory: bool }  // move a VNode into another directory (`/` for root); the item keeps its name",
        |_ctx| NativeBinding::Function(Arc::new(move_vnode)),
    );
    rune_env.register_contextual(
        "read_bytes_file",
        "read_bytes_file(#{ path: string } | #{ id: int }) -> bytes  // read a VNode file's contents as bytes (by absolute path or id)",
        |_ctx| NativeBinding::Function(Arc::new(read_bytes_file)),
    );
    rune_env.register_contextual(
        "list_directory",
        "list_directory(#{ path: string } | #{ id: int }) -> #{ items: [#{ id: int, name: string, path: string, is_directory: bool }] }  // list a VNode directory's children (by absolute path or id; `/` is the root)",
        |_ctx| NativeBinding::Function(Arc::new(list_directory)),
    );
    rune_env.register_contextual(
        "list_chat_attachments",
        "list_chat_attachments(()) -> #{ attachments: [#{ id: int, name: string, path: string|null, missing?: bool }] }  // VNodes attached on messages in the current conversation",
        |_ctx| NativeBinding::Function(Arc::new(list_chat_attachments)),
    );
    rune_env.register_contextual(
        "unarchive_file",
        "unarchive_file(#{ archive: #{ path: string } | #{ id: int }, output_dir: #{ path: string } | #{ id: int }, password?: string }) -> #{ items: [#{ id: int, name: string, path: string, is_directory: bool }] }  // extract an archive VNode into a directory; rejects non-archives",
        |_ctx| NativeBinding::Function(Arc::new(unarchive_file)),
    );
    rune_env.register_contextual(
        "unarchive_single",
        "unarchive_single(#{ archive: #{ path: string } | #{ id: int }, target_file: string, output_dir: #{ path: string } | #{ id: int }, password?: string }) -> #{ id: int, name: string, path: string, bytes: int }  // extract one file from an archive VNode into a directory",
        |_ctx| NativeBinding::Function(Arc::new(unarchive_single)),
    );
    rune_env.register_contextual(
        "list_archive_files",
        "list_archive_files(#{ archive: #{ path: string } | #{ id: int }, password?: string }) -> #{ files: [#{ path: string, size: int, is_directory: bool }] }  // list archive entries with uncompressed sizes; rejects non-archives",
        |_ctx| NativeBinding::Function(Arc::new(list_archive_files)),
    );
    rune_env.register_contextual(
        "stat_file",
        "stat_file(#{ path: string } | #{ id: int }) -> #{ id: int, name: string, path: string, is_directory: bool, size: int, content_type: string }  // VNode metadata without reading file bytes",
        |_ctx| NativeBinding::Function(Arc::new(stat_file)),
    );
}

fn move_vnode(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;
    use serde_json::json;

    use crate::plugins::filesystem::node;
    use crate::rune_env::{block_on_async, json_to_rune, rune_to_json};

    #[derive(Debug, Deserialize, Default)]
    struct Args {
        #[serde(default)]
        path: String,
        #[serde(default)]
        destination: String,
    }

    let value = args
        .first()
        .ok_or_else(|| "move_vnode requires an object argument".to_string())?;
    let parsed: Args = serde_json::from_value(rune_to_json(value)?)
        .map_err(|e| format!("invalid move_vnode arguments: {e}"))?;
    let path = parsed.path.trim().to_string();
    if path.is_empty() {
        return Err("path is required".into());
    }
    let dest_raw = parsed.destination.trim().to_string();
    if dest_raw.is_empty() {
        return Err("destination is required".into());
    }
    let db = ctx.db.clone();
    let out = block_on_async(async move {
        let (src, _) = node::get_by_path(&db, &path)
            .await
            .map_err(|e| e.to_string())?;
        let Some(src) = src else {
            return Err(format!("item not found at path \"{path}\""));
        };

        let (dest_node, dest_norm) = node::get_by_path(&db, &dest_raw)
            .await
            .map_err(|e| e.to_string())?;
        if dest_norm != "/" {
            let Some(ref dest) = dest_node else {
                return Err(format!("destination not found at path \"{dest_raw}\""));
            };
            if !dest.is_directory {
                return Err(format!(
                    "destination \"{dest_norm}\" is a file, not a directory"
                ));
            }
        }

        let moved = node::move_to(&db, src, dest_node.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        let new_path = node::get_path(&db, &moved).await;
        Ok::<_, String>(json!({
            "id": moved.id,
            "name": moved.name,
            "path": new_path,
            "is_directory": moved.is_directory,
        }))
    })?;
    json_to_rune(out)
}

pub(crate) enum VNodeRef {
    Path(String),
    Id(i64),
}

pub(crate) fn parse_vnode_ref(
    value: &serde_json::Value,
    fn_name: &str,
) -> Result<VNodeRef, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| format!("{fn_name} requires an object argument"))?;
    let path = obj
        .get("path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let id = obj.get("id").and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_u64().and_then(|n| i64::try_from(n).ok()))
    });
    match (path, id) {
        (Some(path), None) => Ok(VNodeRef::Path(path.to_string())),
        (None, Some(id)) => Ok(VNodeRef::Id(id)),
        (Some(_), Some(_)) => Err("provide either path or id, not both".into()),
        (None, None) => Err("path or id is required".into()),
    }
}

fn child_path(parent_path: &str, name: &str) -> String {
    if parent_path == "/" {
        format!("/{name}")
    } else {
        format!("{parent_path}/{name}")
    }
}

fn optional_password(value: &serde_json::Value) -> Option<String> {
    value
        .get("password")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn required_field<'a>(
    value: &'a serde_json::Value,
    field: &str,
    fn_name: &str,
) -> Result<&'a serde_json::Value, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| format!("{fn_name} requires an object argument"))?;
    obj.get(field)
        .filter(|v| !v.is_null())
        .ok_or_else(|| format!("{field} is required"))
}

async fn resolve_file_vnode(
    db: &sea_orm::DatabaseConnection,
    parsed: VNodeRef,
) -> Result<crate::plugins::filesystem::entities::VNode, String> {
    use crate::plugins::filesystem::node;

    match parsed {
        VNodeRef::Path(path) => {
            let (node, norm) = node::get_by_path(db, &path)
                .await
                .map_err(|e| e.to_string())?;
            if norm == "/" {
                return Err("path \"/\" is the filesystem root, not a file".into());
            }
            let Some(vnode) = node else {
                return Err(format!("file not found at path \"{path}\""));
            };
            if vnode.is_directory {
                return Err(format!("path \"{norm}\" is a directory, not a file"));
            }
            Ok(vnode)
        }
        VNodeRef::Id(id) => {
            let Some(vnode) = node::get_by_id(db, id).await.map_err(|e| e.to_string())? else {
                return Err(format!("file not found with id {id}"));
            };
            if vnode.is_directory {
                return Err(format!("id {id} is a directory, not a file"));
            }
            Ok(vnode)
        }
    }
}

async fn resolve_dir_vnode(
    db: &sea_orm::DatabaseConnection,
    parsed: VNodeRef,
) -> Result<(Option<crate::plugins::filesystem::entities::VNode>, String), String> {
    use crate::plugins::filesystem::node;

    match parsed {
        VNodeRef::Path(path) => {
            let (node, norm) = node::get_by_path(db, &path)
                .await
                .map_err(|e| e.to_string())?;
            if norm == "/" {
                return Ok((None, "/".to_string()));
            }
            let Some(vnode) = node else {
                return Err(format!("directory not found at path \"{path}\""));
            };
            if !vnode.is_directory {
                return Err(format!("path \"{norm}\" is a file, not a directory"));
            }
            Ok((Some(vnode), norm))
        }
        VNodeRef::Id(id) => {
            let Some(vnode) = node::get_by_id(db, id).await.map_err(|e| e.to_string())? else {
                return Err(format!("directory not found with id {id}"));
            };
            if !vnode.is_directory {
                return Err(format!("id {id} is a file, not a directory"));
            }
            let path = node::get_path(db, &vnode).await;
            Ok((Some(vnode), path))
        }
    }
}

pub(crate) async fn resolve_any_vnode(
    db: &sea_orm::DatabaseConnection,
    parsed: VNodeRef,
) -> Result<(crate::plugins::filesystem::entities::VNode, String), String> {
    use crate::plugins::filesystem::node;

    match parsed {
        VNodeRef::Path(path) => {
            let (node, norm) = node::get_by_path(db, &path)
                .await
                .map_err(|e| e.to_string())?;
            if norm == "/" {
                return Err("path \"/\" is the filesystem root".into());
            }
            let Some(vnode) = node else {
                return Err(format!("item not found at path \"{path}\""));
            };
            Ok((vnode, norm))
        }
        VNodeRef::Id(id) => {
            let Some(vnode) = node::get_by_id(db, id).await.map_err(|e| e.to_string())? else {
                return Err(format!("item not found with id {id}"));
            };
            let path = node::get_path(db, &vnode).await;
            Ok((vnode, path))
        }
    }
}

fn unarchive_file(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use std::sync::Arc;

    use serde_json::json;

    use crate::plugins::filesystem::zip::read_file_bytes;
    use crate::rune_env::{block_on_async, json_to_rune, rune_to_json};

    let value = args
        .first()
        .ok_or_else(|| "unarchive_file requires an object argument".to_string())?;
    let parsed = rune_to_json(value)?;
    let archive = parse_vnode_ref(
        required_field(&parsed, "archive", "unarchive_file")?,
        "archive",
    )?;
    let output_dir = parse_vnode_ref(
        required_field(&parsed, "output_dir", "unarchive_file")?,
        "output_dir",
    )?;
    let password = optional_password(&parsed);

    let db = ctx.db.clone();
    let store = Arc::clone(&ctx.store);
    let out = block_on_async(async move {
        let archive_node = resolve_file_vnode(&db, archive).await?;
        let bytes = read_file_bytes(store.as_ref(), &archive_node)
            .await
            .map_err(|e| e.to_string())?;
        let (dest, dest_path) = resolve_dir_vnode(&db, output_dir).await?;
        let items = super::archive::extract_all(
            &db,
            store.as_ref(),
            &bytes,
            password.as_deref(),
            dest.as_ref(),
            &dest_path,
        )
        .await?;
        Ok::<_, String>(json!({
            "items": items.iter().map(super::archive::ExtractedVNode::to_json).collect::<Vec<_>>(),
        }))
    })?;
    json_to_rune(out)
}

fn unarchive_single(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use std::sync::Arc;

    use crate::plugins::filesystem::zip::read_file_bytes;
    use crate::rune_env::{block_on_async, json_to_rune, rune_to_json};

    let value = args
        .first()
        .ok_or_else(|| "unarchive_single requires an object argument".to_string())?;
    let parsed = rune_to_json(value)?;
    let archive = parse_vnode_ref(
        required_field(&parsed, "archive", "unarchive_single")?,
        "archive",
    )?;
    let output_dir = parse_vnode_ref(
        required_field(&parsed, "output_dir", "unarchive_single")?,
        "output_dir",
    )?;
    let target_file = parsed
        .get("target_file")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "target_file is required".to_string())?
        .to_string();
    let password = optional_password(&parsed);

    let db = ctx.db.clone();
    let store = Arc::clone(&ctx.store);
    let out = block_on_async(async move {
        let archive_node = resolve_file_vnode(&db, archive).await?;
        let bytes = read_file_bytes(store.as_ref(), &archive_node)
            .await
            .map_err(|e| e.to_string())?;
        let (dest, dest_path) = resolve_dir_vnode(&db, output_dir).await?;
        let item = super::archive::extract_single(
            &db,
            store.as_ref(),
            &bytes,
            password.as_deref(),
            &target_file,
            dest.as_ref(),
            &dest_path,
        )
        .await?;
        Ok::<_, String>(item.to_single_json())
    })?;
    json_to_rune(out)
}

fn list_archive_files(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use std::sync::Arc;

    use serde_json::json;

    use crate::plugins::filesystem::zip::read_file_bytes;
    use crate::rune_env::{block_on_async, json_to_rune, rune_to_json};

    let value = args
        .first()
        .ok_or_else(|| "list_archive_files requires an object argument".to_string())?;
    let parsed = rune_to_json(value)?;
    let archive = parse_vnode_ref(
        required_field(&parsed, "archive", "list_archive_files")?,
        "archive",
    )?;
    let password = optional_password(&parsed);

    let db = ctx.db.clone();
    let store = Arc::clone(&ctx.store);
    let out = block_on_async(async move {
        let archive_node = resolve_file_vnode(&db, archive).await?;
        let bytes = read_file_bytes(store.as_ref(), &archive_node)
            .await
            .map_err(|e| e.to_string())?;
        let files = super::archive::list_archive_bytes(&bytes, password.as_deref())?;
        Ok::<_, String>(json!({
            "files": files.iter().map(|f| json!({
                "path": f.path,
                "size": f.size,
                "is_directory": f.is_directory,
            })).collect::<Vec<_>>(),
        }))
    })?;
    json_to_rune(out)
}

fn stat_file(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use std::sync::Arc;

    use serde_json::json;

    use crate::rune_env::{block_on_async, json_to_rune, rune_to_json};

    let value = args
        .first()
        .ok_or_else(|| "stat_file requires an object argument".to_string())?;
    let parsed = parse_vnode_ref(&rune_to_json(value)?, "stat_file")?;

    let db = ctx.db.clone();
    let store = Arc::clone(&ctx.store);
    let out = block_on_async(async move {
        let (vnode, path) = resolve_any_vnode(&db, parsed).await?;
        let (size, content_type) = if vnode.is_directory {
            (0u64, "inode/directory".to_string())
        } else {
            let content_type = mime_guess::from_path(&vnode.name)
                .first_or_octet_stream()
                .to_string();
            let size = match vnode.file_path.as_deref().filter(|p| !p.is_empty()) {
                Some(blob) => match store.stored_size(blob).await {
                    Ok(n) => n,
                    Err(e) if e.is_missing() => 0,
                    Err(e) => return Err(e.to_string()),
                },
                None => 0,
            };
            (size, content_type)
        };
        Ok::<_, String>(json!({
            "id": vnode.id,
            "name": vnode.name,
            "path": path,
            "is_directory": vnode.is_directory,
            "size": size,
            "content_type": content_type,
        }))
    })?;
    json_to_rune(out)
}

fn read_bytes_file(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use std::sync::Arc;

    use crate::plugins::filesystem::{node, zip::read_file_bytes};
    use crate::rune_env::{block_on_async, rune_to_json};

    let value = args
        .first()
        .ok_or_else(|| "read_bytes_file requires an object argument".to_string())?;
    let parsed = parse_vnode_ref(&rune_to_json(value)?, "read_bytes_file")?;

    let db = ctx.db.clone();
    let store = Arc::clone(&ctx.store);
    let data = block_on_async(async move {
        let vnode = match parsed {
            VNodeRef::Path(path) => {
                let (node, _) = node::get_by_path(&db, &path)
                    .await
                    .map_err(|e| e.to_string())?;
                let Some(vnode) = node else {
                    return Err(format!("file not found at path \"{path}\""));
                };
                if vnode.is_directory {
                    return Err(format!("path \"{path}\" is a directory, not a file"));
                }
                vnode
            }
            VNodeRef::Id(id) => {
                let Some(vnode) = node::get_by_id(&db, id).await.map_err(|e| e.to_string())? else {
                    return Err(format!("file not found with id {id}"));
                };
                if vnode.is_directory {
                    return Err(format!("id {id} is a directory, not a file"));
                }
                vnode
            }
        };
        read_file_bytes(store.as_ref(), &vnode)
            .await
            .map_err(|e| e.to_string())
    })?;
    let bytes = rune::runtime::Bytes::try_from(data).map_err(|e| e.to_string())?;
    rune::to_value(bytes).map_err(|e| e.to_string())
}

fn list_directory(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde_json::json;

    use crate::plugins::filesystem::node;
    use crate::rune_env::{block_on_async, json_to_rune, rune_to_json};

    let value = args
        .first()
        .ok_or_else(|| "list_directory requires an object argument".to_string())?;
    let parsed = parse_vnode_ref(&rune_to_json(value)?, "list_directory")?;

    let db = ctx.db.clone();
    let out = block_on_async(async move {
        let (parent_id, parent_path) = match parsed {
            VNodeRef::Path(path) => {
                let (node, norm) = node::get_by_path(&db, &path)
                    .await
                    .map_err(|e| e.to_string())?;
                if norm == "/" {
                    (None, "/".to_string())
                } else {
                    let Some(vnode) = node else {
                        return Err(format!("directory not found at path \"{path}\""));
                    };
                    if !vnode.is_directory {
                        return Err(format!("path \"{norm}\" is a file, not a directory"));
                    }
                    (Some(vnode.id), norm)
                }
            }
            VNodeRef::Id(id) => {
                let Some(vnode) = node::get_by_id(&db, id).await.map_err(|e| e.to_string())? else {
                    return Err(format!("directory not found with id {id}"));
                };
                if !vnode.is_directory {
                    return Err(format!("id {id} is a file, not a directory"));
                }
                let path = node::get_path(&db, &vnode).await;
                (Some(vnode.id), path)
            }
        };
        let children = node::list_children(&db, parent_id, false, "")
            .await
            .map_err(|e| e.to_string())?;
        let items: Vec<_> = children
            .into_iter()
            .map(|child| {
                json!({
                    "id": child.id,
                    "name": child.name,
                    "path": child_path(&parent_path, &child.name),
                    "is_directory": child.is_directory,
                })
            })
            .collect();
        Ok::<_, String>(json!({ "items": items }))
    })?;
    json_to_rune(out)
}

fn list_chat_attachments(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    _args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde_json::json;

    use crate::plugins::filesystem::node;
    use crate::plugins::llm_assistant::chat_attachments;
    use crate::rune_env::{block_on_async, json_to_rune};

    let Some(session_id) = ctx.session_id.filter(|id| *id > 0) else {
        return Err("no active conversation session".into());
    };
    let db = ctx.db.clone();
    let out = block_on_async(async move {
        let refs = chat_attachments::list_session_attachment_refs(&db, session_id)
            .await
            .map_err(|e| e.to_string())?;
        let mut attachments = Vec::with_capacity(refs.len());
        for att in refs {
            let Some(vnode) = node::get_by_id(&db, att.vnode_id)
                .await
                .map_err(|e| e.to_string())?
            else {
                attachments.push(json!({
                    "id": att.vnode_id,
                    "name": att.display_name,
                    "path": serde_json::Value::Null,
                    "missing": true,
                }));
                continue;
            };
            let path = node::get_path(&db, &vnode).await;
            let name = if att.display_name.is_empty() {
                vnode.name
            } else {
                att.display_name
            };
            attachments.push(json!({
                "id": vnode.id,
                "name": name,
                "path": path,
            }));
        }
        Ok::<_, String>(json!({ "attachments": attachments }))
    })?;
    json_to_rune(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use crate::plugins::llm_assistant::rune_engine;
    use crate::rune_env::{RuneEnvCapability, RuneEnvCtx};

    fn test_env_ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: &'a Arc<DynFilestore>,
        session_id: Option<i64>,
    ) -> RuneEnvCtx<'a> {
        RuneEnvCtx {
            db,
            store: Arc::clone(store),
            session_id,
        }
    }

    fn registered_env() -> RuneEnvCapability {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        cap
    }

    #[test]
    fn registers_filesystem_bindings() {
        let names = registered_env().all_names();
        for expected in [
            "move_vnode",
            "read_bytes_file",
            "list_directory",
            "list_chat_attachments",
            "unarchive_file",
            "unarchive_single",
            "list_archive_files",
            "stat_file",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "expected {expected} in {names:?}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn move_vnode_rejects_missing_path() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "move_vnode(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(error.contains("path"), "unexpected error payload: {out}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn read_bytes_file_rejects_missing_path_or_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "read_bytes_file(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("path or id is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn read_bytes_file_rejects_path_and_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"read_bytes_file(#{ path: "/docs/a.txt", id: 1 })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("either path or id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_directory_rejects_missing_path_or_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "list_directory(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("path or id is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_directory_rejects_path_and_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"list_directory(#{ path: "/docs", id: 1 })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("either path or id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_chat_attachments_rejects_missing_session() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, "list_chat_attachments(())", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("no active conversation session"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unarchive_file_rejects_missing_archive() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "unarchive_file(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("archive is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unarchive_file_rejects_archive_path_and_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"unarchive_file(#{ archive: #{ path: "/a.zip", id: 1 }, output_dir: #{ path: "/out" } })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("either path or id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unarchive_single_rejects_missing_target_file() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"unarchive_single(#{ archive: #{ path: "/a.zip" }, output_dir: #{ path: "/out" } })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("target_file"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_archive_files_rejects_missing_archive() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, "list_archive_files(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("archive is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stat_file_rejects_missing_path_or_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store, None);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "stat_file(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("path or id is required"),
            "unexpected error payload: {out}"
        );
    }
}
