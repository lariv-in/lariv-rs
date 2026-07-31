//! `POST /filesystem/chat-upload/` — multipart files → root VNodes → JSON `[{id,name}]`.

use axum::{
    Json,
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{
    html_form::multipart::collect_multipart,
    http::Cap,
    plugins::{
        filesystem::{node, state::FilesystemState},
        users::middleware::RequireAuth,
    },
};

#[derive(Debug, Serialize)]
struct NodeResult {
    id: i64,
    name: String,
}

/// Accept multipart field `Files` (multiple); create VNodes at filesystem root.
pub async fn chat_upload(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    multipart: Multipart,
) -> Response {
    let parts = match collect_multipart(multipart, &[], &["Files"]).await {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"invalid multipart form"})),
            )
                .into_response();
        }
    };

    let mut parts = parts;
    let files = parts.file_lists.remove("Files").unwrap_or_default();
    if files.is_empty() {
        return Json(Vec::<NodeResult>::new()).into_response();
    }

    let mut results = Vec::with_capacity(files.len());
    for file in files {
        let filename = file.filename().to_string();
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
            Err(e) => {
                tracing::error!(error = %e, "filesystem: chat-upload failed to create vnode");
            }
        }
    }

    Json(results).into_response()
}
