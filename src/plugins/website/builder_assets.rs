//! GrapesJS asset upload + public media stream.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path},
    http::{HeaderMap, HeaderValue, StatusCode, header},
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

pub async fn public_asset(
    Cap(state): Cap<WebsiteState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Response {
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
            let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
            file_bytes_response(buf, &download.content_type, Some(&download.filename), range)
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Inclusive byte range `start..=end` from a single `Range: bytes=` request.
pub(crate) fn parse_bytes_range(range: &str, len: u64) -> Option<(u64, u64)> {
    let rest = range.trim().strip_prefix("bytes=")?;
    if rest.contains(',') {
        return None;
    }
    let (start_s, end_s) = rest.split_once('-')?;
    if start_s.is_empty() {
        let n: u64 = end_s.parse().ok()?;
        if n == 0 || len == 0 {
            return None;
        }
        let start = len.saturating_sub(n);
        return Some((start, len - 1));
    }
    let start: u64 = start_s.parse().ok()?;
    if start >= len {
        return None;
    }
    let end = if end_s.is_empty() {
        len - 1
    } else {
        end_s.parse::<u64>().ok()?.min(len - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

pub(crate) fn file_bytes_response(
    buf: Vec<u8>,
    content_type: &str,
    filename: Option<&str>,
    range_header: Option<&str>,
) -> Response {
    let len = buf.len() as u64;
    let mut headers = Vec::new();
    if let Ok(v) = HeaderValue::from_str(content_type) {
        headers.push((header::CONTENT_TYPE, v));
    }
    if let Some(name) = filename
        && let Ok(v) =
            HeaderValue::from_str(&format!("inline; filename=\"{}\"", name.replace('"', "")))
    {
        headers.push((header::CONTENT_DISPOSITION, v));
    }
    headers.push((header::ACCEPT_RANGES, HeaderValue::from_static("bytes")));

    if let Some(range) = range_header {
        match parse_bytes_range(range, len) {
            Some((start, end)) => {
                let body = buf[start as usize..=end as usize].to_vec();
                let content_len = (end - start + 1).to_string();
                headers.push((
                    header::CONTENT_RANGE,
                    HeaderValue::from_str(&format!("bytes {start}-{end}/{len}"))
                        .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
                ));
                headers.push((
                    header::CONTENT_LENGTH,
                    HeaderValue::from_str(&content_len)
                        .unwrap_or_else(|_| HeaderValue::from_static("0")),
                ));
                let mut res = Response::new(Body::from(body));
                *res.status_mut() = StatusCode::PARTIAL_CONTENT;
                res.headers_mut().extend(headers);
                return res;
            }
            None if len > 0 => {
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
                res.headers_mut()
                    .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
                if let Ok(v) = HeaderValue::from_str(&format!("bytes */{len}")) {
                    res.headers_mut().insert(header::CONTENT_RANGE, v);
                }
                return res;
            }
            None => {}
        }
    }

    headers.push((
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string()).unwrap_or_else(|_| HeaderValue::from_static("0")),
    ));
    let mut res = Response::new(Body::from(buf));
    res.headers_mut().extend(headers);
    res
}

#[cfg(test)]
mod tests {
    use super::{file_bytes_response, parse_bytes_range};
    use axum::http::{StatusCode, header};

    #[test]
    fn parse_bytes_range_start_end() {
        assert_eq!(parse_bytes_range("bytes=0-3", 10), Some((0, 3)));
        assert_eq!(parse_bytes_range("bytes=2-", 10), Some((2, 9)));
        assert_eq!(parse_bytes_range("bytes=-4", 10), Some((6, 9)));
        assert_eq!(parse_bytes_range("bytes=0-999", 10), Some((0, 9)));
        assert_eq!(parse_bytes_range("bytes=10-12", 10), None);
        assert_eq!(parse_bytes_range("bytes=5-2", 10), None);
    }

    #[test]
    fn file_bytes_response_full_and_partial() {
        let full = file_bytes_response(
            b"abcdefghij".to_vec(),
            "video/webm",
            Some("hero.webm"),
            None,
        );
        assert_eq!(full.status(), StatusCode::OK);
        assert_eq!(full.headers().get(header::ACCEPT_RANGES).unwrap(), "bytes");
        assert_eq!(
            full.headers().get(header::CONTENT_TYPE).unwrap(),
            "video/webm"
        );

        let part = file_bytes_response(
            b"abcdefghij".to_vec(),
            "video/webm",
            Some("hero.webm"),
            Some("bytes=0-3"),
        );
        assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            part.headers().get(header::CONTENT_RANGE).unwrap(),
            "bytes 0-3/10"
        );
        assert_eq!(part.headers().get(header::CONTENT_LENGTH).unwrap(), "4");

        let unsat = file_bytes_response(
            b"abcdefghij".to_vec(),
            "video/webm",
            None,
            Some("bytes=99-100"),
        );
        assert_eq!(unsat.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    }
}
