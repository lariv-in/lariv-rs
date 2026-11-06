use axum::{
    body::Body,
    extract::Path,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use tokio::io::AsyncReadExt;

use crate::{
    entities::VNode, node, permissions::AccessActor, routes::VNodeListRouteTag,
    state::FilesystemState, zip,
};
use lariv_core::http::Cap;
use lariv_plugin_users::middleware::RequireAuth;

use super::{forbidden, require_root_view};

pub(crate) fn zip_response(filename: &str, bytes: Vec<u8>) -> Response {
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, v);
    }
    response
}

async fn stream_file(state: &FilesystemState, n: &VNode) -> Response {
    let Some(path) = n.file_path.as_deref().filter(|p| !p.is_empty()) else {
        return (StatusCode::NOT_FOUND, "file missing").into_response();
    };
    match state.store.open(path, &n.name).await {
        Ok(mut download) => {
            let mut buf = Vec::new();
            if let Err(e) = download.reader.read_to_end(&mut buf).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
            let mut response = Response::new(Body::from(buf));
            if let Ok(v) = HeaderValue::from_str(&download.content_type) {
                response.headers_mut().insert(header::CONTENT_TYPE, v);
            }
            if let Ok(v) =
                HeaderValue::from_str(&format!("attachment; filename=\"{}\"", download.filename))
            {
                response
                    .headers_mut()
                    .insert(header::CONTENT_DISPOSITION, v);
            }
            response
        }
        Err(e) if e.is_missing() => (StatusCode::NOT_FOUND, "file missing").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// HTTP handler: `download`.
pub async fn download(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(n) =
        lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
    else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    let actor = AccessActor::from_auth(&ctx);
    let check = if n.is_directory {
        node::authorize_open(&n, &actor).and(node::authorize_view_tree(&state.db, &n, &actor).await)
    } else {
        node::authorize_view(&n, &actor)
    };
    if let Err(e) = check {
        return forbidden(e);
    }
    if n.is_directory {
        match zip::build_zip(&state.db, state.store.as_ref(), Some(&n)).await {
            Ok((filename, bytes)) => zip_response(&filename, bytes),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        stream_file(&state, &n).await
    }
}

/// HTTP handler: `download_root`.
pub async fn download_root(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
) -> Response {
    if let Err(response) = require_root_view(&state.db, &ctx).await {
        return response;
    }
    let roots = match node::list_children(&state.db, None, false, "").await {
        Ok(rows) => rows,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let actor = AccessActor::from_auth(&ctx);
    for n in &roots {
        if let Err(e) = node::authorize_view_tree(&state.db, n, &actor).await {
            return forbidden(e);
        }
    }
    match zip::build_zip(&state.db, state.store.as_ref(), None).await {
        Ok((filename, bytes)) => zip_response(&filename, bytes),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
