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
                HITL functions require human approval before they run. Use get_rune_env with a \
                name to get that identifier's kind and schema. Use before writing run_rune scripts."
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
            "hitl_functions": ctx.hitl.map(|h| h.all_names()).unwrap_or_default(),
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
            hitl: None,
            hitl_gate: None,
            session_id: None,
        };
        let out = ListRuneEnvTool.run(&ctx, Value::Null).await.unwrap();
        assert_eq!(out["env_variables"], json!([]));
        assert_eq!(out["hitl_functions"], json!([]));
        assert!(out["standard_library"].as_array().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn lists_registered_hitl_function_names() {
        use crate::plugins::llm_assistant::hitl::HitlCapability;
        use crate::rune_env::NativeBinding;

        let cap = RuneEnvCapability::new();
        let mut hitl = HitlCapability::new();
        hitl.register(
            "delete_draft_invoice",
            "delete_draft_invoice(#{ id: int }) -> ()",
            |_ctx| NativeBinding::Function(Arc::new(|_ctx, _args| Ok(rune::Value::from(())))),
        );
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = ToolCtx {
            db: &db,
            store,
            cse_api_key: "",
            cse_cx: "",
            rune_env: &cap,
            hitl: Some(&hitl),
            hitl_gate: None,
            session_id: None,
        };
        let out = ListRuneEnvTool.run(&ctx, Value::Null).await.unwrap();
        assert_eq!(out["hitl_functions"], json!(["delete_draft_invoice"]));
    }
}
