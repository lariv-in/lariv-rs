//! Render DB-backed website pages via minijinja.

use std::{collections::HashMap, path::Path, sync::Arc};

use axum::{
    body::Body,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use minijinja::Environment;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tokio::io::AsyncReadExt;

use crate::{
    grapesjs::GrapesJsCapability,
    plugins::filesystem::{entities::VNode, node, storage::DynFilestore},
};

use super::{
    entities::{
        DbRoute,
        route_reference::{self, Entity as RouteRefEntity},
    },
    preferences,
    publish::fix_navbar_logos,
    template_funcs::register_funcs,
    theme::inject_theme_assets,
};

const TEMPLATE_EXTS: &[&str] = &[
    ".html", ".tmpl", ".htm", ".js", ".css", ".txt", ".md", ".json", ".yaml", ".yml",
];

fn is_template_ext(name: &str) -> bool {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_ascii_lowercase()))
        .unwrap_or_default();
    TEMPLATE_EXTS.contains(&ext.as_str())
}

pub(crate) async fn read_vnode_text(
    store: &DynFilestore,
    vnode: &VNode,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path = vnode.file_path.as_deref().unwrap_or("");
    let mut download = store.open(path, &vnode.name).await?;
    let mut buf = String::new();
    download.reader.read_to_string(&mut buf).await?;
    Ok(buf)
}

/// Render a matched route's page (template or binary stream).
pub async fn render_db_route(
    db: &DatabaseConnection,
    store: &DynFilestore,
    grapes: &GrapesJsCapability,
    route: &DbRoute,
    req_path: &str,
    query: Vec<(String, String)>,
) -> Response {
    let Some(page) =
        crate::web::opt_or_log(node::get_by_id(db, route.page_id).await, "get node by id")
    else {
        return (StatusCode::NOT_FOUND, "404 Not Found").into_response();
    };
    if page.is_directory {
        return (StatusCode::NOT_FOUND, "404 Not Found").into_response();
    }

    let vnode_path = node::get_path(db, &page).await;
    let rel = vnode_path.trim_start_matches('/');

    if is_template_ext(&page.name) {
        match render_template(db, store, grapes, route, &page, rel, req_path, query).await {
            Ok(html) => {
                let mut res = Response::new(Body::from(html));
                res.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/html; charset=utf-8"),
                );
                res
            }
            Err(e) => {
                tracing::error!(error = %e, "website: template render failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    } else {
        stream_file(store, &page).await
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "render needs route + request context"
)]
async fn render_template(
    db: &DatabaseConnection,
    store: &DynFilestore,
    grapes: &GrapesJsCapability,
    route: &DbRoute,
    page: &VNode,
    main_name: &str,
    req_path: &str,
    query: Vec<(String, String)>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut sources: HashMap<String, String> = HashMap::new();
    sources.insert(main_name.to_string(), read_vnode_text(store, page).await?);

    let refs = RouteRefEntity::find()
        .filter(route_reference::Column::DbRouteId.eq(route.id))
        .all(db)
        .await?;
    for r in refs {
        if let Some(vnode) = node::get_by_id(db, r.v_node_id).await? {
            let path = node::get_path(db, &vnode).await;
            let rel = path.trim_start_matches('/').to_string();
            if rel.is_empty() || rel == main_name {
                continue;
            }
            if let Ok(src) = read_vnode_text(store, &vnode).await {
                sources.insert(rel, src);
            }
        }
    }

    let sources = Arc::new(sources);
    let mut env = Environment::new();
    register_funcs(&mut env, db.clone(), req_path.to_string(), query);
    let loader_sources = sources.clone();
    env.set_loader(move |name| Ok(loader_sources.get(name).cloned()));

    let tmpl = env.get_template(main_name)?;
    let mut html = tmpl.render(())?;

    if !route.theme.trim().is_empty() {
        let theme = preferences::resolve_theme(grapes, db, store, &route.theme).await;
        html = inject_theme_assets(&html, &route.theme, theme.as_ref());
    }
    Ok(fix_navbar_logos(&html))
}

async fn stream_file(store: &DynFilestore, page: &VNode) -> Response {
    let path = page.file_path.as_deref().unwrap_or("");
    match store.open(path, &page.name).await {
        Ok(download) => {
            let mut buf = Vec::new();
            let mut reader = download.reader;
            if reader.read_to_end(&mut buf).await.is_err() {
                return (StatusCode::INTERNAL_SERVER_ERROR, "read error").into_response();
            }
            let mut res = Response::new(Body::from(buf));
            if let Ok(v) = HeaderValue::from_str(&download.content_type) {
                res.headers_mut().insert(header::CONTENT_TYPE, v);
            }
            res
        }
        Err(e) if e.is_missing() => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "website: stream file failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

/// Replace a VNode's blob content with `bytes`.
pub async fn replace_vnode_content(
    db: &DatabaseConnection,
    store: &DynFilestore,
    page: VNode,
    bytes: &[u8],
) -> Result<VNode, Box<dyn std::error::Error + Send + Sync>> {
    let ext = node::ext_of(&page.name);
    let new_path = store.save(bytes, &ext).await?;
    let old_path = page.file_path.clone();
    use sea_orm::{ActiveModelTrait, ActiveValue::Set};
    let mut am: crate::plugins::filesystem::entities::filesystem_node::ActiveModel = page.into();
    am.file_path = Set(Some(new_path.clone()));
    am.updated_at = Set(Some(chrono::Utc::now()));
    let updated = am.update(db).await?;
    if let Some(old) = old_path
        && old != new_path
        && let Err(e) = store.delete(&old).await
    {
        tracing::error!(path = old, error = %e, "website: failed deleting replaced blob");
    }
    Ok(updated)
}
