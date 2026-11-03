use maud::{Markup, PreEscaped, html};

use lariv_core::components::{ButtonPost, RenderSlot, SlotCtx, button_post, hx_nav_app_layout};
use lariv_core::http::{AppPaneGet, RouteUrl};

use crate::routes::{
    UsersListRouteTag, UsersLogoutGetRouteTag, UsersLogoutPostRouteTag, UsersSelfRouteTag,
};

use super::effective_user::EffectiveUserSelector;

// Identity tag for the users plugin topbar nav slot contributor.
pub struct UsersTopbarNavTag;

// Topbar nav — kept for tests / optional registration; not registered by default
//.
#[derive(Default)]
pub struct UsersTopbarNav;

impl RenderSlot for UsersTopbarNav {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        users_nav()
    }
}

fn users_nav_link<R: AppPaneGet + RouteUrl + Copy + Default>(label: &str) -> Markup {
    let route = R::default();
    users_nav_href(&route.url(), label)
}

fn users_nav_href(href: &str, label: &str) -> Markup {
    html! {
        (PreEscaped(format!(
            r#"<a class="btn btn-ghost btn-sm" href="{href}"{hx}>{label}</a>"#,
            href = href,
            hx = lariv_core::components::swap::hx_nav_app_layout_for_url(href).as_string(),
            label = label,
        )))
    }
}

#[derive(Default)]
pub struct UsersUserDropdown;

impl RenderSlot for UsersUserDropdown {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup {
        let name = ctx.name.as_deref().unwrap_or("");
        let avatar = name
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".into());
        let user_ok = ctx.name.is_some();
        html! {
            (PreEscaped(
                r##"<details class="dropdown dropdown-end" @click.outside="if(!$event.target.closest('.fk-picker-panel, .fk-modal-container')) $el.removeAttribute('open')">"##,
            ))
            summary class="btn btn-sm btn-circle avatar placeholder" {
                div class="rounded-full w-10" {
                    span class="text-xl" { (avatar) }
                }
            }
            div class="card w-80 my-1.5 card-body shadow dropdown-content border border-base-300 rounded-box z-50 bg-base-100 p-4" {
                div class="flex flex-col gap-1" {
                    div class="font-bold text-lg" { (name) }
                    (EffectiveUserSelector.render_slot(ctx))
                }
                @if user_ok {
                    div class="flex flex-col gap-1 mt-2 pt-2 border-t border-base-300" {
                        (PreEscaped(format!(
                            r##"<a class="btn justify-start w-full" href="{self_url}"{}>My Account</a>"##,
                            hx_nav_app_layout(UsersSelfRouteTag).as_string(),
                            self_url = UsersSelfRouteTag.url(),
                        )))
                        (button_post(ButtonPost {
                            label: "Logout",
                            action: &UsersLogoutPostRouteTag.url(),
                            classes: "btn btn-error justify-start w-full",
                            icon_name: Some("arrow-right-start-on-rectangle"),
                            ..Default::default()
                        }))
                    }
                }
            }
            (PreEscaped("</details>"))
        }
    }
}

fn users_nav() -> Markup {
    html! {
        (users_nav_href(lariv_core::components::nav_origin::DASHBOARD_URL, "Apps"))
        (users_nav_link::<UsersSelfRouteTag>("Profile"))
        (lariv_core::components::authorized_role(
            &crate::role_authorization::roles_for::<crate::routes::UsersAdmin>(),
            html! {
                (users_nav_link::<UsersListRouteTag>("Users"))
            },
        ))
        (PreEscaped(format!(
            r#"<a class="btn btn-ghost btn-sm" href="{href}"{hx}>Logout</a>"#,
            href = UsersLogoutGetRouteTag.url(),
            hx = hx_nav_app_layout(UsersLogoutGetRouteTag).as_string(),
        )))
    }
}
