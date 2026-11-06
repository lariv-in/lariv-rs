use axum::{
    extract::Path,
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    forms::MoveForm,
    node,
    permissions::AccessActor,
    routes::{VNodeDetailRouteTag, VNodeListRouteTag},
    state::FilesystemState,
    templates::VNodeMoveFormPage,
};
use lariv_core::components::SharedChromeFolder;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_users::middleware::RequireAuth;

use super::slot_ctx;

pub async fn move_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(n) =
        lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
    else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    let access_error = node::authorize_remove(&n, &AccessActor::from_auth(&ctx))
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    let page = VNodeMoveFormPage {
        id: n.id,
        name: n.name,
        is_directory: n.is_directory,
        destination_id: 0,
        destination_display: String::new(),
        error: access_error,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
}

/// HTTP handler: `move_post`.
pub async fn move_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<MoveForm>,
) -> Response {
    let Some(n) =
        lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
    else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    let destination = if form.destination_id == 0 {
        None
    } else {
        lariv_core::web::opt_or_log(
            node::get_by_id(&state.db, form.destination_id).await,
            "get node by id",
        )
    };
    let actor = AccessActor::from_auth(&ctx);
    if let Err(e) = node::authorize_remove(&n, &actor) {
        let page = VNodeMoveFormPage {
            id,
            name: n.name,
            is_directory: n.is_directory,
            destination_id: form.destination_id,
            destination_display: destination
                .as_ref()
                .map(|d| d.name.clone())
                .unwrap_or_default(),
            error: e.to_string(),
        };
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx))
            .into_response();
    }
    if let Err(e) = node::authorize_create_in(&state.db, destination.as_ref(), &actor).await {
        let page = VNodeMoveFormPage {
            id,
            name: n.name,
            is_directory: n.is_directory,
            destination_id: form.destination_id,
            destination_display: destination
                .as_ref()
                .map(|d| d.name.clone())
                .unwrap_or_default(),
            error: e.to_string(),
        };
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx))
            .into_response();
    }
    let name = n.name.clone();
    let is_directory = n.is_directory;
    match node::move_to(&state.db, n, destination.as_ref()).await {
        Ok(_) => htmx.redirect(&VNodeDetailRouteTag::new(id).url()),
        Err(e) => {
            let destination_display = destination.map(|d| d.name).unwrap_or_default();
            let page = VNodeMoveFormPage {
                id,
                name,
                is_directory,
                destination_id: form.destination_id,
                destination_display,
                error: e.to_string(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
        }
    }
}
