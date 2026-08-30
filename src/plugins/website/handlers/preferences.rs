//! Website preferences (Custom theme CSS/JS VNodes).

use axum::response::{IntoResponse, Response};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        filesystem::entities::filesystem_node::Entity as VNodeEntity,
        users::middleware::RequireAuth,
        website::{
            entities::WebsitePreferences,
            forms::PreferencesForm,
            preferences::{load_preferences, save_preferences},
            state::WebsiteState,
            templates::WebsitePreferencesPage,
        },
    },
    web::{Htmx, html_built_page_or_app_layout},
};
use sea_orm::EntityTrait;

async fn vnode_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|&id| id > 0) else {
        return String::new();
    };
    crate::web::opt_or_log(VNodeEntity::find_by_id(id).one(db).await, "find theme vnode")
        .map(|n| n.name)
        .unwrap_or_default()
}

fn prefs_page(
    prefs: &WebsitePreferences,
    css_display: String,
    js_display: String,
    error: String,
) -> WebsitePreferencesPage {
    WebsitePreferencesPage {
        custom_theme_css_vnode_id: prefs.custom_theme_css_vnode_id.filter(|&id| id > 0),
        custom_theme_css_display: css_display,
        custom_theme_js_vnode_id: prefs.custom_theme_js_vnode_id.filter(|&id| id > 0),
        custom_theme_js_display: js_display,
        error,
    }
}

/// HTTP handler: `get`.
pub async fn get(
    Cap(state): Cap<WebsiteState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match load_preferences(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            let page = WebsitePreferencesPage {
                custom_theme_css_vnode_id: None,
                custom_theme_css_display: String::new(),
                custom_theme_js_vnode_id: None,
                custom_theme_js_display: String::new(),
                error: e.to_string(),
            };
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let css_display = vnode_display(&state.db, prefs.custom_theme_css_vnode_id).await;
    let js_display = vnode_display(&state.db, prefs.custom_theme_js_vnode_id).await;
    let page = prefs_page(&prefs, css_display, js_display, String::new());
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

/// HTTP handler: `post`.
pub async fn post(
    Cap(state): Cap<WebsiteState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<PreferencesForm>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);

    let prefs = WebsitePreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        custom_theme_css_vnode_id: form.custom_theme_css_vnode_id.filter(|&id| id > 0),
        custom_theme_js_vnode_id: form.custom_theme_js_vnode_id.filter(|&id| id > 0),
    };

    match save_preferences(&state.db, prefs.clone()).await {
        Ok(_) => htmx.redirect("/website/preferences"),
        Err(e) => {
            let css_display = vnode_display(&state.db, prefs.custom_theme_css_vnode_id).await;
            let js_display = vnode_display(&state.db, prefs.custom_theme_js_vnode_id).await;
            let page = prefs_page(&prefs, css_display, js_display, e.to_string());
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}
