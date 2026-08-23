//! Maud templates for dashboard apps grid and topbar slot hooks.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        RenderSlot, ShellChrome, ShellTopbar, SlotCapability, SlotCtx, SlotOf, SlotRegistrar,
        TopbarItemsSlotTag, dashboard_app_href, hx_nav_app_layout, hx_nav_app_layout_for_url, icon,
        shell_topbar,
    },
    http::ProvideRequestCaps,
    plugins::dashboard::AppTile,
    plugins::dashboard::routes::DashboardAppsRouteTag,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

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

// Topbar slot contributors.
#[derive(Default)]
pub struct DashboardAppsPageButton;

impl RenderSlot for DashboardAppsPageButton {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        html! {
            (PreEscaped(format!(
                r##"<a href="/dashboard/" class="btn btn-sm btn-square btn-neutral"{}>"##,
                hx_nav_app_layout(DashboardAppsRouteTag).as_string(),
            )))
            (icon("squares-2x2", ""))
            (PreEscaped("</a>"))
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
            (PreEscaped({
                let href = dashboard_app_href(&app.href);
                format!(
                    r##"<a href="{href}" class="btn btn-md h-auto flex-col space-y-1 py-4" x-show="'{name}'.toLowerCase().includes(search.toLowerCase())" x-cloak{hx}>"##,
                    href = html_escape_attr(&href),
                    name = html_escape_js_string(&app.verbose_name),
                    hx = hx_nav_app_layout_for_url(&href).as_string(),
                )
            }))
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        use crate::components::app_layout_pane;
        app_layout_pane(self.pane_body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        use crate::components::{LayoutMain, layout_main};
        layout_main(LayoutMain {
            breadcrumbs: Markup::default(),
            content: self.pane_body(),
        })
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
            right_sidebar: chrome.right_sidebar.clone(),
            body: self.pane_body(),
            ..Default::default()
        })
    }
}

// add() prepends — register theme, apps so display order is apps, theme, (user from p_users).
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
    ]
}
