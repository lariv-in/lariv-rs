//! `run_rune` — evaluate inline Rune source in the assistant VM.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::llm_assistant::{genai::FunctionDeclaration, rune_engine},
    rune_env::RuneEnvCtx,
};

pub struct RunRuneTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    source: String,
}

#[async_trait]
impl LlmTool for RunRuneTool {
    fn name(&self) -> &str {
        "run_rune"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "run_rune".into(),
            description: "Evaluate Rune source (https://github.com/rune-rs/rune). \
                Use for arithmetic, data transforms, or calls to registered lariv bindings. \
                Returns JSON `{result}` or `{error}`."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Rune source to compile and run (calls pub fn main when present)"
                    }
                },
                "required": ["source"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let source = parsed.source.trim();
        if source.is_empty() {
            return Err("source is required".into());
        }
        let env_ctx = RuneEnvCtx {
            db: ctx.db,
            store: Arc::clone(&ctx.store),
        };
        Ok(rune_engine::compile_and_run(ctx.rune_env, &env_ctx, source, &[]).await)
    }
}
