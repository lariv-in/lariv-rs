//! `POST /filesystem/chat-upload/` — multipart files → root VNodes → JSON `[{id,name}]`.

use axum::{
    Json,
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    html_form::{HtmlForm, UploadedFile},
    http::Cap,
    plugins::{
        filesystem::{
            entities::VNode,
            forms::ChatUploadForm,
            node::{self, NodeError},
            state::FilesystemState,
            storage::DynFilestore,
            zip::read_file_bytes,
        },
        users::middleware::RequireAuth,
    },
};

#[derive(Debug, Serialize)]
struct NodeResult {
    id: i64,
    name: String,
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

/// Accept multipart field `Files` (multiple); create VNodes at filesystem root.
///
/// If a root file with the same name already exists and the SHA-256 of the upload
/// matches the stored file, return that vnode instead of failing with a conflict.
pub async fn chat_upload(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
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

    if parsed.files.is_empty() {
        return Json(Vec::<NodeResult>::new()).into_response();
    }

    let mut results = Vec::with_capacity(parsed.files.len());
    for file in parsed.files {
        let filename = file.filename().to_string();
        let name = node::sanitize_node_name(&filename);
        if name.is_empty() {
            tracing::error!("filesystem: chat-upload rejected empty filename");
            continue;
        }

        let upload_hash = match upload_content_hash(&file).await {
            Ok(h) => h,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    name = %name,
                    "filesystem: chat-upload failed hashing upload"
                );
                continue;
            }
        };

        match resolve_existing(&state, &name, upload_hash).await {
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
                    "filesystem: chat-upload name conflict with different content"
                );
                continue;
            }
            ExistingResolve::Failed => continue,
        }

        match node::create(
            &state.db,
            state.store.as_ref(),
            filename,
            false,
            Some(node::NodeFile::Upload(file)),
            None,
        )
        .await
        {
            Ok(vnode) => results.push(NodeResult {
                id: vnode.id,
                name: vnode.name,
            }),
            Err(NodeError::Conflict) => {
                // Race with another upload; reuse if content still matches.
                match resolve_existing(&state, &name, upload_hash).await {
                    ExistingResolve::Reuse(existing) => results.push(NodeResult {
                        id: existing.id,
                        name: existing.name,
                    }),
                    ExistingResolve::DifferentContent | ExistingResolve::Absent => {
                        tracing::error!(
                            name = %name,
                            "filesystem: chat-upload name conflict with different content"
                        );
                    }
                    ExistingResolve::Failed => {}
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "filesystem: chat-upload failed to create vnode");
            }
        }
    }

    Json(results).into_response()
}

enum ExistingResolve {
    Reuse(VNode),
    Absent,
    DifferentContent,
    Failed,
}

async fn resolve_existing(
    state: &FilesystemState,
    name: &str,
    upload_hash: [u8; 32],
) -> ExistingResolve {
    let existing = match node::find_child(&state.db, None, name, false).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                error = %e,
                name,
                "filesystem: chat-upload failed looking up existing vnode"
            );
            return ExistingResolve::Failed;
        }
    };
    let Some(existing) = existing else {
        return ExistingResolve::Absent;
    };
    match vnode_content_hash(state.store.as_ref(), &existing).await {
        Ok(existing_hash) if existing_hash == upload_hash => ExistingResolve::Reuse(existing),
        Ok(_) => ExistingResolve::DifferentContent,
        Err(e) => {
            tracing::error!(
                error = %e,
                name,
                "filesystem: chat-upload failed comparing existing file"
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
