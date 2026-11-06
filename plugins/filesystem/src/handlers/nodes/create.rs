use axum::{
    extract::{Multipart, Path, Query},
    response::{IntoResponse, Response},
};

use crate::{
    forms::{VNodeForm, VNodeKindSubmit},
    handlers::ModalNameQuery,
    keys::V_NODE_CREATE_MODAL,
    node,
    permissions::AccessActor,
    routes::VNodeDetailRouteTag,
    state::FilesystemState,
    templates::VNodeCreateModalPage,
};
use lariv_core::components::SharedChromeFolder;
use lariv_core::html_form::{CsrfToken, HtmlForm};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_with_slots, respond_create_modal_done_fk};
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::state::AuthContext;

use super::slot_ctx;

async fn render_create_get(
    state: FilesystemState,
    auth: AuthContext,
    chrome: SharedChromeFolder,
    q: ModalNameQuery,
    parent_id: Option<i64>,
) -> maud::Markup {
    let parent = match parent_id {
        Some(id) => {
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        }
        None => None,
    };
    let error =
        match node::authorize_create_in(&state.db, parent.as_ref(), &AccessActor::from_auth(&auth))
            .await
        {
            Ok(()) => String::new(),
            Err(e) => e.to_string(),
        };
    let page = VNodeCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        is_directory: false,
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&auth))
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    render_create_get(state, auth, chrome, q, None).await
}

/// HTTP handler: `create_get_in`.
pub async fn create_get_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
) -> maud::Markup {
    render_create_get(state, auth, chrome, q, Some(parent_id)).await
}

async fn render_create_post(
    state: FilesystemState,
    auth: AuthContext,
    chrome: SharedChromeFolder,
    htmx: Htmx,
    q: ModalNameQuery,
    parent_id_from_route: Option<i64>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    let parsed = match VNodeForm::from_multipart(multipart, &csrf).await {
        Ok(p) => p,
        Err(e) => {
            return render_create_error(
                &state,
                &chrome,
                &auth,
                &q,
                parent_id_from_route,
                String::new(),
                false,
                e.to_string(),
            )
            .await;
        }
    };
    let parent_id = parent_id_from_route.or(parsed.parent_id.filter(|id| *id != 0));
    let parent = match parent_id {
        Some(id) => {
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        }
        None => None,
    };
    let (is_directory, file) = match parsed.kind {
        VNodeKindSubmit::Directory => (true, None),
        VNodeKindSubmit::File { file } => (false, Some(node::NodeFile::Upload(file))),
    };
    if let Err(e) =
        node::authorize_create_in(&state.db, parent.as_ref(), &AccessActor::from_auth(&auth)).await
    {
        return render_create_error(
            &state,
            &chrome,
            &auth,
            &q,
            parent_id,
            parsed.name,
            is_directory,
            e.to_string(),
        )
        .await;
    }
    let created = if is_directory {
        node::create(
            &state.db,
            state.store.as_ref(),
            parsed.name.clone(),
            true,
            file,
            parent.as_ref(),
            Some(auth.user.id),
            Some(auth.user.role.clone()),
        )
        .await
    } else {
        node::create_owned(
            &state.db,
            state.store.as_ref(),
            parsed.name.clone(),
            false,
            file,
            parent.as_ref(),
            auth.user.id,
        )
        .await
    };
    match created {
        Ok(created) => respond_create_modal_done_fk(
            &htmx,
            V_NODE_CREATE_MODAL,
            &q.refresh_table(),
            &VNodeDetailRouteTag::new(created.id).url(),
            created.id,
            &created.name,
            &q.target_input(),
        ),
        Err(e) => {
            render_create_error(
                &state,
                &chrome,
                &auth,
                &q,
                parent_id,
                parsed.name,
                is_directory,
                e.to_string(),
            )
            .await
        }
    }
}

async fn render_create_error(
    state: &FilesystemState,
    chrome: &SharedChromeFolder,
    auth: &AuthContext,
    q: &ModalNameQuery,
    parent_id: Option<i64>,
    name: String,
    is_directory: bool,
    error: String,
) -> Response {
    let parent = match parent_id {
        Some(id) => {
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        }
        None => None,
    };
    let page = VNodeCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name,
        is_directory,
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &slot_ctx(auth)).into_response()
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    render_create_post(state, auth, chrome, htmx, q, None, csrf, multipart).await
}

/// HTTP handler: `create_post_in`.
pub async fn create_post_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    render_create_post(
        state,
        auth,
        chrome,
        htmx,
        q,
        Some(parent_id),
        csrf,
        multipart,
    )
    .await
}
