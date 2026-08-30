//! `move_vnode` — move a VNode (file or directory) to another directory by path.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{filesystem::node, llm_assistant::genai::FunctionDeclaration},
};

pub struct MoveVnodeTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    path: String,
    #[serde(default)]
    destination: String,
}

#[async_trait]
impl LlmTool for MoveVnodeTool {
    fn name(&self) -> &str {
        "move_vnode"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "move_vnode".into(),
            description: "Move a virtual file or directory VNode into another directory. \
                `path` is the item to move; `destination` is the target directory path \
                (`/` for the filesystem root). The item keeps its name."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute VNode path of the file or directory to move"
                    },
                    "destination": {
                        "type": "string",
                        "description": "Absolute VNode path of the destination directory (`/` for root)"
                    }
                },
                "required": ["path", "destination"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let path = parsed.path.trim();
        if path.is_empty() {
            return Err("path is required".into());
        }
        let dest_raw = parsed.destination.trim();
        if dest_raw.is_empty() {
            return Err("destination is required".into());
        }

        let (src, _) = node::get_by_path(ctx.db, path)
            .await
            .map_err(|e| e.to_string())?;
        let Some(src) = src else {
            return Err(format!("item not found at path \"{path}\""));
        };

        let (dest_node, dest_norm) = node::get_by_path(ctx.db, dest_raw)
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

        let moved = node::move_to(ctx.db, src, dest_node.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        let new_path = node::get_path(ctx.db, &moved).await;

        Ok(json!({
            "id": moved.id,
            "name": moved.name,
            "path": new_path,
            "is_directory": moved.is_directory,
        }))
    }
}
