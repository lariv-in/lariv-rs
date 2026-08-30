//! `list_chat_attachments` — list VNodes attached on messages in the current conversation.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::node,
        llm_assistant::{chat_attachments, genai::FunctionDeclaration},
    },
};

pub struct ListChatAttachmentsTool;

#[async_trait]
impl LlmTool for ListChatAttachmentsTool {
    fn name(&self) -> &str {
        "list_chat_attachments"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "list_chat_attachments".into(),
            description: "List filesystem VNodes attached to messages in the current conversation \
                (device uploads and files selected from the filesystem). Returns each \
                file's id, name, and absolute VNode path. Use when you need vnode ids or \
                paths to pass to read_file or other tools."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {}
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, _args: Value) -> Result<Value, String> {
        let Some(session_id) = ctx.session_id.filter(|id| *id > 0) else {
            return Err("no active conversation session".into());
        };
        let refs = chat_attachments::list_session_attachment_refs(ctx.db, session_id)
            .await
            .map_err(|e| e.to_string())?;

        let mut out = Vec::with_capacity(refs.len());
        for att in refs {
            let Some(vnode) = node::get_by_id(ctx.db, att.vnode_id)
                .await
                .map_err(|e| e.to_string())?
            else {
                out.push(json!({
                    "id": att.vnode_id,
                    "name": att.display_name,
                    "path": Value::Null,
                    "missing": true,
                }));
                continue;
            };
            let path = node::get_path(ctx.db, &vnode).await;
            let name = if att.display_name.is_empty() {
                vnode.name
            } else {
                att.display_name
            };
            out.push(json!({
                "id": vnode.id,
                "name": name,
                "path": path,
            }));
        }
        Ok(json!({ "attachments": out }))
    }
}
