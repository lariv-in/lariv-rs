//! HTTP handlers for PWA manifest, service worker, offline page, static assets, and asset links.

use std::path::{Path, PathBuf};

use axum::{
    extract::{Path as AxumPath, Request},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::http::Cap;
use crate::views::ViewRegistry;

use super::config::PwaConfig;

const DEFAULT_SERVICE_WORKER: &str = r#"/* lariv p_pwa default service worker */
const CACHE_NAME = "lariv-pwa-v1";
const OFFLINE_URL = "/offline";

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll([OFFLINE_URL]))
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;

  event.respondWith(
    fetch(req).catch(async () => {
      if (req.mode === "navigate") {
        const cache = await caches.open(CACHE_NAME);
        const cached = await cache.match(OFFLINE_URL);
        if (cached) return cached;
      }
      throw new Error("Network error");
    })
  );
});
"#;

const DEFAULT_OFFLINE_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1" />
    <title>Offline</title>
  </head>
  <body>
    <h1>You're offline</h1>
    <p>Please check your connection and try again.</p>
  </body>
</html>"#;

pub async fn manifest(Cap(cfg): Cap<PwaConfig>) -> Response {
    let body = json!({
        "name": cfg.app_name,
        "description": cfg.app_description,
        "theme_color": cfg.app_theme_color,
        "background_color": cfg.app_background_color,
        "display": cfg.app_display,
        "scope": cfg.app_scope,
        "orientation": cfg.app_orientation,
        "start_url": cfg.app_start_url,
        "dir": cfg.app_dir,
        "lang": cfg.app_lang,
        "icons": cfg.app_icons,
        "shortcuts": cfg.app_shortcuts,
        "screenshots": cfg.app_screenshots,
        "status_bar_color": cfg.app_status_bar_color,
        "icons_apple": cfg.app_icons_apple,
        "splash_screen": cfg.app_splash_screen,
    });
    (
        [(
            header::CONTENT_TYPE,
            "application/manifest+json; charset=utf-8",
        )],
        body.to_string(),
    )
        .into_response()
}

pub async fn service_worker(Cap(cfg): Cap<PwaConfig>) -> Response {
    if !cfg.service_worker_path.is_empty() {
        let path = Path::new(&cfg.service_worker_path);
        if path.is_file() {
            match tokio::fs::read(path).await {
                Ok(bytes) => {
                    return (
                        [(
                            header::CONTENT_TYPE,
                            "application/javascript; charset=utf-8",
                        )],
                        bytes,
                    )
                        .into_response();
                }
                Err(err) => {
                    tracing::error!(
                        error = %err,
                        path = %cfg.service_worker_path,
                        "p_pwa: failed reading serviceWorkerPath"
                    );
                    return StatusCode::NOT_FOUND.into_response();
                }
            }
        }
        tracing::warn!(
            path = %cfg.service_worker_path,
            "p_pwa: serviceWorkerPath not found"
        );
        return StatusCode::NOT_FOUND.into_response();
    }

    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        DEFAULT_SERVICE_WORKER,
    )
        .into_response()
}

pub async fn offline(
    Cap(cfg): Cap<PwaConfig>,
    Cap(views): Cap<ViewRegistry>,
    req: Request,
) -> Response {
    if !cfg.offline_view_name.is_empty() {
        if views.contains(&cfg.offline_view_name) {
            return views.dispatch(&cfg.offline_view_name, req).await;
        }
        tracing::error!(
            view = %cfg.offline_view_name,
            "p_pwa: offlineViewName not found in view registry"
        );
        return StatusCode::NOT_FOUND.into_response();
    }

    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        DEFAULT_OFFLINE_HTML,
    )
        .into_response()
}

pub async fn asset_links(Cap(cfg): Cap<PwaConfig>) -> Response {
    let body = json!([{
        "relation": [
            "delegate_permission/common.handle_all_urls",
            "delegate_permission/common.get_login_creds",
        ],
        "target": {
            "namespace": "android_app",
            "package_name": cfg.app_package_name,
            "sha256_cert_fingerprints": [cfg.app_sha256_cert_fingerprints],
        },
    }]);
    (
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

pub async fn static_pwa_root(Cap(cfg): Cap<PwaConfig>, req: Request) -> Response {
    serve_static(&cfg, req, "").await
}

pub async fn static_pwa_file(
    Cap(cfg): Cap<PwaConfig>,
    AxumPath(path): AxumPath<String>,
    req: Request,
) -> Response {
    serve_static(&cfg, req, &path).await
}

async fn serve_static(cfg: &PwaConfig, req: Request, relative: &str) -> Response {
    let Some(dir) = resolve_static_dir(cfg) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let uri_path = if relative.is_empty() {
        "/".to_string()
    } else {
        format!("/{relative}")
    };

    let (parts, body) = req.into_parts();
    let mut new_parts = parts;
    match uri_path.parse() {
        Ok(uri) => {
            new_parts.uri = uri;
        }
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    }
    let req = Request::from_parts(new_parts, body);

    match ServeDir::new(dir).oneshot(req).await {
        Ok(res) => res.into_response(),
        Err(err) => {
            tracing::error!(error = %err, "p_pwa: static file serve failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn resolve_static_dir(cfg: &PwaConfig) -> Option<PathBuf> {
    if cfg.static_dir.is_empty() {
        tracing::warn!("p_pwa: staticDir not configured; returning 404");
        return None;
    }

    let mut dir = PathBuf::from(&cfg.static_dir);
    if !dir.is_absolute() {
        match std::env::current_exe() {
            Ok(exe) => {
                if let Some(parent) = exe.parent() {
                    dir = parent.join(dir);
                }
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    static_dir = %cfg.static_dir,
                    "p_pwa: failed resolving executable path for staticDir"
                );
                return None;
            }
        }
    }

    match std::fs::metadata(&dir) {
        Ok(meta) if meta.is_dir() => Some(dir),
        Ok(_) => {
            tracing::error!(resolved_dir = %dir.display(), "p_pwa: staticDir is not a directory");
            None
        }
        Err(err) => {
            tracing::error!(
                error = %err,
                resolved_dir = %dir.display(),
                "p_pwa: staticDir does not exist or is not accessible"
            );
            None
        }
    }
}
