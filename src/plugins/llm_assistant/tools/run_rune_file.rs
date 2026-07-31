//! `run_rune_file` — read a VNode script and evaluate it with Rune.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::{node, zip::read_file_bytes},
        llm_assistant::{genai::FunctionDeclaration, rune_engine},
    },
    rune_env::RuneEnvCtx,
};

pub struct RunRuneFileTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    path: String,
    #[serde(default)]
    args: HashMap<String, Value>,
}

#[async_trait]
impl LlmTool for RunRuneFileTool {
    fn name(&self) -> &str {
        "run_rune_file"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "run_rune_file".into(),
            description: "Read and evaluate a Rune script stored in a virtual file (VNode). \
                Optional args are merged into the script environment before execution."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute VNode path" },
                    "args": {
                        "type": "object",
                        "description": "Optional variables injected as let-bindings before the script"
                    }
                },
                "required": ["path"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let path = parsed.path.trim();
        if path.is_empty() {
            return Err("path is required".into());
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
        let source = String::from_utf8_lossy(&bytes);

        let extra: Vec<(String, Value)> = parsed
            .args
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect();

        let env_ctx = RuneEnvCtx {
            db: ctx.db,
            store: std::sync::Arc::clone(&ctx.store),
        };
        Ok(rune_engine::compile_and_run(ctx.rune_env, &env_ctx, &source, &extra).await)
    }
}
