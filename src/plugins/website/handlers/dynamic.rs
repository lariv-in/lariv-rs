//! Public catch-all website pages + home fallback.

use std::sync::Arc;
use axum::{
    extract::{OriginalUri, Query},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::{
    grapesjs::GrapesJsCapability,
    http::Cap,
    plugins::{
        users::middleware::OptionalAuth,
        website::{
            match_route::find_matching_db_route,
            render::render_db_route,
            state::WebsiteState,
        },
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct EmptyQuery {
    #[serde(flatten)]
    pub rest: std::collections::HashMap<String, String>,
}

fn query_pairs(q: &EmptyQuery) -> Vec<(String, String)> {
    q.rest.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

/// `GET /` — render matching DB route or redirect login/dashboard.
pub async fn home(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    OptionalAuth(auth): OptionalAuth,
    uri: OriginalUri,
    Query(q): Query<EmptyQuery>,
) -> Response {
    let path = uri.0.path();
    match find_matching_db_route(&state.db, path).await {
        Ok(Some(route)) => {
            render_db_route(
                &state.db,
                state.store.as_ref(),
                &grapes,
                &route,
                path,
                query_pairs(&q),
            )
            .await
        }
        Ok(None) => {
            if auth.is_some() {
                Redirect::to("/dashboard").into_response()
            } else {
                Redirect::to("/users/login").into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "website: home route match failed");
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

/// `GET /{*path}` — public dynamic website pages.
pub async fn catch_all(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    uri: OriginalUri,
    Query(q): Query<EmptyQuery>,
) -> Response {
    let path = uri.0.path();
    match find_matching_db_route(&state.db, path).await {
        Ok(Some(route)) => {
            render_db_route(
                &state.db,
                state.store.as_ref(),
                &grapes,
                &route,
                path,
                query_pairs(&q),
            )
            .await
        }
        Ok(None) => (axum::http::StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "website: catch-all match failed");
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}
