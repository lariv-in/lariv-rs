use axum::response::Redirect;
use frunk::{Generic, hlist};

use crate::{
    apps::AppsCapability,
    components::{FoldSlots, SlotCapability, SlotCtx},
    http::Cap,
    plugins::{
        dashboard::templates::{AppsPage, DashboardAppsPageTag},
        users::middleware::{OptionalAuth, RequireAuth},
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout},
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
pub async fn apps<Templates, Slots, Idx, P>(
    Cap(catalog): Cap<AppsCapability>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<DashboardAppsPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <AppsPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let apps = catalog.visible_apps(&ctx.role, ctx.user.is_superuser);
    let avatar = ctx
        .user
        .name
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "?".into());
    let slot_ctx = SlotCtx {
        name: Some(ctx.user.name.clone()),
        role: Some(ctx.role.clone()),
        is_superuser: ctx.user.is_superuser,
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            ctx.user.name,
            ctx.role,
            avatar,
            ctx.user.is_superuser,
            apps,
        ],
        &slots,
        &slot_ctx,
    )
}
