//! `create_vnode` — create a new text VNode; never overwrites an existing file.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::node::{self, NodeFile},
        llm_assistant::{
            genai::FunctionDeclaration,
            vnode_text::{
                record_read, require_session_id, require_text_content_size, split_vnode_path,
            },
        },
    },
};

pub struct CreateVnodeTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    path: String,
    #[serde(default)]
    content: String,
}

#[async_trait]
impl LlmTool for CreateVnodeTool {
    fn name(&self) -> &str {
        "create_vnode"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "create_vnode".into(),
            description: "Create a new text virtual file (VNode) at an absolute path. Missing \
                parent directories are created. Fails if a file or directory already exists at \
                that path — never overwrites; use edit_vnode to change an existing file. After \
                a successful create you may call edit_vnode without an extra read_vnode."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute VNode path to create (e.g. /notes/todo.md)"
                    },
                    "content": {
                        "type": "string",
                        "description": "UTF-8 file contents"
                    }
                },
                "required": ["path", "content"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let session_id = require_session_id(ctx)?;
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let path = parsed.path.trim();
        if path.is_empty() {
            return Err("path is required".into());
        }
        require_text_content_size(&parsed.content)?;

        let (parent_segments, filename) = split_vnode_path(path)?;
        let parent_id =
            node::ensure_directory_path(ctx.db, ctx.store.as_ref(), None, &parent_segments)
                .await
                .map_err(|e| e.to_string())?;

        if node::find_child(ctx.db, parent_id, &filename, false)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Err(format!("file already exists at \"{path}\"; use edit_vnode"));
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
        let bytes = parsed.content.into_bytes();
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
        }))
    }
}
