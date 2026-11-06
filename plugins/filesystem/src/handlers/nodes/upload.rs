use axum::{
    extract::{Multipart, Path, Query},
    response::{IntoResponse, Response},
};

use crate::{
    forms::{VNodeMultiUploadForm, VNodeZipUploadForm},
    handlers::ModalNameQuery,
    keys::{V_NODE_MULTI_UPLOAD_MODAL, V_NODE_ZIP_UPLOAD_MODAL},
    node,
    permissions::AccessActor,
    routes::{VNodeBrowseRouteTag, VNodeListRouteTag},
    state::FilesystemState,
    templates::{VNodeMultiUploadModalPage, VNodeZipUploadModalPage},
    zip,
};
use lariv_core::components::SharedChromeFolder;
use lariv_core::html_form::{CsrfToken, HtmlForm};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_with_slots, respond_create_modal_done};
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::state::AuthContext;

use super::slot_ctx;

async fn render_multi_upload_get(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
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
        match node::authorize_create_in(&state.db, parent.as_ref(), &AccessActor::from_auth(&ctx))
            .await
        {
            Ok(()) => String::new(),
            Err(e) => e.to_string(),
        };
    let page = VNodeMultiUploadModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx))
}

async fn render_multi_upload_error(
    state: &FilesystemState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalNameQuery,
    parent_id: Option<i64>,
    error: String,
) -> Response {
    let parent = match parent_id {
        Some(id) => {
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        }
        None => None,
    };
    let page = VNodeMultiUploadModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &slot_ctx(ctx)).into_response()
}

async fn render_multi_upload_post(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
    htmx: Htmx,
    q: ModalNameQuery,
    parent_id_from_route: Option<i64>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    let parsed = match VNodeMultiUploadForm::from_multipart(multipart, &csrf).await {
        Ok(p) => p,
        Err(e) => {
            return render_multi_upload_error(
                &state,
                &chrome,
                &ctx,
                &q,
                parent_id_from_route,
                e.to_string(),
            )
            .await;
        }
    };
    let parent_id = parent_id_from_route.or(parsed.parent_id.filter(|id| *id != 0));
    if parsed.files.is_empty() {
        return render_multi_upload_error(
            &state,
            &chrome,
            &ctx,
            &q,
            parent_id,
            "please choose at least one file".into(),
        )
        .await;
    }
    let parent = match parent_id {
        Some(id) => {
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        }
        None => None,
    };
    if let Err(e) =
        node::authorize_create_in(&state.db, parent.as_ref(), &AccessActor::from_auth(&ctx)).await
    {
        return render_multi_upload_error(&state, &chrome, &ctx, &q, parent_id, e.to_string())
            .await;
    }
    let mut first_error = None;
    for file in parsed.files {
        let filename = file.filename().to_string();
        if let Err(e) = node::create(
            &state.db,
            state.store.as_ref(),
            filename,
            false,
            Some(node::NodeFile::Upload(file)),
            parent.as_ref(),
            Some(ctx.user.id),
            Some(ctx.user.role.clone()),
        )
        .await
        {
            first_error.get_or_insert_with(|| e.to_string());
        }
    }
    if let Some(err) = first_error {
        return render_multi_upload_error(&state, &chrome, &ctx, &q, parent_id, err).await;
    }
    let redirect_url = match parent_id {
        Some(id) => VNodeBrowseRouteTag::new(id).url(),
        None => VNodeListRouteTag.url(),
    };
    respond_create_modal_done(
        &htmx,
        V_NODE_MULTI_UPLOAD_MODAL,
        &q.refresh_table(),
        &redirect_url,
    )
}

/// HTTP handler: `upload_get`.
pub async fn upload_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    render_multi_upload_get(state, chrome, ctx, q, None).await
}

/// HTTP handler: `upload_get_in`.
pub async fn upload_get_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
) -> maud::Markup {
    render_multi_upload_get(state, chrome, ctx, q, Some(parent_id)).await
}

