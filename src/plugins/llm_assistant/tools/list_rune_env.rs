//! `list_rune_env` — list registered bindings and Rune std surface.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::llm_assistant::genai::FunctionDeclaration,
    rune_env::standard_library_names,
};

pub struct ListRuneEnvTool;

#[async_trait]
impl LlmTool for ListRuneEnvTool {
    fn name(&self) -> &str {
        "list_rune_env"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "list_rune_env".into(),
            description:
                "List variable and function names available in the Rune assistant environment. \
                Use before writing run_rune scripts."
                    .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {}
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, _args: Value) -> Result<Value, String> {
        Ok(json!({
            "env_variables": ctx.rune_env.all_names(),
            "standard_library": standard_library_names(),
        }))
    }
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

    #[tokio::test]
    async fn empty_registry_lists_no_env_variables() {
        let cap = RuneEnvCapability::new();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = ToolCtx {
            db: &db,
            store,
            cse_api_key: "",
            cse_cx: "",
            rune_env: &cap,
            session_id: None,
        };
        let out = ListRuneEnvTool.run(&ctx, Value::Null).await.unwrap();
        assert_eq!(out["env_variables"], json!([]));
        assert!(out["standard_library"].as_array().unwrap().len() > 0);
    }
}
