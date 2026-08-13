//! Maud templates for the XLSX export page and sidebar menu.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        ButtonSubmit, Crumb, FormOpts, LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold,
        SidebarMenu, SidebarNavLink, SlotCapability, SlotRegistrar, breadcrumbs, button_submit,
        form, form_post_download_route, layout_main, layout_sidebar, shell_scaffold, sidebar_menu,
        sidebar_nav_items_pane,
    },
    http::ProvideRequestCaps,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

use super::routes::{ExportDownloadRouteTag, ExportPageRouteTag};

define_register_items! {
    plugin: ExportPluginTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ExportPageIdx: ExportPageTag => ExportPage,
    ]
}

define_register_items! {
    plugin: ExportPluginTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn export_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Export",
        href: None,
    }])
}

fn export_menu(current_path: &str) -> Markup {
    let export_url = ExportPageRouteTag.url();
    let links = [SidebarNavLink {
        key: "export",
        title: "XLSX Export",
        url: &export_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "Export",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

#[derive(Generic)]
pub struct ExportPage {
    pub tables: Vec<String>,
    pub deps_json: String,
    pub model_count: i64,
}

impl ExportPage {
    fn picker_body(&self) -> Markup {
        let catalog_note = format!("{} models available for export", self.model_count);
        html! {
            div class="container max-w-4xl mx-auto" x-data=(export_picker_xdata(&self.deps_json)) {
                h1 class="text-2xl font-bold mb-2" { "Export Data" }
                p class="text-sm text-base-content/70 mb-4" { (catalog_note) }
                (form(FormOpts {
                    attrs: form_post_download_route(ExportDownloadRouteTag),
                    inputs: html! {
                        div class="overflow-x-auto" {
                            table class="table table-sm" {
                                thead {
                                    tr {
                                        th { "Export" }
                                        th { "Table" }
                                        th { "Details" }
                                    }
                                }
                                tbody {
                                    @for table in &self.tables {
                                        tr {
                                            td {
                                                (PreEscaped(format!(
                                                    r#"<input type="checkbox" name="models" value="{table}" x-bind:checked="isChecked('{table}')" x-bind:disabled="isAuto('{table}')" @change="toggleRoot('{table}', $event.target.checked)">"#,
                                                )))
                                            }
                                            td { (table) }
                                            td class="text-xs text-base-content/60" { "columns" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    actions: html! {
                        div class="flex gap-2 mt-4" {
                            (button_submit(ButtonSubmit { label: "Download XLSX", ..Default::default() }))
                            (PreEscaped(r##"<button type="button" class="btn btn-ghost" @click="clearAll()">Clear All</button>"##))
                        }
                    },
                    ..Default::default()
                }))
            }
        }
    }
}

fn export_picker_xdata(deps_json: &str) -> String {
    format!(
        r#"{{
        deps: {deps_json},
        selectedRoots: [],
        effective: [],
        init() {{ this.recompute(); }},
        toggleRoot(table, checked) {{
            if (checked) {{
                if (!this.selectedRoots.includes(table)) this.selectedRoots.push(table);
            }} else {{
                this.selectedRoots = this.selectedRoots.filter((item) => item !== table);
            }}
            this.recompute();
        }},
        recompute() {{
            const effective = new Set(this.selectedRoots);
            let changed = true;
            while (changed) {{
                changed = false;
                for (const table of Array.from(effective)) {{
                    for (const dep of (this.deps[table] || [])) {{
                        if (!effective.has(dep)) {{
                            effective.add(dep);
                            changed = true;
                        }}
                    }}
                }}
            }}
            this.effective = Array.from(effective).sort();
        }},
        isChecked(table) {{ return this.effective.includes(table); }},
        isAuto(table) {{ return this.isChecked(table) && !this.selectedRoots.includes(table); }},
        clearAll() {{ this.selectedRoots = []; this.recompute(); }}
    }}"#
    )
}

impl crate::template::RenderAppPane for ExportPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_sidebar(LayoutSidebar {
            sidebar: export_menu(&ExportPageRouteTag.url()),
            breadcrumbs: export_crumbs(),
            content: self.picker_body(),
        })
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: export_crumbs(),
            content: self.picker_body(),
        })
    }
}

impl RenderTemplate for ExportPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_scaffold(ShellScaffold {
            title: "Export — Lariv",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar: export_menu(&ExportPageRouteTag.url()),
            breadcrumbs: export_crumbs(),
            body: self.picker_body(),
            ..Default::default()
        })
    }
}
