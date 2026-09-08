//! `edit_vnode` — replace a text VNode; requires a fresh last-read snapshot.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::{
            node::{self, NodeFile},
            zip::read_file_bytes,
        },
        llm_assistant::{
            genai::FunctionDeclaration,
            vnode_text::{
                decode_text, record_read, require_fresh_read, require_session_id,
                require_text_content_size, resolve_file_vnode,
            },
        },
    },
};

pub struct EditVnodeTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    path: String,
    #[serde(default)]
    content: String,
}

#[async_trait]
impl LlmTool for EditVnodeTool {
    fn name(&self) -> &str {
        "edit_vnode"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "edit_vnode".into(),
            description: "Replace the contents of an existing text virtual file (VNode). \
                Fails if the file has not been read with read_vnode (or created with \
                create_vnode) in this session, if it changed since that read, or if it is \
                not a UTF-8 text file. After a successful edit you may edit again without \
                another read_vnode."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute VNode path to edit (e.g. /notes/todo.md)"
                    },
                    "content": {
                        "type": "string",
                        "description": "New UTF-8 file contents (full replacement)"
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

        let vnode = resolve_file_vnode(ctx.db, path).await?;
        let current = read_file_bytes(ctx.store.as_ref(), &vnode)
            .await
            .map_err(|e| e.to_string())?;
        decode_text(&current)?;
        require_fresh_read(ctx.db, session_id, &vnode, &current).await?;

        let bytes = parsed.content.into_bytes();
        let filename = vnode.name.clone();
        let vnode = node::update(
            ctx.db,
            ctx.store.as_ref(),
            vnode,
            filename.clone(),
            Some(NodeFile::Bytes {
                filename,
                data: bytes.clone(),
            }),
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
