use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        ButtonPost, RenderSlot, ShellChrome, ShellTopbar, SlotCapability, SlotRegistrar, SlotCtx,
        SlotOf, TopbarItemsSlotTag, button_post, icon, shell_topbar,
    },
    http::ProvideRequestCaps,
    plugins::dashboard::AppTile,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

use super::DashboardTag;

define_register_items! {
    plugin: DashboardTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        AppsIdx: DashboardAppsPageTag => AppsPage,
    ]
}

// Topbar slot contributors (mirrors Go `dashboard.appsPageButton` / theme / userDropdown).
#[derive(Default)]
pub struct DashboardAppsPageButton;

impl RenderSlot for DashboardAppsPageButton {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        html! {
            a href="/dashboard/" class="btn btn-sm btn-square btn-neutral" {
                (icon("squares-2x2", ""))
            }
        }
    }
}

#[derive(Default)]
pub struct DashboardThemeButton;

impl RenderSlot for DashboardThemeButton {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        // Alpine attrs via PreEscaped (colon / @ prefixes).
        html! {
            (PreEscaped(
                r##"<button type="button" class="btn items-center btn-sm btn-square btn-outline" @click="toggleTheme()">"##,
            ))
            (PreEscaped(r##"<span class="inline-flex items-center justify-center" x-show="theme === 'light'">"##))
            (icon("sun", ""))
            (PreEscaped("</span>"))
            (PreEscaped(r##"<span class="inline-flex items-center justify-center" x-show="theme !== 'light'">"##))
            (icon("moon", ""))
            (PreEscaped("</span></button>"))
        }
    }
}

#[derive(Default)]
pub struct DashboardUserDropdown;

impl RenderSlot for DashboardUserDropdown {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup {
        let name = ctx.name.as_deref().unwrap_or("");
        let role = ctx.role.as_deref().unwrap_or("");
        let avatar = name
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".into());
        let user_ok = ctx.name.is_some();
        html! {
            (PreEscaped(
                r##"<details class="dropdown dropdown-end" @click.outside="$el.removeAttribute('open')">"##,
            ))
            summary class="btn btn-sm btn-circle avatar placeholder" {
                div class="rounded-full w-10" {
                    span class="text-xl" { (avatar) }
                }
            }
            div class="card w-64 my-1.5 card-body shadow dropdown-content border border-base-300 rounded-box z-50 bg-base-100 p-4" {
                div class="flex flex-col gap-1" {
                    div class="font-bold text-lg" { (name) }
                    div class="text-sm opacity-70 cursor-default" { (role) }
                }
                @if user_ok {
                    div class="flex flex-col gap-1 mt-2 pt-2 border-t border-base-300" {
                        a class="btn justify-start w-full" href="/users/self/" { "My Account" }
                        (button_post(ButtonPost {
                            label: "Logout",
                            action: "/users/logout/",
                            // Go UserDropdown passes "btn btn-error…" then Build adds another "btn ".
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

fn apps_grid(apps: &[AppTile]) -> Markup {
    html! {
        (PreEscaped(
            r##"<div class="container max-w-5xl mx-auto mt-4 @container" x-data="{ search: ''}">"##,
        ))
        div class="mb-4" {
            (PreEscaped(
                r##"<input type="text" x-model="search" placeholder="Search apps..." class="input input-bordered w-full">"##,
            ))
        }
        (PreEscaped(r##"<div class="grid grid-cols-2 @md:grid-cols-4 @2xl:grid-cols-6 gap-2">"##))
        @for app in apps {
            (PreEscaped(format!(
                r##"<a href="{href}" class="btn btn-md h-auto flex-col space-y-1 py-4" x-show="'{name}'.toLowerCase().includes(search.toLowerCase())" x-cloak>"##,
                href = html_escape_attr(&ensure_trailing_slash(&app.href)),
                name = html_escape_js_string(&app.verbose_name),
            )))
            (icon(&app.icon, "w-8 h-8"))
            div class="text-sm truncate min-w-0 w-full" { (app.verbose_name) }
            (PreEscaped("</a>"))
        }
        (PreEscaped("</div></div>"))
    }
}

fn html_escape_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn html_escape_js_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out
}

fn ensure_trailing_slash(path: &str) -> String {
    if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    }
}

#[derive(Generic)]
pub struct AppsPage {
    pub name: String,
    pub role: String,
    pub avatar: String,
    pub is_superuser: bool,
    pub apps: Vec<AppTile>,
}

impl AppsPage {
    fn pane_body(&self) -> Markup {
        apps_grid(&self.apps)
    }
}

impl crate::template::RenderAppPane for AppsPage {
    fn render_pane(&self) -> Markup {
        use crate::components::swap::{AppLayoutKey, SwapKey};
        html! {
            (PreEscaped(format!(
                r#"<div id="{}" class="size-full overflow-y-auto p-4">"#,
                AppLayoutKey::ID
            )))
            (self.pane_body())
            (PreEscaped("</div>"))
        }
    }
}

impl RenderTemplate for AppsPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        // Topbar comes from slots (apps button, theme, user dropdown) — same as Go Catalog.
        // shell_topbar wraps body in `#app-layout` (do not also call layout_simple).
        shell_topbar(ShellTopbar {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            body: self.pane_body(),
            ..Default::default()
        })
    }
}

// add() prepends — register user, theme, apps so display order is apps, theme, user.
define_register_items! {
    plugin: DashboardTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    wrapper: SlotOf;
    bounds: [];
    hook: SlotsHook;
    items: [
        AppsBtnIdx: DashboardAppsPageButtonTag, TopbarItemsSlotTag => DashboardAppsPageButton,
        ThemeBtnIdx: DashboardThemeButtonTag, TopbarItemsSlotTag => DashboardThemeButton,
        UserDropdownIdx: DashboardUserDropdownTag, TopbarItemsSlotTag => DashboardUserDropdown,
    ]
}
