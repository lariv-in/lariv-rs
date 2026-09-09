//! `download_file` — fetch a public URL and save it as a VNode at a path.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::node::{self, NodeFile},
        llm_assistant::{
            content::attachments::resolve_mime,
            genai::FunctionDeclaration,
            vnode_text::{record_read, require_session_id, split_vnode_path},
        },
    },
};

use super::http_fetch::{FetchOptions, Fetched, fetch_public_url};

const FETCH_TIMEOUT_SECS: u64 = 30;
const MAX_BYTES: usize = 10 * 1024 * 1024;
const USER_AGENT: &str = "LarivAssistant/0.1 (download_file)";
const ACCEPT: &str = "*/*";
const DOWNLOAD_URL_EXAMPLE: &str = "http://localhost:42069/filesystem/3/download";

pub struct DownloadFileTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    url: String,
    #[serde(default)]
    path: String,
}

#[async_trait]
impl LlmTool for DownloadFileTool {
    fn name(&self) -> &str {
        "download_file"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "download_file".into(),
            description: format!(
                "Download a public http(s) file and save it as a virtual file (VNode) at an \
                absolute path. Missing parent directories are created. Fails if a file or \
                directory already exists at that path — never overwrites. Private, local, and \
                credentialed URLs are rejected. Files larger than {MAX_BYTES} bytes are rejected. \
                Returns JSON with `vnode_id`, `name`, `path`, `bytes`, `content_type`, `url`, \
                and `download_url` (example: {DOWNLOAD_URL_EXAMPLE}). To inspect the saved file, \
                call `read_vnode` for UTF-8 text or `attach_vnode_to_context` for PDFs, images, \
                and other binaries."
            ),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "http(s) URL of the file to download"
                    },
                    "path": {
                        "type": "string",
                        "description": "Absolute destination VNode path (e.g. /downloads/report.pdf)"
                    }
                },
                "required": ["url", "path"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let url = parsed.url.trim();
        if url.is_empty() {
            return Err("url is required".into());
        }
        let path = parsed.path.trim();
        if path.is_empty() {
            return Err("path is required".into());
        }

        let fetched = fetch_public_url(
            url,
            &FetchOptions {
                user_agent: USER_AGENT,
                accept: ACCEPT,
                max_bytes: MAX_BYTES,
                timeout: Duration::from_secs(FETCH_TIMEOUT_SECS),
                error_prefix: "download_file",
            },
        )
        .await?;
        store_at_path(ctx, path, fetched).await
    }
}

pub(super) async fn store_at_path(
    ctx: &ToolCtx<'_>,
    path: &str,
    fetched: Fetched,
) -> Result<Value, String> {
    let session_id = require_session_id(ctx)?;
    let path = path.trim();
    if path.is_empty() {
        return Err("path is required".into());
    }
    if fetched.body.is_empty() {
        return Err("downloaded file is empty".into());
    }

    let (parent_segments, filename) = split_vnode_path(path)?;
    let parent_id = node::ensure_directory_path(ctx.db, ctx.store.as_ref(), None, &parent_segments)
        .await
        .map_err(|e| e.to_string())?;

    if node::find_child(ctx.db, parent_id, &filename, false)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err(format!("file already exists at \"{path}\""));
    }
    if node::find_child(ctx.db, parent_id, &filename, true)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err(format!("path \"{path}\" is a directory, not a file"));
    }

    let parent = match parent_id {
        Some(id) => Some(
            node::get_by_id(ctx.db, id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "parent directory not found".to_string())?,
        ),
        None => None,
    };
    let bytes = fetched.body;
    let content_type = resolve_mime(&fetched.content_type, &filename, &bytes);
    let vnode = node::create(
        ctx.db,
        ctx.store.as_ref(),
        filename.clone(),
        false,
        Some(NodeFile::Bytes {
            filename: filename.clone(),
            data: bytes.clone(),
        }),
        parent.as_ref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    record_read(ctx.db, session_id, &vnode, &bytes).await?;
    let stored_path = node::get_path(ctx.db, &vnode).await;
    Ok(json!({
        "vnode_id": vnode.id,
        "name": vnode.name,
        "path": stored_path,
        "bytes": bytes.len(),
        "url": fetched.url,
        "content_type": content_type,
        "download_url": download_url(vnode.id),
    }))
}

fn download_url(vnode_id: i64) -> String {
    format!("http://localhost:42069/filesystem/{vnode_id}/download")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::{
        llm_tools::ToolCtx,
        plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore},
        rune_env::RuneEnvCapability,
    };

    #[test]
    fn declaration_names_the_tool() {
        let decl = DownloadFileTool.declaration();
        assert_eq!(decl.name, "download_file");
        assert!(decl.description.contains("attach_vnode_to_context"));
        assert!(decl.description.contains("never overwrites"));
    }

    #[tokio::test]
    async fn missing_args_error() {
        let cap = RuneEnvCapability::new();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = ToolCtx {
            db: &db,
            store,
            cse_api_key: "",
            cse_cx: "",
            rune_env: &cap,
            hitl: None,
            hitl_gate: None,
            session_id: Some(1),
            genai: None,
        };
        let err = DownloadFileTool
            .run(&ctx, json!({ "path": "/downloads/a.pdf" }))
            .await
            .unwrap_err();
        assert!(err.contains("url is required"), "{err}");

        let err = DownloadFileTool
            .run(&ctx, json!({ "url": "https://example.com/a.pdf" }))
            .await
            .unwrap_err();
        assert!(err.contains("path is required"), "{err}");
    }

    #[tokio::test]
    async fn rejects_private_url_before_fetch() {
        let cap = RuneEnvCapability::new();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = ToolCtx {
            db: &db,
            store,
            cse_api_key: "",
            cse_cx: "",
            rune_env: &cap,
            hitl: None,
            hitl_gate: None,
            session_id: Some(1),
            genai: None,
        };
        let err = DownloadFileTool
            .run(
                &ctx,
                json!({
                    "url": "http://127.0.0.1/secret.pdf",
                    "path": "/downloads/secret.pdf"
                }),
            )
            .await
            .unwrap_err();
        assert!(err.contains("host is not allowed"), "{err}");
    }
}