/// HTTP handler: `upload_post`.
pub async fn upload_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    render_multi_upload_post(state, chrome, ctx, htmx, q, None, csrf, multipart).await
}

/// HTTP handler: `upload_post_in`.
pub async fn upload_post_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    render_multi_upload_post(
        state,
        chrome,
        ctx,
        htmx,
        q,
        Some(parent_id),
        csrf,
        multipart,
    )
    .await
}

// ---------------------------------------------------------------------------
// Zip upload
// ---------------------------------------------------------------------------

async fn render_zip_upload_get(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
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
        match node::authorize_create_in(&state.db, parent.as_ref(), &AccessActor::from_auth(&ctx))
            .await
        {
            Ok(()) => String::new(),
            Err(e) => e.to_string(),
        };
    let page = VNodeZipUploadModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx))
}

async fn render_zip_upload_error(
    state: &FilesystemState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalNameQuery,
    parent_id: Option<i64>,
    error: String,
) -> Response {
    let parent = match parent_id {
        Some(id) => {
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        }
        None => None,
    };
    let page = VNodeZipUploadModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &slot_ctx(ctx)).into_response()
}

async fn render_zip_upload_post(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
    htmx: Htmx,
    q: ModalNameQuery,
    parent_id_from_route: Option<i64>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    let parsed = match VNodeZipUploadForm::from_multipart(multipart, &csrf).await {
        Ok(p) => p,
        Err(e) => {
            return render_zip_upload_error(
                &state,
                &chrome,
                &ctx,
                &q,
                parent_id_from_route,
                e.to_string(),
            )
            .await;
        }
    };
    let parent_id = parent_id_from_route.or(parsed.parent_id.filter(|id| *id != 0));
    let zip_bytes = match parsed.zip_file.into_bytes().await {
        Ok(b) => b,
        Err(e) => {
            return render_zip_upload_error(&state, &chrome, &ctx, &q, parent_id, e.to_string())
                .await;
        }
    };
    let parent = match parent_id {
        Some(id) => {
            lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
        }
        None => None,
    };
    if let Err(e) =
        node::authorize_create_in(&state.db, parent.as_ref(), &AccessActor::from_auth(&ctx)).await
    {
        return render_zip_upload_error(&state, &chrome, &ctx, &q, parent_id, e.to_string()).await;
    }
    match zip::replace_children_from_zip(
        &state.db,
        state.store.as_ref(),
        parent.as_ref(),
        &zip_bytes,
        Some(ctx.user.id),
        Some(ctx.user.role.clone()),
    )
    .await
    {
        Ok(()) => {
            let redirect_url = match parent_id {
                Some(id) => VNodeBrowseRouteTag::new(id).url(),
                None => VNodeListRouteTag.url(),
            };
            respond_create_modal_done(
                &htmx,
                V_NODE_ZIP_UPLOAD_MODAL,
                &q.refresh_table(),
                &redirect_url,
            )
        }
        Err(e) => {
            render_zip_upload_error(&state, &chrome, &ctx, &q, parent_id, e.to_string()).await
        }
    }
}

/// HTTP handler: `zip_upload_get`.
pub async fn zip_upload_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    render_zip_upload_get(state, chrome, ctx, q, None).await
}

/// HTTP handler: `zip_upload_get_in`.
pub async fn zip_upload_get_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
) -> maud::Markup {
    render_zip_upload_get(state, chrome, ctx, q, Some(parent_id)).await
}

/// HTTP handler: `zip_upload_post`.
pub async fn zip_upload_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    render_zip_upload_post(state, chrome, ctx, htmx, q, None, csrf, multipart).await
}

/// HTTP handler: `zip_upload_post_in`.
pub async fn zip_upload_post_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    render_zip_upload_post(
        state,
        chrome,
        ctx,
        htmx,
        q,
        Some(parent_id),
        csrf,
        multipart,
    )
    .await
}
