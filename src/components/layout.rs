//! Page layout containers.

use maud::{Markup, PreEscaped, html};

use crate::components::swap::{MainContentKey, SwapKey};
use crate::components::text::icon;

/// HTMX fragment for `<main id="main-content">` swaps (sidebar menu navigation).
///
/// Construct only via [`layout_main`].
#[derive(Debug, Clone)]
pub struct MainContentHtml(Markup);

impl MainContentHtml {
    pub fn into_markup(self) -> Markup {
        self.0
    }
}

impl From<MainContentHtml> for Markup {
    fn from(value: MainContentHtml) -> Self {
        value.into_markup()
    }
}

/// HTMX fragment for `#app-layout` swaps (dashboard tiles, form POST, boosted nav).
///
/// Construct via [`layout_sidebar`] or [`app_layout_pane`].
#[derive(Debug, Clone)]
pub struct AppLayoutHtml(Markup);

impl AppLayoutHtml {
    pub fn into_markup(self) -> Markup {
        self.0
    }
}

impl From<AppLayoutHtml> for Markup {
    fn from(value: AppLayoutHtml) -> Self {
        value.into_markup()
    }
}

pub struct LayoutCard;
pub struct LayoutSimple;
pub struct LayoutSidebar {
    pub sidebar: Markup,
    /// Trail rendered after the sidebar toggle in [`layout_main`].
    pub breadcrumbs: Markup,
    pub content: Markup,
}

/// Arguments for [`layout_main`] (`#main-content` column chrome + body).
pub struct LayoutMain {
    /// Trail rendered after the sidebar toggle. Empty ⇒ toggle only.
    pub breadcrumbs: Markup,
    pub content: Markup,
}
pub struct LayoutTopbar {
    pub topbar_items: Markup,
    pub content: Markup,
    pub has_sidebar: bool,
    pub x_data: Option<&'static str>,
    pub right_panels: Markup,
}

/// Alpine state for the collapsible left sidebar drawer.
pub const LEFT_SIDEBAR_X_DATA: &str = r#"{
    showLeft: window.innerWidth >= 768,
    isMobile: window.innerWidth < 768,
    messages: [],
    init() {
        const mq = window.matchMedia('(max-width: 767px)');
        const onChange = () => {
            this.isMobile = mq.matches;
            this.showLeft = !this.isMobile;
        };
        mq.addEventListener('change', onChange);
        this.$el.addEventListener('htmx:after-swap', (event) => {
            const target = event.detail?.target;
            if (this.isMobile && target && target.id === 'main-content') {
                this.showLeft = false;
            }
        });
    },
    toggleLeft() {
        this.showLeft = !this.showLeft;
    },
    closeLeft() {
        if (this.isMobile) this.showLeft = false;
    }
}"#;

/// Alpine state for the collapsible right drawer.
pub const RIGHT_SIDEBAR_X_DATA: &str = r#"{
    showRight: $persist(true).as('right-sidebar-show'),
    rightSidebarWidth: $persist(320).as('right-sidebar-width'),
    isResizing: false,
    toggleRight() {
        this.showRight = !this.showRight;
    },
    startResize(e) {
        e.preventDefault();
        this.isResizing = true;
        const startWidth = this.rightSidebarWidth;
        const startX = e.clientX;
        const onMouseMove = (moveEvent) => {
            if (!this.isResizing) return;
            const deltaX = moveEvent.clientX - startX;
            let newWidth = startWidth - deltaX;
            if (newWidth < 240) newWidth = 240;
            if (newWidth > 600) newWidth = 600;
            this.rightSidebarWidth = newWidth;
        };
        const onMouseUp = () => {
            this.isResizing = false;
            document.removeEventListener('mousemove', onMouseMove);
            document.removeEventListener('mouseup', onMouseUp);
        };
        document.addEventListener('mousemove', onMouseMove);
        document.addEventListener('mouseup', onMouseUp);
    }
}"#;

fn markup_has_content(m: &Markup) -> bool {
    !m.clone().into_string().trim().is_empty()
}

