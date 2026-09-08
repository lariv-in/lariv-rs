//! `attach_vnode_to_context` — put a VNode into the current conversation via Gemini Files API.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx, ToolResult},
    plugins::{
        filesystem::{entities::VNode, node, zip::read_file_bytes},
        llm_assistant::{
            compaction::{contents_for_api, latest_fence, load_session_fences},
            content::{attachments::detect_mime, load_session_turns},
            gemini_file_cache,
            genai::{FunctionDeclaration, FunctionResponseFileData, FunctionResponsePart},
        },
    },
};

pub struct AttachVnodeToContextTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    path: String,
    #[serde(default)]
    vnode_id: i64,
}

#[async_trait]
impl LlmTool for AttachVnodeToContextTool {
    fn name(&self) -> &str {
        "attach_vnode_to_context"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "attach_vnode_to_context".into(),
            description: "Attach a virtual file (VNode) to this conversation so you can read \
                and reason about its contents (PDF, image, text, and other supported types). \
                Pass `path` (absolute VNode path) or `vnode_id`. The file is uploaded to Gemini \
                once and reused while the Files API cache is still valid — do not re-attach \
                the same file unless the user asks or the file may have changed. The tool result \
                includes the document as a `file_data` part in the conversation, not only a JSON \
                acknowledgement. If this tool reports `already_in_context`, the file was already \
                referenced in the current conversation window."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute VNode path (e.g. /chat_attachments/12/report.pdf)"
                    },
                    "vnode_id": {
                        "type": "integer",
                        "description": "VNode id (alternative to path)"
                    }
                }
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        Ok(self.run_with_parts(ctx, args).await?.response)
    }

    async fn run_with_parts(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<ToolResult, String> {
        attach(ctx, args).await
    }
}

async fn attach(ctx: &ToolCtx<'_>, args: Value) -> Result<ToolResult, String> {
    let parsed: Args = serde_json::from_value(args).unwrap_or_default();
    let vnode = resolve_vnode(ctx, &parsed).await?;
    if vnode.is_directory {
        return Err(format!(
            "path \"{}\" is a directory, not a file",
            vnode.name
        ));
    }

    let bytes = read_file_bytes(ctx.store.as_ref(), &vnode)
        .await
        .map_err(|e| e.to_string())?;
    let mime = detect_mime(&vnode.name, &bytes);
    let path = node::get_path(ctx.db, &vnode).await;

    let already_in_context = match ctx.session_id {
        Some(session_id) => vnode_in_api_window(ctx.db, session_id, vnode.id).await?,
        None => false,
    };

    let genai = ctx
        .genai
        .ok_or_else(|| "Gemini client is not available for this turn".to_string())?;

    let (cached, from_cache) =
        gemini_file_cache::resolve_or_upload(ctx.db, genai, vnode.id, &vnode.name, &mime, &bytes)
            .await?;

    // Always include `file_data` parts. `already_in_context` is only a hint: chat
    // uploads live as `inline_data` and are elided on later tool rounds, so skipping
    // parts here would leave the PDF out of the model's window.
    Ok(ToolResult {
        response: json!({
            "attached": true,
            "vnode_id": vnode.id,
            "name": vnode.name,
            "path": path,
            "mime_type": cached.mime_type,
            "bytes": bytes.len(),
            "cached": from_cache,
            "already_in_context": already_in_context,
        }),
        parts: vec![FunctionResponsePart {
            file_data: Some(FunctionResponseFileData {
                file_uri: cached.file_uri,
                mime_type: cached.mime_type,
            }),
            display_name: vnode.name.clone(),
            vnode_id: Some(vnode.id),
            ..Default::default()
        }],
    })
}

async fn resolve_vnode(ctx: &ToolCtx<'_>, parsed: &Args) -> Result<VNode, String> {
    if parsed.vnode_id > 0 {
        return node::get_by_id(ctx.db, parsed.vnode_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("vnode {} not found", parsed.vnode_id));
    }
    let path = parsed.path.trim();
    if path.is_empty() {
        return Err("path or vnode_id is required".into());
    }
    let (node, _) = node::get_by_path(ctx.db, path)
        .await
        .map_err(|e| e.to_string())?;
    node.ok_or_else(|| format!("file not found at path \"{path}\""))
}

/// True when this VNode is already present as an attachment in the uncompacted API window.
async fn vnode_in_api_window(
    db: &sea_orm::DatabaseConnection,
    session_id: i64,
    vnode_id: i64,
) -> Result<bool, String> {
    let turns = load_session_turns(db, session_id)
        .await
        .map_err(|e| e.to_string())?;
    let fences = load_session_fences(db, session_id)
        .await
        .map_err(|e| e.to_string())?;
    let contents = contents_for_api(&turns, latest_fence(&fences));
    Ok(contents.iter().any(|c| {
        c.parts.iter().any(|p| {
            if p.vnode_id == Some(vnode_id) {
                return true;
            }
            p.function_response
                .as_ref()
                .is_some_and(|fr| fr.parts.iter().any(|fp| fp.vnode_id == Some(vnode_id)))
        })
    }))
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
            session_id: None,
            genai: None,
        }
    }

    #[test]
    fn declaration_names_tool_and_cache_behavior() {
        let decl = AttachVnodeToContextTool.declaration();
        assert_eq!(decl.name, "attach_vnode_to_context");
        assert!(decl.description.contains("vnode_id"));
        assert!(decl.description.contains("cache"));
        assert!(decl.description.contains("already_in_context"));
    }

    #[tokio::test]
    async fn missing_path_and_id_errors() {
        let cap = RuneEnvCapability::new();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let err = AttachVnodeToContextTool
            .run(&ctx(&db, store, &cap), json!({}))
            .await
            .unwrap_err();
        assert!(err.contains("path or vnode_id is required"));
    }
}
