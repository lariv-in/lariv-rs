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

enum VNodeRef {
    Path(String),
    Id(i64),
}

fn parse_vnode_ref(value: &serde_json::Value, fn_name: &str) -> Result<VNodeRef, String> {
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
}