/// Build topbar layout, enabling the right drawer when `right_sidebar` is non-empty.
pub fn layout_topbar_with_right_sidebar(
    topbar_items: Markup,
    content: Markup,
    right_sidebar: Markup,
) -> Markup {
    let has_sidebar = markup_has_content(&right_sidebar);
    let topbar_items = if has_sidebar {
        html! {
            (right_sidebar_toggle_buttons())
            (topbar_items)
        }
    } else {
        topbar_items
    };
    layout_topbar(LayoutTopbar {
        topbar_items,
        content,
        has_sidebar,
        x_data: if has_sidebar {
            Some(RIGHT_SIDEBAR_X_DATA)
        } else {
            None
        },
        right_panels: if has_sidebar {
            right_sidebar_aside(right_sidebar)
        } else {
            Markup::default()
        },
    })
}

fn right_sidebar_toggle_buttons() -> Markup {
    html! {
        (PreEscaped(
            r##"<button type="button" class="btn btn-sm btn-square btn-ghost" @click="toggleRight()">"##,
        ))
        (PreEscaped(r##"<span x-show="!showRight">"##))
        (icon("bars-3-bottom-right", ""))
        (PreEscaped("</span>"))
        (PreEscaped(r##"<span x-show="showRight">"##))
        (icon("x-mark", ""))
        (PreEscaped("</span></button>"))
    }
}

fn right_sidebar_aside(panel: Markup) -> Markup {
    html! {
        (PreEscaped(
            r##"<div class="xl:hidden absolute inset-0 bg-neutral-900/40 z-30 transition-opacity" x-show="showRight" x-transition:enter="transition ease-out duration-200" x-transition:enter-start="opacity-0" x-transition:enter-end="opacity-100" x-transition:leave="transition ease-in duration-150" x-transition:leave-start="opacity-100" x-transition:leave-end="opacity-0" @click="toggleRight()"></div>"##,
        ))
        (PreEscaped(
            r##"<div class="hidden xl:flex w-2 -mx-1 cursor-col-resize flex-none h-full relative z-50 items-center justify-center hover:bg-primary/20 active:bg-primary/30 transition-all duration-150 group" x-show="showRight" @mousedown="startResize($event)" :class="isResizing ? 'bg-primary/20' : ''">"##,
        ))
        (PreEscaped(
            r##"<div class="w-[1px] h-full bg-base-300 group-hover:bg-primary group-active:bg-primary transition-colors duration-150" :class="isResizing ? 'bg-primary' : ''"></div></div>"##,
        ))
        (PreEscaped(
            r##"<aside class="flex-none bg-base-100 flex flex-col h-full overflow-hidden absolute right-0 top-0 z-40 border-l border-base-300 shadow-2xl max-w-[85vw] sm:max-w-[400px] xl:static xl:border-l-0 xl:shadow-none xl:max-w-none" x-show="showRight" x-transition:enter="transition ease-out duration-200 transform" x-transition:enter-start="translate-x-full" x-transition:enter-end="translate-x-0" x-transition:leave="transition ease-in duration-150 transform" x-transition:leave-start="translate-x-0" x-transition:leave-end="translate-x-full" :style="'width: ' + rightSidebarWidth + 'px'" style="width: 320px;">"##,
        ))
        div class="flex-1 overflow-hidden relative" {
            div class="h-full overflow-y-auto p-0" {
                (panel)
            }
        }
        (PreEscaped("</aside>"))
    }
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
                    // Same swap root as scaffold for auth ↔ app HTMX navigations.
                    (PreEscaped(format!(
                        r#"<div {}>"#,
                        crate::components::swap::app_layout_history_attrs()
                    )))
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
            (PreEscaped(format!(
                r#"<div {}>"#,
                crate::components::swap::app_layout_history_attrs()
            )))
            (children)
            (PreEscaped("</div>"))
        }
    }
}

pub fn layout_sidebar(opts: LayoutSidebar) -> AppLayoutHtml {
    let x_data = LEFT_SIDEBAR_X_DATA.replace('"', "&quot;");
    AppLayoutHtml(html! {
        (PreEscaped(format!(
            r##"<div {} class="size-full" x-data="{}">"##,
            crate::components::swap::app_layout_history_attrs(),
            x_data,
        )))
        (PreEscaped(r##"<div class="relative grid h-full transition-[grid-template-columns] duration-[400ms] ease-in" :class="isMobile ? 'grid-cols-1' : (showLeft ? 'grid-cols-[250px_1fr]' : 'grid-cols-[0px_1fr]')">"##))
        (PreEscaped(r##"<div x-show="isMobile && showLeft" x-transition.opacity="" @click="closeLeft()" class="fixed inset-x-0 bottom-0 top-16 bg-black/50 z-40"></div>"##))
        (PreEscaped(r##"<aside class="bg-base-100 border-r border-base-300 overflow-hidden max-md:fixed max-md:left-0 max-md:top-16 max-md:bottom-0 max-md:z-50 max-md:w-[250px] max-md:shadow-xl max-md:transition-transform max-md:duration-300" :class="isMobile && !showLeft ? 'max-md:-translate-x-full' : ''">"##))
        div class="h-full overflow-y-auto w-[250px] bg-base-100 p-2" {
            (opts.sidebar)
        }
        (PreEscaped("</aside>"))
        (layout_main(LayoutMain {
            breadcrumbs: opts.breadcrumbs,
            content: opts.content,
        })
        .0)
        (PreEscaped("</div></div>"))
    })
}

/// `#app-layout` pane without a left sidebar column (e.g. dashboard app grid).
pub fn app_layout_pane(content: Markup) -> AppLayoutHtml {
    AppLayoutHtml(html! {
        (PreEscaped(format!(
            r#"<div {} class="size-full overflow-y-auto p-4">"#,
            crate::components::swap::app_layout_history_attrs()
        )))
        (content)
        (PreEscaped("</div>"))
    })
}

/// `<main id="main-content">` column — sidebar menu swaps this, not `#app-layout`.
pub fn layout_main(opts: LayoutMain) -> MainContentHtml {
    let menu = icon("bars-3", "");
    MainContentHtml(html! {
        (PreEscaped(format!(
            r##"<main id="{}" class="overflow-y-auto p-4 relative h-full bg-base-100">"##,
            MainContentKey::ID
        )))
        div class="flex items-center gap-2 mb-2 min-w-0" {
            (PreEscaped(
                r##"<button type="button" @click="toggleLeft()" class="btn btn-sm btn-square shrink-0">"##,
            ))
            (menu)
            (PreEscaped("</button>"))
            (opts.breadcrumbs)
        }
        div class="messages mb-4" {
            (PreEscaped(
                r##"<template x-for="msg in messages"><div class="alert shadow-lg mb-2" :class="msg.tags == 'error' ? 'alert-error' : (msg.tags == 'success' ? 'alert-success' : 'alert-info')"><div class="flex-1"><span class="font-semibold" x-text="msg.tags.charAt(0).toUpperCase() + msg.tags.slice(1) + ':'"></span> <span x-text="msg.text"></span></div></div></template>"##,
            ))
        }
        (opts.content)
        (PreEscaped("</main>"))
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use maud::html;

    #[test]
    fn right_sidebar_enables_drawer() {
        let panel = html! { div { "history panel" } };
        let out = layout_topbar_with_right_sidebar(Markup::default(), html! { p { "main" } }, panel);
        let s = out.into_string();
        assert!(s.contains("showRight"));
        assert!(s.contains("rightSidebarWidth"));
        assert!(s.contains("history panel"));
    }

    #[test]
    fn empty_right_sidebar_disables_drawer() {
        let out =
            layout_topbar_with_right_sidebar(Markup::default(), html! { p { "main" } }, Markup::default());
        let s = out.into_string();
        assert!(!s.contains("showRight"));
    }

    #[test]
    fn left_sidebar_mobile_drawer_closes_on_nav() {
        let out = layout_sidebar(LayoutSidebar {
            sidebar: html! { nav { "menu" } },
            breadcrumbs: Markup::default(),
            content: html! { p { "body" } },
        });
        let s = out.into_markup().into_string();
        assert!(s.contains("closeLeft"));
        assert!(s.contains("toggleLeft"));
        assert!(s.contains("main-content"));
        assert!(s.contains("htmx:after-swap"));
        assert!(s.contains("max-md:fixed"));
        assert!(s.contains("max-md:-translate-x-full"));
        assert!(!s.contains("translate: none"));
    }

    #[test]
    fn layout_main_places_breadcrumbs_after_toggle() {
        let out = layout_main(LayoutMain {
            breadcrumbs: html! { div class="breadcrumbs" { "Blog" } },
            content: html! { p { "body" } },
        });
        let s = out.into_markup().into_string();
        let crumb_pos = s.find(r#"class="breadcrumbs""#).expect("crumbs");
        let toggle_pos = s.find("toggleLeft()").expect("toggle");
        assert!(toggle_pos < crumb_pos);
    }
}
