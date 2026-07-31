//! `read_file` — read a VNode by path as UTF-8 text.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::{node, zip::read_file_bytes},
        llm_assistant::genai::FunctionDeclaration,
    },
};

pub struct ReadFileTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    path: String,
}

#[async_trait]
impl LlmTool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "read_file".into(),
            description:
                "Read the content of a virtual file VNode using its file path (e.g., /Skills/code.py)."
                    .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute VNode path" }
                },
                "required": ["path"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let path = parsed.path.trim();
        if path.is_empty() {
            return Err("file path is required".into());
        }
        let (node, _) = node::get_by_path(ctx.db, path)
            .await
            .map_err(|e| e.to_string())?;
        let Some(vnode) = node else {
            return Err(format!("file not found at path \"{path}\""));
        };
        if vnode.is_directory {
            return Err(format!("path \"{path}\" is a directory, not a file"));
        }
        let bytes = read_file_bytes(ctx.store.as_ref(), &vnode)
            .await
            .map_err(|e| e.to_string())?;
        let content = String::from_utf8_lossy(&bytes).into_owned();
        Ok(json!({ "content": content }))
    }
}
