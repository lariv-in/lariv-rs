//! `get_rune_env` — schema and kind of one Rune environment identifier.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::llm_assistant::genai::FunctionDeclaration,
    rune_env::standard_library_names,
};

pub struct GetRuneEnvTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    name: String,
}

#[async_trait]
impl LlmTool for GetRuneEnvTool {
    fn name(&self) -> &str {
        "get_rune_env"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "get_rune_env".into(),
            description:
                "Get the kind and schema of one identifier in the Rune assistant environment. \
                `name` may be a registered binding, a HITL function, or a Rune standard-library \
                module (as listed by list_rune_env). Call this whenever you do not already know \
                a Rune function's arguments or return type — do not guess the schema. Use before \
                calling the identifier from run_rune."
                    .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Identifier to describe (function, static value, HITL function, or std module)"
                    }
                },
                "required": ["name"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let name = parsed.name.trim();
        if name.is_empty() {
            return Err("name is required".into());
        }

        if let Some(info) = ctx.rune_env.lookup(name) {
            return Ok(json!({
                "name": info.name,
                "kind": info.kind.as_str(),
                "schema": info.schema,
            }));
        }

        if let Some(schema) = ctx.hitl.and_then(|h| h.lookup(name)) {
            return Ok(json!({
                "name": name,
                "kind": "hitl_function",
                "schema": schema,
            }));
        }

        if standard_library_names().iter().any(|n| *n == name) {
            return Ok(json!({
                "name": name,
                "kind": "standard_library",
                "schema": format!("Rune standard library module {name}"),
            }));
        }

        Err(format!("unknown rune identifier {name:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::{
        llm_tools::ToolCtx,
        plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore},
        rune_env::{NativeBinding, RuneEnvCapability},
    };

    fn ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: Arc<DynFilestore>,
        rune_env: &'a RuneEnvCapability,
        hitl: Option<&'a dyn crate::llm_tools::HitlSource>,
    ) -> ToolCtx<'a> {
        ToolCtx {
            db,
            store,
            cse_api_key: "",
            cse_cx: "",
            rune_env,
            hitl,
            hitl_gate: None,
            session_id: None,
        }
    }

    #[tokio::test]
    async fn describes_registered_function() {
        let mut cap = RuneEnvCapability::new();
        cap.register_contextual(
            "search_products",
            "search_products(#{ query: string }) -> #{ results: [...] }",
            |_ctx| NativeBinding::Function(Arc::new(|_ctx, _args| Err("unused".into()))),
        );
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let tool_ctx = ctx(&db, store, &cap, None);
        let out = GetRuneEnvTool
            .run(&tool_ctx, json!({ "name": "search_products" }))
            .await
            .unwrap();
        assert_eq!(out["name"], json!("search_products"));
        assert_eq!(out["kind"], json!("function"));
        assert!(
            out["schema"]
                .as_str()
                .unwrap_or_default()
                .contains("search_products(")
        );
    }

    #[tokio::test]
    async fn describes_hitl_function() {
        use crate::plugins::llm_assistant::hitl::HitlCapability;

        let cap = RuneEnvCapability::new();
        let mut hitl = HitlCapability::new();
        hitl.register(
            "delete_draft_invoice",
            "delete_draft_invoice(#{ id: int }) -> ()",
            |_ctx| NativeBinding::Function(Arc::new(|_ctx, _args| Ok(rune::Value::from(())))),
        );
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let tool_ctx = ctx(&db, store, &cap, Some(&hitl));
        let out = GetRuneEnvTool
            .run(&tool_ctx, json!({ "name": "delete_draft_invoice" }))
            .await
            .unwrap();
        assert_eq!(out["kind"], json!("hitl_function"));
        assert!(
            out["schema"]
                .as_str()
                .unwrap_or_default()
                .contains("delete_draft_invoice")
        );
    }

    #[tokio::test]
    async fn describes_standard_library_module() {
        let cap = RuneEnvCapability::new();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let tool_ctx = ctx(&db, store, &cap, None);
        let out = GetRuneEnvTool
            .run(&tool_ctx, json!({ "name": "std::vec" }))
            .await
            .unwrap();
        assert_eq!(out["kind"], json!("standard_library"));
    }

    #[tokio::test]
    async fn unknown_identifier_errors() {
        let cap = RuneEnvCapability::new();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let tool_ctx = ctx(&db, store, &cap, None);
        let err = GetRuneEnvTool
            .run(&tool_ctx, json!({ "name": "nope" }))
            .await
            .unwrap_err();
        assert!(err.contains("unknown rune identifier"));
    }
}
