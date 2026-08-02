use axum::response::Redirect;

use crate::{
    apps::AppsCapability,
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::{
        dashboard::templates::AppsPage,
        users::middleware::{OptionalAuth, RequireAuth},
    },
    web::{Htmx, html_built_page_or_app_layout},
};

/// Go `core.HomeView` patch: logged-in → dashboard, guest → login.
pub async fn home_redirect(OptionalAuth(auth): OptionalAuth) -> Redirect {
    if auth.is_some() {
        Redirect::to("/dashboard")
    } else {
        Redirect::to("/users/login")
    }
}

/// Apps launchpad (requires auth), matching Go `dashboard.AppsView`.
///
/// Tiles come from the App's [`AppsCapability`] (Go `App.Plugins`), not a dashboard snapshot.
pub async fn apps(
    Cap(catalog): Cap<AppsCapability>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup {
    let apps = catalog.visible_apps(&ctx.role, ctx.user.is_superuser);
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
        is_superuser: ctx.user.is_superuser,
        apps,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}
