//! Maud templates for the XLSX import page and sidebar menu.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    capability::define_register_items,
    components::{
        ButtonSubmit, Crumb, FormOpts, LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold,
        SidebarMenu, SidebarNavLink, SlotCapability, SlotRegistrar, breadcrumbs, button_submit,
        form, form_hx_post_main, layout_main, layout_sidebar, shell_scaffold, sidebar_menu,
        sidebar_nav_items_pane,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

use super::forms::ImportForm;
use super::routes::{ImportPageRouteTag, ImportPostRouteTag};
use super::upsert::ImportReport;

define_register_items! {
    plugin: ImportPluginTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ImportPageIdx: ImportPageTag => ImportPage,
    ]
}

define_register_items! {
    plugin: ImportPluginTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn import_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Import",
        href: None,
    }])
}

fn import_menu(current_path: &str) -> Markup {
    let import_url = ImportPageRouteTag.url();
    let links = [SidebarNavLink {
        key: "import",
        title: "XLSX Import",
        url: &import_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "Import",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

#[derive(Generic)]
pub struct ImportPage {
    pub error: String,
    pub result: Option<ImportReport>,
    pub model_count: i64,
}

impl ImportPage {
    fn body(&self) -> Markup {
        let catalog_note = format!("{} models registered for import", self.model_count);
        html! {
            div class="container max-w-4xl mx-auto" {
                h1 class="text-2xl font-bold mb-2" { "Import Data" }
                p class="text-sm text-base-content/70 mb-4" { (catalog_note) }
                @if let Some(report) = &self.result {
                    (report_body(report))
                }
                (form(FormOpts {
                    attrs: form_hx_post_main(ImportPostRouteTag)
                        .set("hx-encoding", "multipart/form-data"),
                    enctype: Some("multipart/form-data"),
                    form_error: if self.error.is_empty() {
                        None
                    } else {
                        Some(self.error.as_str())
                    },
                    inputs: ImportForm::render_inputs(&FormCtx::form::<ImportForm>()),
                    actions: html! {
                        div class="flex gap-2 mt-4" {
                            (button_submit(ButtonSubmit { label: "Import XLSX", ..Default::default() }))
                        }
                    },
                    ..Default::default()
                }))
            }
        }
    }
}

fn report_body(report: &ImportReport) -> Markup {
    html! {
        div class="mb-6" {
            h2 class="text-lg font-semibold mb-2" { "Import complete" }
            @if !report.skipped_sheets.is_empty() {
                p class="text-sm text-base-content/70 mb-2" {
                    "Skipped unregistered sheets: "
                    (report.skipped_sheets.join(", "))
                }
            }
            div class="overflow-x-auto" {
                table class="table table-sm" {
                    thead {
                        tr {
                            th { "Table" }
                            th { "Inserted" }
                            th { "Updated" }
                        }
                    }
                    tbody {
                        @for table in &report.tables {
                            tr {
                                td { (table.table) }
                                td { (table.inserted) }
                                td { (table.updated) }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl crate::template::RenderAppPane for ImportPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_sidebar(LayoutSidebar {
            sidebar: import_menu(&ImportPageRouteTag.url()),
            breadcrumbs: import_crumbs(),
            content: self.body(),
        })
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: import_crumbs(),
            content: self.body(),
        })
    }
}

impl RenderTemplate for ImportPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_scaffold(ShellScaffold {
            title: "Import — Lariv",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar: import_menu(&ImportPageRouteTag.url()),
            breadcrumbs: import_crumbs(),
            body: self.body(),
            ..Default::default()
        })
    }
}
