//! `generate_pdf` — compile Typst markup to a PDF and save it as a VNode.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::{
            entities::VNode,
            node::{self, NodeFile},
        },
        llm_assistant::{chat_attachments, genai::FunctionDeclaration},
    },
};

/// Official Typst docs the model should fetch with `read_webpage` before writing markup.
const TYPST_DOCS_HOME: &str = "https://typst.app/docs/";
const TYPST_SYNTAX_DOCS: &str = "https://typst.app/docs/reference/syntax/";
const TYPST_REFERENCE_DOCS: &str = "https://typst.app/docs/reference/";

const DEFAULT_FILENAME: &str = "document.pdf";
/// Public download URL origin + path pattern (`{id}` is the VNode id).
const DOWNLOAD_URL_EXAMPLE: &str = "http://localhost:42069/filesystem/3/download";

pub struct GeneratePdfTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    source: String,
    #[serde(default)]
    filename: String,
}

#[async_trait]
impl LlmTool for GeneratePdfTool {
    fn name(&self) -> &str {
        "generate_pdf"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "generate_pdf".into(),
            description: format!(
                "Compile Typst markup to a PDF and save it as a virtual file (VNode) in this \
                conversation's attachments folder. Returns JSON with `vnode_id`, `name`, `path`, \
                `bytes`, and `download_url`. Download links use `/filesystem/{{id}}/download` \
                (example: {DOWNLOAD_URL_EXAMPLE}). In your reply, give the user that download URL \
                so they can open the PDF. To read or reason about the PDF in this conversation, \
                call `attach_vnode_to_context` with the returned `vnode_id` or `path`. Before \
                writing Typst source, use the `read_webpage` tool \
                to check Typst grammar and language rules on the official docs: {TYPST_SYNTAX_DOCS} \
                (syntax), {TYPST_REFERENCE_DOCS} (language reference), and {TYPST_DOCS_HOME} \
                (guides). Do not guess unfamiliar Typst syntax, functions, or typesetting rules — \
                fetch the relevant doc page first. If compile fails, read the diagnostics, consult \
                the docs again, then retry."
            ),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Typst markup to compile (must be valid Typst, not Markdown)"
                    },
                    "filename": {
                        "type": "string",
                        "description": format!(
                            "Output PDF filename (default {DEFAULT_FILENAME}); `.pdf` is appended if missing"
                        )
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

        let pdf_bytes = crate::typst::typst_compile(source).await?;
        let parent = save_parent(ctx).await?;
        let filename = unique_filename(ctx.db, parent.id, &pdf_filename(&parsed.filename)).await;
        let vnode = node::create(
            ctx.db,
            ctx.store.as_ref(),
            filename.clone(),
            false,
            Some(NodeFile::Bytes {
                filename: filename.clone(),
                data: pdf_bytes.clone(),
            }),
            Some(&parent),
        )
        .await
        .map_err(|e| format!("save PDF: {e}"))?;
        let path = node::get_path(ctx.db, &vnode).await;
        let download_url = download_url(vnode.id);

        Ok(json!({
            "vnode_id": vnode.id,
            "name": vnode.name,
            "path": path,
            "bytes": pdf_bytes.len(),
            "download_url": download_url,
        }))
    }
}

fn download_url(vnode_id: i64) -> String {
    format!("http://localhost:42069/filesystem/{vnode_id}/download")
}

fn pdf_filename(raw: &str) -> String {
    let mut name = node::sanitize_node_name(raw);
    if name.is_empty() {
        name = DEFAULT_FILENAME.to_string();
    }
    if !name.to_ascii_lowercase().ends_with(".pdf") {
        name.push_str(".pdf");
    }
    name
}

async fn save_parent(ctx: &ToolCtx<'_>) -> Result<VNode, String> {
    match ctx.session_id {
        Some(session_id) => {
            chat_attachments::ensure_conversation_folder(ctx.db, ctx.store.as_ref(), session_id)
                .await
                .map_err(|e| format!("chat attachments folder: {e}"))
        }
        None => chat_attachments::ensure_chat_attachments_parent(ctx.db, ctx.store.as_ref())
            .await
            .map_err(|e| format!("chat attachments folder: {e}")),
    }
}

async fn unique_filename(db: &sea_orm::DatabaseConnection, parent_id: i64, base: &str) -> String {
    if node::find_child(db, Some(parent_id), base, false)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return base.to_string();
    }

    let path = std::path::Path::new(base);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(base);
    let ext = path.extension().and_then(|s| s.to_str());
    let ext_suffix = ext.map(|e| format!(".{e}")).unwrap_or_default();

    for n in 2..1000 {
        let candidate = format!("{stem}-{n}{ext_suffix}");
        if node::find_child(db, Some(parent_id), &candidate, false)
            .await
            .ok()
            .flatten()
            .is_none()
        {
            return candidate;
        }
    }
    format!("{stem}-{}", chrono::Utc::now().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_tells_model_to_read_typst_docs() {
        let decl = GeneratePdfTool.declaration();
        assert_eq!(decl.name, "generate_pdf");
        assert!(decl.description.contains("read_webpage"));
        assert!(decl.description.contains(TYPST_SYNTAX_DOCS));
        assert!(decl.description.contains(TYPST_REFERENCE_DOCS));
        assert!(decl.description.contains(TYPST_DOCS_HOME));
        assert!(decl.description.contains(DOWNLOAD_URL_EXAMPLE));
        assert!(decl.description.contains("download_url"));
        assert!(decl.description.contains("attach_vnode_to_context"));
    }

    #[test]
    fn download_url_matches_filesystem_route() {
        assert_eq!(download_url(3), DOWNLOAD_URL_EXAMPLE);
        assert_eq!(
            download_url(12),
            "http://localhost:42069/filesystem/12/download"
        );
    }

    #[test]
    fn pdf_filename_defaults_and_appends_extension() {
        assert_eq!(pdf_filename(""), DEFAULT_FILENAME);
        assert_eq!(pdf_filename("  "), DEFAULT_FILENAME);
        assert_eq!(pdf_filename("report"), "report.pdf");
        assert_eq!(pdf_filename("report.pdf"), "report.pdf");
        assert_eq!(pdf_filename("report.PDF"), "report.PDF");
        assert_eq!(pdf_filename("../secret.pdf"), "secret.pdf");
    }

    #[tokio::test]
    async fn empty_source_errors() {
        use std::sync::Arc;

        use crate::{
            llm_tools::ToolCtx,
            plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore},
            rune_env::RuneEnvCapability,
        };

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
            genai: None,
            subagents: None,
        };
        let err = GeneratePdfTool
            .run(&ctx, json!({ "source": "  " }))
            .await
            .unwrap_err();
        assert!(err.contains("source is required"));
    }
}
