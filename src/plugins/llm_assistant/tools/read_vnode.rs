//! `read_vnode` — return a text VNode's contents and record a last-read snapshot.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::node,
        llm_assistant::{
            genai::FunctionDeclaration,
            vnode_text::{load_text_file, record_read, require_session_id, resolve_file_vnode},
        },
    },
};

pub struct ReadVnodeTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    path: String,
}

#[async_trait]
impl LlmTool for ReadVnodeTool {
    fn name(&self) -> &str {
        "read_vnode"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "read_vnode".into(),
            description: "Read a text virtual file (VNode) by absolute path and return its \
                UTF-8 contents. Only text files are supported — use attach_vnode_to_context \
                for PDFs, images, and other binary files. You must call this before edit_vnode \
                (and again if the file may have changed)."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute VNode path (e.g. /notes/todo.md)"
                    }
                },
                "required": ["path"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let session_id = require_session_id(ctx)?;
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let vnode = resolve_file_vnode(ctx.db, &parsed.path).await?;
        let (bytes, content) = load_text_file(ctx, &vnode).await?;
        record_read(ctx.db, session_id, &vnode, &bytes).await?;
        let path = node::get_path(ctx.db, &vnode).await;
        Ok(json!({
            "vnode_id": vnode.id,
            "name": vnode.name,
            "path": path,
            "content": content,
        }))
    }
}
