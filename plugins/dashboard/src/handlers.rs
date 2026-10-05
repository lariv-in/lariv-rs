//! HTTP handlers for `/` (auth redirect) and `/dashboard/` (apps launchpad).
use axum::response::Redirect;

use lariv_core::apps::AppsCapability;
use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use crate::routes::DashboardAppsRouteTag;
use crate::templates::AppsPage;
use lariv_plugin_users::middleware::OptionalAuth;
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::routes::UsersLoginGetRouteTag;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};

/// `GET /` — logged-in → dashboard, guest → login.
pub async fn home_redirect(OptionalAuth(auth): OptionalAuth) -> Redirect {
    if auth.is_some() {
        Redirect::to(&DashboardAppsRouteTag.url())
    } else {
        Redirect::to(&UsersLoginGetRouteTag.url())
    }
}

/// Apps launchpad (requires auth).
///
/// Tiles come from the App's [`AppsCapability`], not a dashboard snapshot.
pub async fn apps(
    Cap(catalog): Cap<AppsCapability>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup {
    let apps = catalog.visible_apps(&ctx.role);
    let avatar = ctx
        .user
        .name
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "?".into());
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let page = AppsPage {
        name: ctx.user.name.clone(),
        role: ctx.role.clone(),
        avatar,
        apps,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}
