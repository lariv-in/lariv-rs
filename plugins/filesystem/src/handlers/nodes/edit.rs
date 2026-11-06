use axum::{
    extract::{Multipart, Path, Query},
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    forms::VNodeEditForm,
    handlers::ModalNameQuery,
    keys::{V_NODE_DELETE_MODAL, V_NODE_EDIT_MODAL},
    node,
    permissions::AccessActor,
    routes::{VNodeDetailRouteTag, VNodeListRouteTag},
    state::FilesystemState,
    templates::{VNodeConfirmDeletePage, VNodeEditModalPage},
};
use lariv_core::components::SharedChromeFolder;
use lariv_core::html_form::{CsrfToken, HtmlForm};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_with_slots, respond_edit_modal_done};
use lariv_plugin_users::middleware::RequireAuth;

use super::slot_ctx;

pub async fn edit_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    use crate::layers::VNodeDetailLoader;
    use lariv_core::layers::LoadById;
    let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    let d = &data;
    let access_error = node::authorize_remove(&d.node, &AccessActor::from_auth(&auth))
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    let has_file = d.node.file_path.as_deref().is_some_and(|p| !p.is_empty());
    let form = VNodeEditModalPage {
        id: d.node.id,
        form_name: q.form_name(),
        name: d.node.name.clone(),
        is_directory: d.node.is_directory,
        has_file,
        error: access_error,
    };
    html_built_page_with_slots(&form, &chrome, &slot_ctx(&auth)).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    use crate::layers::VNodeDetailLoader;
    use lariv_core::layers::LoadById;
    let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    let n = data.node;
    let parsed = match VNodeEditForm::from_multipart(multipart, &csrf).await {
        Ok(v) => v,
        Err(e) => {
            let has_file = n.file_path.as_deref().is_some_and(|p| !p.is_empty());
            let form = VNodeEditModalPage {
                id: n.id,
                form_name: q.form_name(),
                name: n.name.clone(),
                is_directory: n.is_directory,
                has_file,
                error: e.to_string(),
            };
            return html_built_page_with_slots(&form, &chrome, &slot_ctx(&auth)).into_response();
        }
    };
    let actor = AccessActor::from_auth(&auth);
    if let Err(e) = node::authorize_remove(&n, &actor) {
        let has_file = n.file_path.as_deref().is_some_and(|p| !p.is_empty());
        let form = VNodeEditModalPage {
            id: n.id,
            form_name: q.form_name(),
            name: n.name.clone(),
            is_directory: n.is_directory,
            has_file,
            error: e.to_string(),
        };
        return html_built_page_with_slots(&form, &chrome, &slot_ctx(&auth)).into_response();
    }
    let file = parsed.file.map(node::NodeFile::Upload);
    if file.is_some()
        && let Err(e) = node::authorize_change_file(&n, &actor)
    {
        let has_file = n.file_path.as_deref().is_some_and(|p| !p.is_empty());
        let form = VNodeEditModalPage {
            id: n.id,
            form_name: q.form_name(),
            name: parsed.name,
            is_directory: n.is_directory,
            has_file,
            error: e.to_string(),
        };
        return html_built_page_with_slots(&form, &chrome, &slot_ctx(&auth)).into_response();
    }
    let is_directory = n.is_directory;
    let has_file_before = n.file_path.as_deref().is_some_and(|p| !p.is_empty());
    let name = parsed.name;
    match node::update(&state.db, state.store.as_ref(), n, name.clone(), file).await {
        Ok(_) => respond_edit_modal_done(
            &htmx,
            V_NODE_EDIT_MODAL,
            &VNodeDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let form = VNodeEditModalPage {
                id,
                form_name: q.form_name(),
                name,
                is_directory,
                has_file: has_file_before,
                error: e.to_string(),
            };
            html_built_page_with_slots(&form, &chrome, &slot_ctx(&auth)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let node = lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id");
    let access_error = match &node {
        Some(n) => node::authorize_remove(n, &AccessActor::from_auth(&ctx))
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default(),
        None => String::new(),
    };
    let message = match node {
        Some(n) if n.is_directory => {
            format!(
                "Are you sure you want to delete \"{}\" and everything inside it?",
                n.name
            )
        }
        Some(n) => format!("Are you sure you want to delete \"{}\"?", n.name),
        None => "Are you sure you want to delete this item?".to_string(),
    };
    let page = VNodeConfirmDeletePage {
        modal_uid: V_NODE_DELETE_MODAL.id().to_string(),
        message,
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_filesystem.VNodeDeleteForm".into()),
        id,
        error: access_error,
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    use crate::layers::{VNodeDeleter, VNodeDetailLoader};
    use lariv_core::layers::{DeleteEntity, LoadById};
    let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
        return htmx.redirect(&VNodeListRouteTag.url());
    };
    let message = if data.node.is_directory {
        format!(
            "Are you sure you want to delete \"{}\" and everything inside it?",
            data.node.name
        )
    } else {
        format!("Are you sure you want to delete \"{}\"?", data.node.name)
    };
    if let Err(e) = node::authorize_remove(&data.node, &AccessActor::from_auth(&ctx)) {
        let page = VNodeConfirmDeletePage {
            modal_uid: V_NODE_DELETE_MODAL.id().to_string(),
            message,
            form_name: "p_filesystem.VNodeDeleteForm".into(),
            id,
            error: e.to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx)).into_response();
    }
    match VNodeDeleter::delete_model(&state, data).await {
        Ok(_) => htmx.redirect(&VNodeListRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete vnode");
            let page = VNodeConfirmDeletePage {
                modal_uid: V_NODE_DELETE_MODAL.id().to_string(),
                message,
                form_name: "p_filesystem.VNodeDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx)).into_response()
        }
    }
}
