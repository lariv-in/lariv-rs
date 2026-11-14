use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, Crumb, CsrfToken, FormOpts, HtmlAttrs, LayoutMain, LayoutSidebar, ManyToManyItem,
    ShellChrome, ShellScaffold, SidebarMenu, SidebarNavLink, SlotCapability, SlotRegistrar,
    breadcrumbs, button_submit, form, layout_main, layout_sidebar, shell_scaffold, sidebar_menu,
    sidebar_nav_items_pane,
};
use lariv_core::html_form::{FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};

use crate::forms::{GstPreferencesForm, GstPreferencesFormField};
use crate::routes::{GstPreferencesPostRouteTag, GstPreferencesRouteTag};

lariv_core::define_register_items! {
    plugin: crate::FinanceGstExportTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        GstPreferencesIdx: GstPreferencesPageTag => GstPreferencesPage,
    ]
}

lariv_core::define_register_items! {
    plugin: crate::FinanceGstExportTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

#[derive(Generic)]
pub struct GstPreferencesPage {
    pub igst: Vec<ManyToManyItem>,
    pub cgst: Vec<ManyToManyItem>,
    pub sgst: Vec<ManyToManyItem>,
    pub cess: Vec<ManyToManyItem>,
    pub error: String,
}

impl GstPreferencesPage {
    fn body(&self) -> Markup {
        html! {
            div class="container max-w-4xl mx-auto" {
                h1 class="text-2xl font-bold mb-2" { "GST export" }
                p class="text-sm text-base-content/70 mb-4" {
                    "Choose the taxes that make up each GST component."
                }
                @if !self.error.is_empty() {
                    p class="text-error mb-4" { (self.error) }
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: HtmlAttrs::new()
                        .set("method", "POST")
                        .set("action", GstPreferencesPostRouteTag.path())
                        .set("hx-boost", "false"),
                    inputs: GstPreferencesForm::render_inputs(
                        &FormCtx::form::<GstPreferencesForm>(CsrfToken::current())
                            .m2m(GstPreferencesFormField::Igst, &self.igst)
                            .m2m(GstPreferencesFormField::Cgst, &self.cgst)
                            .m2m(GstPreferencesFormField::Sgst, &self.sgst)
                            .m2m(GstPreferencesFormField::Cess, &self.cess),
                    ),
                    actions: html! {
                        div class="mt-4" {
                            (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        }
                    },
                    ..Default::default()
                }))
            }
        }
    }
}

fn menu() -> Markup {
    let current = GstPreferencesRouteTag.url();
    let links = [SidebarNavLink {
        key: "preferences",
        title: "Preferences",
        url: &current,
        icon_name: Some("adjustments-horizontal"),
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "GST export",
        children: sidebar_nav_items_pane(&links, &current),
    })
}

fn crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "GST export",
        href: None,
    }])
}

impl RenderAppPane for GstPreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_sidebar(LayoutSidebar {
            sidebar: menu(),
            breadcrumbs: crumbs(),
            content: self.body(),
        })
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: crumbs(),
            content: self.body(),
        })
    }
}

impl RenderTemplate for GstPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_scaffold(ShellScaffold {
            title: "GST export — Lariv",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar: menu(),
            breadcrumbs: crumbs(),
            body: self.body(),
            ..Default::default()
        })
    }
}
