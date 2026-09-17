//! Spawn, status, and wait tools for background subagent sessions.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, SubagentHost, ToolCtx},
    plugins::llm_assistant::genai::FunctionDeclaration,
};

pub struct SpawnSubagentTool;
pub struct GetSubagentStatusTool;
pub struct GetSubagentResultTool;
pub struct WaitForSubagentsTool;

fn host<'a>(ctx: &ToolCtx<'a>) -> Result<&'a dyn SubagentHost, String> {
    ctx.subagents
        .ok_or_else(|| "subagent tools are only available during an assistant turn".to_string())
}

fn caller_session_id(ctx: &ToolCtx<'_>) -> Result<i64, String> {
    ctx.session_id
        .ok_or_else(|| "subagent tools require an active chat session".to_string())
}

#[derive(Debug, Deserialize)]
struct SpawnArgs {
    prompt: String,
    #[serde(default)]
    title: Option<String>,
}

#[async_trait]
impl LlmTool for SpawnSubagentTool {
    fn name(&self) -> &str {
        "spawn_subagent"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "spawn_subagent".into(),
            description: "Start a background child assistant with its own conversation. \
                 Returns immediately with a session_id. Use get_subagent_status to poll, \
                 or wait_for_subagents to block until one or more children finish. \
                 Give a complete, self-contained prompt — the child does not see this chat."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "Full instructions for the child assistant."
                    },
                    "title": {
                        "type": "string",
                        "description": "Optional short title for the child session."
                    }
                },
                "required": ["prompt"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: SpawnArgs =
            serde_json::from_value(args).map_err(|e| format!("invalid arguments: {e}"))?;
        host(ctx)?
            .spawn_subagent(caller_session_id(ctx)?, parsed.prompt, parsed.title)
            .await
    }
}

#[derive(Debug, Deserialize)]
struct StatusArgs {
    session_id: i64,
}

#[async_trait]
impl LlmTool for GetSubagentStatusTool {
    fn name(&self) -> &str {
        "get_subagent_status"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "get_subagent_status".into(),
            description: "Check whether a child session from spawn_subagent is still running, \
                 and read its title and latest assistant text."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "integer",
                        "description": "Child session id returned by spawn_subagent."
                    }
                },
                "required": ["session_id"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: StatusArgs =
            serde_json::from_value(args).map_err(|e| format!("invalid arguments: {e}"))?;
        host(ctx)?
            .subagent_status(caller_session_id(ctx)?, parsed.session_id)
            .await
    }
}

#[derive(Debug, Deserialize)]
struct ResultArgs {
    session_id: i64,
}

#[async_trait]
impl LlmTool for GetSubagentResultTool {
    fn name(&self) -> &str {
        "get_subagent_result"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "get_subagent_result".into(),
            description:
                "Get the result and status of a child assistant session from spawn_subagent.".into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "integer",
                        "description": "Child session id returned by spawn_subagent."
                    }
                },
                "required": ["session_id"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: ResultArgs =
            serde_json::from_value(args).map_err(|e| format!("invalid arguments: {e}"))?;
        host(ctx)?
            .subagent_result(caller_session_id(ctx)?, parsed.session_id)
            .await
    }
}

#[derive(Debug, Deserialize)]
struct WaitArgs {
    session_ids: Vec<i64>,
}

#[async_trait]
impl LlmTool for WaitForSubagentsTool {
    fn name(&self) -> &str {
        "wait_for_subagents"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "wait_for_subagents".into(),
            description: "Block until every listed child session from spawn_subagent has finished \
                 its current turn. Returns each child's status, title, and latest assistant text."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "session_ids": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Child session ids returned by spawn_subagent."
                    }
                },
                "required": ["session_ids"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: WaitArgs =
            serde_json::from_value(args).map_err(|e| format!("invalid arguments: {e}"))?;
        host(ctx)?
            .wait_subagents(caller_session_id(ctx)?, parsed.session_ids)
            .await
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

    fn ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: Arc<DynFilestore>,
        rune_env: &'a RuneEnvCapability,
    ) -> ToolCtx<'a> {
        ToolCtx {
            db,
            store,
            cse_api_key: "",
            cse_cx: "",
            rune_env,
            hitl: None,
            hitl_gate: None,
            session_id: Some(1),
            genai: None,
            subagents: None,
        }
    }

    #[test]
    fn declarations_name_the_tools() {
        assert_eq!(SpawnSubagentTool.declaration().name, "spawn_subagent");
        assert_eq!(
            GetSubagentStatusTool.declaration().name,
            "get_subagent_status"
        );
        assert_eq!(
            GetSubagentResultTool.declaration().name,
            "get_subagent_result"
        );
        assert_eq!(
            WaitForSubagentsTool.declaration().name,
            "wait_for_subagents"
        );
    }

    #[tokio::test]
    async fn missing_host_errors() {
        let cap = RuneEnvCapability::new();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = ctx(&db, store, &cap);
        let err = SpawnSubagentTool
            .run(&ctx, json!({ "prompt": "do work" }))
            .await
            .unwrap_err();
        assert!(err.contains("assistant turn"), "{err}");
    }
}
