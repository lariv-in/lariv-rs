use std::sync::Arc;

use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use maud::Markup;
use sea_orm::DatabaseConnection;

use crate::{
    components::shell::{ShellBase, shell_base},
    grapesjs::GrapesJsCapability,
    plugins::{
        filesystem::storage::DynFilestore,
        website::{preferences::resolve_theme, theme::inject_theme_assets},
    },
};

pub const DEFAULT_PUBLIC_THEME: &str = "p_website.daisyui";

pub async fn render_themed_public_page(
    db: &DatabaseConnection,
    store: &DynFilestore,
    grapes: &GrapesJsCapability,
    title: &str,
    body: Markup,
) -> Response {
    let theme = resolve_theme(grapes, db, store, DEFAULT_PUBLIC_THEME).await;
    let html = shell_base(ShellBase {
        title,
        body,
        ..Default::default()
    })
    .into_string();
    let themed = inject_theme_assets(&html, DEFAULT_PUBLIC_THEME, theme.as_ref());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))
        .body(axum::body::Body::from(themed))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
            )
                .into_response()
        })
}

pub async fn render_themed_public_page_arc(
    db: &DatabaseConnection,
    store: &DynFilestore,
    grapes: &Arc<GrapesJsCapability>,
    title: &str,
    body: Markup,
) -> Response {
    render_themed_public_page(db, store, grapes.as_ref(), title, body).await
}
