//! `POST /llm-assistant/chat-upload/` — multipart files → conversation folder VNodes.

use axum::{
    Json,
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    html_form::{HtmlForm, UploadedFile},
    http::Cap,
    plugins::{
        filesystem::{
            entities::VNode,
            node::{self, NodeError, NodeFile},
            state::FilesystemState,
            storage::DynFilestore,
            zip::read_file_bytes,
        },
        llm_assistant::{
            chat_attachments,
            entities::session::{self, Entity as SessionEntity},
            forms::ChatUploadForm,
            state::LlmAssistantState,
        },
        users::middleware::RequireAuth,
    },
};

#[derive(Debug, Serialize)]
struct NodeResult {
    id: i64,
    name: String,
}

#[derive(Debug, Serialize)]
struct UploadResponse {
    session_id: i64,
    files: Vec<NodeResult>,
}

fn content_hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

async fn upload_content_hash(file: &UploadedFile) -> Result<[u8; 32], NodeError> {
    let data = tokio::fs::read(file.path())
        .await
        .map_err(|e| NodeError::Validation(e.to_string()))?;
    Ok(content_hash(&data))
}

async fn vnode_content_hash(store: &DynFilestore, node: &VNode) -> Result<[u8; 32], NodeError> {
    let data = read_file_bytes(store, node).await?;
    Ok(content_hash(&data))
}

fn can_access_session(session: &session::Model, user_id: i64, is_superuser: bool) -> bool {
    is_superuser || session.user_id == user_id
}

async fn resolve_or_create_session(
    state: &LlmAssistantState,
    user_id: i64,
    is_superuser: bool,
    session_id: i64,
) -> Result<i64, String> {
    if session_id == 0 {
        let now = Utc::now();
        let model = session::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            title: Set(String::new()),
            user_id: Set(user_id),
            reply_email: Set(None),
            email_message_id: Set(None),
            email_references: Set(None),
        };
        let saved = model.insert(&state.db).await.map_err(|e| e.to_string())?;
        return Ok(saved.id);
    }

    let sess = SessionEntity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "session not found".to_string())?;
    if !can_access_session(&sess, user_id, is_superuser) {
        return Err("session belongs to another user".into());
    }
    Ok(sess.id)
}

/// Accept multipart `Files` (+ optional `session_id`); store under the conversation folder.
///
/// Creates the session when `session_id` is 0 so uploads for a new chat have a stable folder.
/// Duplicate name+hash under that folder reuses the existing vnode.
pub async fn chat_upload(
    Cap(state): Cap<LlmAssistantState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    multipart: Multipart,
) -> Response {
    let parsed = match ChatUploadForm::from_multipart(multipart).await {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"invalid multipart form"})),
            )
                .into_response();
        }
    };

    let session_id_raw = parsed.session_id.unwrap_or(0).max(0);

    let session_id =
        match resolve_or_create_session(&state, ctx.user.id, ctx.user.is_superuser, session_id_raw)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e})),
                )
                    .into_response();
            }
        };

    let folder =
        match chat_attachments::ensure_conversation_folder(&fs.db, fs.store.as_ref(), session_id)
            .await
        {
            Ok(dir) => dir,
            Err(e) => {
                tracing::error!(error = %e, "llm_assistant: chat-upload failed ensuring folder");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        };

    if parsed.files.is_empty() {
        return Json(UploadResponse {
            session_id,
            files: Vec::new(),
        })
        .into_response();
    }

    let mut results = Vec::with_capacity(parsed.files.len());
    for file in parsed.files {
        let filename = file.filename().to_string();
        let name = node::sanitize_node_name(&filename);
        if name.is_empty() {
            tracing::error!("llm_assistant: chat-upload rejected empty filename");
            continue;
        }

        let upload_hash = match upload_content_hash(&file).await {
            Ok(h) => h,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    name = %name,
                    "llm_assistant: chat-upload failed hashing upload"
                );
                continue;
            }
        };

        match resolve_existing(&fs, folder.id, &name, upload_hash).await {
            ExistingResolve::Reuse(existing) => {
                results.push(NodeResult {
                    id: existing.id,
                    name: existing.name,
                });
                continue;
            }
            ExistingResolve::Absent => {}
            ExistingResolve::DifferentContent => {
                tracing::error!(
                    name = %name,
                    "llm_assistant: chat-upload name conflict with different content"
                );
                continue;
            }
            ExistingResolve::Failed => continue,
        }

        match node::create(
            &fs.db,
            fs.store.as_ref(),
            filename,
            false,
            Some(NodeFile::Upload(file)),
            Some(&folder),
        )
        .await
        {
            Ok(vnode) => results.push(NodeResult {
                id: vnode.id,
                name: vnode.name,
            }),
            Err(NodeError::Conflict) => {
                match resolve_existing(&fs, folder.id, &name, upload_hash).await {
                    ExistingResolve::Reuse(existing) => results.push(NodeResult {
                        id: existing.id,
                        name: existing.name,
                    }),
                    ExistingResolve::DifferentContent | ExistingResolve::Absent => {
                        tracing::error!(
                            name = %name,
                            "llm_assistant: chat-upload name conflict with different content"
                        );
                    }
                    ExistingResolve::Failed => {}
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "llm_assistant: chat-upload failed to create vnode");
            }
        }
    }

    Json(UploadResponse {
        session_id,
        files: results,
    })
    .into_response()
}

enum ExistingResolve {
    Reuse(VNode),
    Absent,
    DifferentContent,
    Failed,
}

async fn resolve_existing(
    fs: &FilesystemState,
    parent_id: i64,
    name: &str,
    upload_hash: [u8; 32],
) -> ExistingResolve {
    let existing = match node::find_child(&fs.db, Some(parent_id), name, false).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                error = %e,
                name,
                "llm_assistant: chat-upload failed looking up existing vnode"
            );
            return ExistingResolve::Failed;
        }
    };
    let Some(existing) = existing else {
        return ExistingResolve::Absent;
    };
    match vnode_content_hash(fs.store.as_ref(), &existing).await {
        Ok(existing_hash) if existing_hash == upload_hash => ExistingResolve::Reuse(existing),
        Ok(_) => ExistingResolve::DifferentContent,
        Err(e) => {
            tracing::error!(
                error = %e,
                name,
                "llm_assistant: chat-upload failed comparing existing file"
            );
            ExistingResolve::Failed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::content_hash;

    #[test]
    fn content_hash_stable_for_same_bytes() {
        let a = content_hash(b"hello");
        let b = content_hash(b"hello");
        let c = content_hash(b"world");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
