use axum::{
    extract::Query,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde::Deserialize;

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_with_slots, respond_edit_modal_done};
use lariv_plugin_documents::routes::DocumentDefaultRouteTag;
use lariv_plugin_documents::routes::DocumentDetailRouteTag;
use lariv_plugin_users::middleware::RequireAuth;

use super::super::{
    entities::user_signature, keys::SignatureCreateModalKey, scope::find_own_signature,
    state::SigningState, templates::SignatureCreateModalPage,
};

#[derive(Debug, Deserialize, Default)]
pub struct SignatureCreateQuery {
    #[serde(default)]
    pub document: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct EmptyForm {}

pub async fn create_get(
    Cap(state): Cap<SigningState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<SignatureCreateQuery>,
) -> maud::Markup {
    let page = SignatureCreateModalPage {
        document_id: q.document.unwrap_or(0),
        debug_local: state.keys.debug_local(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<SigningState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<SignatureCreateQuery>,
    HtmlFormBody(_form): HtmlFormBody<EmptyForm>,
) -> Response {
    let document_id = q.document.unwrap_or(0);
    if find_own_signature(&state.db, &ctx).await.is_some() {
        return modal_error(
            &chrome,
            &ctx,
            document_id,
            state.keys.debug_local(),
            "You already have a signature",
        );
    }
    let key_ref = match state.keys.create_key(ctx.user.id).await {
        Ok(key_ref) => key_ref,
        Err(err) => {
            return modal_error(
                &chrome,
                &ctx,
                document_id,
                state.keys.debug_local(),
                &err.to_string(),
            );
        }
    };
    let now = Utc::now();
    let row = user_signature::ActiveModel {
        user_id: Set(ctx.user.id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        key_ref: Set(key_ref),
    };
    if let Err(err) = row.insert(&state.db).await {
        return modal_error(
            &chrome,
            &ctx,
            document_id,
            state.keys.debug_local(),
            &err.to_string(),
        );
    }
    let next = if document_id > 0 {
        DocumentDetailRouteTag::new(document_id).url()
    } else {
        DocumentDefaultRouteTag.url()
    };
    respond_edit_modal_done::<SignatureCreateModalKey>(&htmx, &next)
}

fn modal_error(
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    document_id: i64,
    debug_local: bool,
    error: &str,
) -> Response {
    let page = SignatureCreateModalPage {
        document_id,
        debug_local,
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}
