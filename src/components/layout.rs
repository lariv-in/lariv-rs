//! Page layout containers.

use maud::{Markup, PreEscaped, html};

use crate::components::swap::{AppLayoutKey, MainContentKey, SwapKey};
use crate::components::text::icon;

pub struct LayoutCard;
pub struct LayoutSimple;
pub struct LayoutSidebar {
    pub sidebar: Markup,
    pub content: Markup,
}
pub struct LayoutTopbar {
    pub topbar_items: Markup,
    pub content: Markup,
    pub has_sidebar: bool,
    pub x_data: Option<&'static str>,
    pub right_panels: Markup,
}

pub fn layout_card(children: Markup) -> Markup {
    html! {
        div class="min-h-screen flex items-center justify-center bg-base-200" {
            // Go layout_card: progress shares the loading-indicator id with shell_base.
            (PreEscaped(
                r#"<progress class="progress w-full fixed top-0 left-0 h-1 z-50" id="global-loading-indicator"></progress>"#,
            ))
            div class="card shadow-xl" {
                div class="card-body" {
                    // Same swap root as scaffold so hx-boost can move between auth ↔ app.
                    (PreEscaped(format!(r#"<div id="{}">"#, AppLayoutKey::ID)))
                    (children)
                    (PreEscaped("</div>"))
                }
            }
        }
    }
}

pub fn layout_simple(children: Markup) -> Markup {
    html! {
        div class="size-full overflow-y-auto p-4" {
            (PreEscaped(format!(r#"<div id="{}">"#, AppLayoutKey::ID)))
            (children)
            (PreEscaped("</div>"))
        }
    }
}

pub fn layout_sidebar(opts: LayoutSidebar) -> Markup {
    // Alpine bindings use PreEscaped so colon-prefixed attrs parse correctly.
    // XData matches Go layout_sidebar.go (showLeft from viewport width).
    html! {
        (PreEscaped(
            r##"<div id="app-layout" class="size-full" x-data="{
        showLeft: window.innerWidth >= 768,
        isMobile: window.innerWidth < 768,
        messages: []
}">"##,
        ))
        (PreEscaped(r##"<div class="grid h-full transition-[grid-template-columns] duration-[400ms] ease-in" :class="isMobile ? 'grid-cols-1' : (showLeft ? 'grid-cols-[250px_1fr]' : 'grid-cols-[0px_1fr]')">"##))
        (PreEscaped(r##"<div x-show="isMobile && showLeft" x-transition.opacity="" @click="showLeft = false" class="absolute inset-x-0 bottom-0 top-16 bg-black/50 z-20"></div>"##))
        (PreEscaped(r##"<aside class="bg-base-100 border-r border-base-300 overflow-hidden max-md:absolute max-md:left-0 max-md:top-16 max-md:z-50 max-md:h-[calc(100vh-4rem)] max-md:shadow-xl max-md:transition-all max-md:duration-300 max-md:-translate-x-full" :style="isMobile && showLeft ? 'translate: none' : ''">"##))
        div class="h-full overflow-y-auto w-[250px] bg-base-100 p-2" {
            (opts.sidebar)
        }
        (PreEscaped("</aside>"))
        (layout_main(opts.content))
        (PreEscaped("</div></div>"))
    }
}

/// `<main id="main-content">` column — sidebar menu swaps this, not `#app-layout`.
pub fn layout_main(content: Markup) -> Markup {
    let menu = icon("bars-3", "");
    html! {
        (PreEscaped(format!(
            r##"<main id="{}" class="overflow-y-auto p-4 relative h-full bg-base-100">"##,
            MainContentKey::ID
        )))
        (PreEscaped(r##"<button @click="showLeft = !showLeft" class="btn btn-sm btn-square mb-2">"##))
        (menu)
        (PreEscaped("</button>"))
        div class="messages mb-4" {
            (PreEscaped(
                r##"<template x-for="msg in messages"><div class="alert shadow-lg mb-2" :class="msg.tags == 'error' ? 'alert-error' : (msg.tags == 'success' ? 'alert-success' : 'alert-info')"><div class="flex-1"><span class="font-semibold" x-text="msg.tags.charAt(0).toUpperCase() + msg.tags.slice(1) + ':'"></span> <span x-text="msg.text"></span></div></div></template>"##,
            ))
        }
        (content)
        (PreEscaped("</main>"))
    }
}

pub fn layout_topbar(opts: LayoutTopbar) -> Markup {
    match opts.x_data {
        Some(xd) => html! {
            (PreEscaped(format!(
                r#"<div class="h-screen flex flex-col overflow-hidden" x-data="{}">"#,
                xd.replace('"', "&quot;")
            )))
            (topbar_chrome(&opts))
            (PreEscaped("</div>"))
        },
        None => html! {
            div class="h-screen flex flex-col overflow-hidden" {
                (topbar_chrome(&opts))
            }
        },
    }
}

fn topbar_chrome(opts: &LayoutTopbar) -> Markup {
    html! {
        div class="navbar bg-base-100 border-b border-base-300 px-4 flex justify-between items-center flex-none" {
            div class="flex-1" {}
            div class="flex-none flex items-center gap-2" {
                (opts.topbar_items)
            }
        }
        @if opts.has_sidebar {
            div class="flex-1 flex overflow-hidden relative" {
                div class="flex-1 overflow-hidden" { (opts.content) }
                (opts.right_panels)
            }
        } @else {
            div class="flex-1 overflow-hidden" { (opts.content) }
        }
    }
}
