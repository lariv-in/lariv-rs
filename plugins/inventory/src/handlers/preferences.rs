use axum::response::{IntoResponse, Response};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_filesystem::state::FilesystemState;
use lariv_plugin_users::middleware::RequireAuth;

use crate::forms::InventoryPreferencesForm;
use crate::logic::pdf_assets::vnode_label;
use crate::logic::preferences;
use crate::routes::InventoryPrefsGetRouteTag;
use crate::state::InventoryState;
use crate::templates::InventoryPreferencesPage;

fn render_page(
    page: InventoryPreferencesPage,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    htmx: &Htmx,
) -> Response {
    html_built_page_or_app_layout(&page, htmx, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn get(
    Cap(state): Cap<InventoryState>,
    Cap(fs): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    match preferences::load_preferences(&state.db).await {
        Ok(prefs) => {
            let logo = vnode_label(&fs.db, prefs.logo_vnode_id).await;
            let signature = vnode_label(&fs.db, prefs.signature_vnode_id).await;
            render_page(
                InventoryPreferencesPage::from_model(&prefs, logo, signature, String::new()),
                &chrome,
                &ctx,
                &htmx,
            )
        }
        Err(err) => render_page(InventoryPreferencesPage::empty(err), &chrome, &ctx, &htmx),
    }
}

pub async fn post(
    Cap(state): Cap<InventoryState>,
    Cap(fs): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<InventoryPreferencesForm>,
) -> Response {
    match preferences::save_preferences(&state.db, &form).await {
        Ok(()) => htmx.redirect(&InventoryPrefsGetRouteTag.url()),
        Err(err) => {
            let logo =
                vnode_label(&fs.db, preferences::parse_optional_id(&form.logo_vnode_id)).await;
            let signature = vnode_label(
                &fs.db,
                preferences::parse_optional_id(&form.signature_vnode_id),
            )
            .await;
            render_page(
                InventoryPreferencesPage::from_form(form, logo, signature, err),
                &chrome,
                &ctx,
                &htmx,
            )
        }
    }
}
