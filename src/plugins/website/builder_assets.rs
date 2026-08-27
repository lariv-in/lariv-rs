//! GrapesJS asset upload + public media stream.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    extract::{Multipart, Path},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;
use tokio::io::AsyncReadExt;

use crate::{
    http::Cap,
    plugins::{filesystem::node, users::middleware::RequireAuth, website::state::WebsiteState},
};

pub fn public_asset_url(id: i64) -> String {
    format!("/media/{id}/")
}

pub async fn builder_asset_upload(
    Cap(state): Cap<WebsiteState>,
    RequireAuth(_ctx): RequireAuth,
    mut multipart: Multipart,
) -> Response {
    let mut urls = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_ascii_lowercase();
        if !(name == "files" || name == "files[]" || name.starts_with("files")) {
            continue;
        }
        let filename = field.file_name().unwrap_or("upload.bin").to_string();
        let Ok(bytes) = field.bytes().await else {
            return (
                StatusCode::BAD_REQUEST,
                r#"{"error":"invalid multipart form"}"#,
            )
                .into_response();
        };
        let ext = node::ext_of(&filename);
        let base = filename.strip_suffix(&ext).unwrap_or(&filename).to_string();
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let unique = format!("{base}_{millis}{ext}");
        let segments = state.config.assets_dir_segments();
        let parent_id =
            match node::ensure_directory_path(&state.db, state.store.as_ref(), None, &segments)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!(error = %e, "builder asset: ensure dir");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        r#"{"error":"failed to store asset"}"#,
                    )
                        .into_response();
                }
            };
        let parent = match parent_id {
            Some(id) => {
                crate::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
            }
            None => None,
        };
        match node::create(
            &state.db,
            state.store.as_ref(),
            unique,
            false,
            Some(node::NodeFile::Bytes {
                filename,
                data: bytes.to_vec(),
            }),
            parent.as_ref(),
        )
        .await
        {
            Ok(n) => urls.push(public_asset_url(n.id)),
            Err(e) => {
                tracing::error!(error = %e, "builder asset: create vnode");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    r#"{"error":"failed to store asset"}"#,
                )
                    .into_response();
            }
        }
    }
    Json(json!({ "data": urls })).into_response()
}

pub async fn public_asset(Cap(state): Cap<WebsiteState>, Path(id): Path<i64>) -> Response {
    let Some(n) = crate::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if n.is_directory {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = n.file_path.as_deref().unwrap_or("");
    match state.store.open(path, &n.name).await {
        Ok(download) => {
            let mut buf = Vec::new();
            let mut reader = download.reader;
            if reader.read_to_end(&mut buf).await.is_err() {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            let mut res = Response::new(axum::body::Body::from(buf));
            if let Ok(v) = HeaderValue::from_str(&download.content_type) {
                res.headers_mut().insert(header::CONTENT_TYPE, v);
            }
            if let Ok(v) = HeaderValue::from_str(&format!(
                "inline; filename=\"{}\"",
                download.filename.replace('"', "")
            )) {
                res.headers_mut().insert(header::CONTENT_DISPOSITION, v);
            }
            res
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
