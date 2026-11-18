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
use crate::routes::{
    GstExcelRouteTag, GstExcelView, GstExportView, GstPreferencesPostRouteTag,
    GstPreferencesRouteTag, GstWorkbookRouteTag,
};

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
        GstExcelIdx: GstExcelPageTag => GstExcelPage,
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

fn role_allowed<Tag: 'static>() -> bool {
    use lariv_plugin_users::role_authorization::{current_auth, principal_allowed, roles_for};
    current_auth().is_some_and(|auth| principal_allowed(&auth, &roles_for::<Tag>()))
}

fn menu(current: &str) -> Markup {
    let preferences = GstPreferencesRouteTag.url();
    let excel = GstExcelRouteTag.url();
    let preference_link = role_allowed::<GstExportView>().then_some(SidebarNavLink {
        key: "preferences",
        title: "Preferences",
        url: &preferences,
        icon_name: Some("adjustments-horizontal"),
        match_prefixes: &[],
    });
    let excel_link = role_allowed::<GstExcelView>().then_some(SidebarNavLink {
        key: "excel",
        title: "Excel",
        url: &excel,
        icon_name: Some("document-arrow-down"),
        match_prefixes: &[],
    });
    let links: Vec<SidebarNavLink<'_>> = [preference_link, excel_link]
        .into_iter()
        .flatten()
        .collect();
    sidebar_menu(SidebarMenu {
        title: "GST export",
        children: sidebar_nav_items_pane(&links, current),
    })
}

fn preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Preferences",
        href: None,
    }])
}

fn excel_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Excel",
        href: None,
    }])
}

impl RenderAppPane for GstPreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_sidebar(LayoutSidebar {
            sidebar: menu(&GstPreferencesRouteTag.url()),
            breadcrumbs: preferences_crumbs(),
            content: self.body(),
        })
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: preferences_crumbs(),
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
            sidebar: menu(&GstPreferencesRouteTag.url()),
            breadcrumbs: preferences_crumbs(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct GstExcelPage;

impl GstExcelPage {
    fn body(&self) -> Markup {
        html! {
            div class="container max-w-4xl mx-auto" {
                h1 class="text-2xl font-bold mb-2" { "GST Excel" }
                p class="text-sm text-base-content/70 mb-4" {
                    "Download posted invoices as an Excel workbook for GST filing."
                }
                a class="btn btn-outline" href=(GstWorkbookRouteTag.path()) hx-boost="false" {
                    "Download GST Excel"
                }
            }
        }
    }
}

impl RenderAppPane for GstExcelPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_sidebar(LayoutSidebar {
            sidebar: menu(&GstExcelRouteTag.url()),
            breadcrumbs: excel_crumbs(),
            content: self.body(),
        })
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: excel_crumbs(),
            content: self.body(),
        })
    }
}

impl RenderTemplate for GstExcelPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_scaffold(ShellScaffold {
            title: "GST Excel — Lariv",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar: menu(&GstExcelRouteTag.url()),
            breadcrumbs: excel_crumbs(),
            body: self.body(),
            ..Default::default()
        })
    }
}
