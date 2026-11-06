use axum::{
    extract::Path,
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    forms::VNodeContentForm, node, permissions::AccessActor, routes::VNodeListRouteTag,
    state::FilesystemState,
};
use lariv_core::components::SharedChromeFolder;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_users::middleware::RequireAuth;

use super::{require_view, slot_ctx, vnode_detail_page};

pub async fn detail(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    // Layer stack `vnode_detail_layers()` is equivalent; run_layers inside Route::get
    // hits rustc #100013, so execute the detail loader directly here.
    use crate::layers::VNodeDetailLoader;
    use lariv_core::layers::LoadById;
    let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    if let Err(response) = require_view(&data.node, &auth) {
        return response;
    }
    let mut detail = vnode_detail_page(
        &data,
        &auth.timezone,
        data.text_content.clone(),
        String::new(),
    );
    if data.node.is_directory {
        let actor = AccessActor::from_auth(&auth);
        detail.items_display = node::count_visible_children(&state.db, data.node.id, &actor)
            .await
            .unwrap_or(0)
            .to_string();
    }
    html_built_page_or_app_layout(&detail, &htmx, &chrome, &slot_ctx(&auth)).into_response()
}

pub async fn content_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<VNodeContentForm>,
) -> Response {
    use crate::layers::VNodeDetailLoader;
    use lariv_core::layers::LoadById;
    let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    if let Err(response) = require_view(&data.node, &auth) {
        return response;
    }
    if let Err(e) = node::authorize_change_file(&data.node, &AccessActor::from_auth(&auth)) {
        let detail = vnode_detail_page(&data, &auth.timezone, Some(form.content), e.to_string());
        return html_built_page_or_app_layout(&detail, &htmx, &chrome, &slot_ctx(&auth))
            .into_response();
    }
    if data.node.is_directory {
        let detail = vnode_detail_page(
            &data,
            &auth.timezone,
            None,
            "cannot edit contents of a directory".into(),
        );
        return html_built_page_or_app_layout(&detail, &htmx, &chrome, &slot_ctx(&auth))
            .into_response();
    }
    let content = form.content;
    if let Err(e) = node::require_text_content_size(&content) {
        let detail = vnode_detail_page(&data, &auth.timezone, Some(content), e);
        return html_built_page_or_app_layout(&detail, &htmx, &chrome, &slot_ctx(&auth))
            .into_response();
    }
    let filename = data.node.name.clone();
    match node::update(
        &state.db,
        state.store.as_ref(),
        data.node.clone(),
        filename.clone(),
        Some(node::NodeFile::Bytes {
            filename,
            data: content.clone().into_bytes(),
        }),
    )
    .await
    {
        Ok(_) => {
            let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
                return Redirect::to(&VNodeListRouteTag.url()).into_response();
            };
            let detail = vnode_detail_page(
                &data,
                &auth.timezone,
                data.text_content.clone(),
                String::new(),
            );
            html_built_page_or_app_layout(&detail, &htmx, &chrome, &slot_ctx(&auth)).into_response()
        }
        Err(e) => {
            let detail = vnode_detail_page(&data, &auth.timezone, Some(content), e.to_string());
            html_built_page_or_app_layout(&detail, &htmx, &chrome, &slot_ctx(&auth)).into_response()
        }
    }
}
