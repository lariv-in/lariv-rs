use axum::response::{IntoResponse, Redirect, Response};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    duration::format_duration,
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{Htmx, html_built_page_or_app_layout},
};

use super::super::{
    forms::PreferencesForm,
    preferences::{load_preferences, parse_validity, save_preferences, validate_authority_name},
    routes::{DocumentDefaultRouteTag, DocumentPrefsGetRouteTag},
    state::DocumentsState,
    templates::DocumentPreferencesPage,
};

fn page(name: String, validity: String, error: String) -> DocumentPreferencesPage {
    DocumentPreferencesPage {
        signing_authority_name: name,
        validity_duration: validity,
        error,
    }
}

pub async fn get(
    Cap(state): Cap<DocumentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&DocumentDefaultRouteTag.url()).into_response();
    }
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match load_preferences(&state.db).await {
        Ok(prefs) => prefs,
        Err(err) => {
            let page = page(String::new(), String::new(), err.to_string());
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = page(
        prefs.signing_authority_name,
        format_duration(prefs.validity_duration),
        String::new(),
    );
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

pub async fn post(
    Cap(state): Cap<DocumentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<PreferencesForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&DocumentDefaultRouteTag.url()).into_response();
    }
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let show_error = |error: String| {
        let page = page(
            form.signing_authority_name.clone(),
            form.validity_duration.clone(),
            error,
        );
        html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
    };
    let name = match validate_authority_name(&form.signing_authority_name) {
        Ok(name) => name.to_string(),
        Err(err) => return show_error(err),
    };
    let validity = match parse_validity(&form.validity_duration) {
        Ok(validity) => validity,
        Err(err) => return show_error(err),
    };
    match save_preferences(&state.db, name, validity).await {
        Ok(_) => htmx.redirect(&DocumentPrefsGetRouteTag.url()),
        Err(err) => show_error(err.to_string()),
    }
}
