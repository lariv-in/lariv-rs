use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        ButtonClear, ButtonLink, ButtonModalForm, ButtonSubmit, DeleteConfirmation, FieldText,
        FieldTitle, FormOpts, LayoutSidebar, ManyToManyItem, Modal, ObjectList,
        PaginationPage, RegisterSlots, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuBack,
        SidebarMenuItem, SlotCapability, SwapKey, TableButtonFilter, TableColumnHeader,
        TablePagination, TableRow, button_clear, button_link, button_modal_form, button_submit,
        container_column, container_row, data_table_list, delete_confirmation, detail, field_text,
        field_title, form, form_hx_get, form_hx_post_main, form_hx_post_selector,
        label_inline_with_classes, layout_sidebar, modal, pagination_pages, row_attr_navigate,
        shell_scaffold, sidebar_menu, sidebar_menu_item, table_button_filter, table_pagination,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    plugins::website::forms::{RouteCreateForm, RouteEditForm, RoutePathFilterForm},
    template::{RegisterTemplates, RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf},
};

use super::WebsiteTag;
use super::keys::{RouteDeleteModalKey, RoutesTableKey};

define_register_items! {
    plugin: WebsiteTag;
    capability: TemplateCapability;
    trait: RegisterTemplates;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    items: [
        RouteListIdx: RouteListPageTag => RouteListPage,
        RouteDetailIdx: RouteDetailPageTag => RouteDetailPage,
        RouteFormIdx: RouteFormPageTag => RouteFormPage,
        ConfirmDeleteIdx: WebsiteConfirmDeletePageTag => ConfirmDeletePage,
        BuilderIdx: RoutesBuilderPageTag => RoutesBuilderPage,
    ]
}

define_register_items! {
    plugin: WebsiteTag;
    capability: SlotCapability;
    trait: RegisterSlots;
    method: register_slots;
    bounds: [];
    items: [];
}

fn app_scaffold(_title: &str, chrome: &ShellChrome, sidebar: Markup, body: Markup) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(sidebar: Markup, body: Markup) -> Markup {
    layout_sidebar(LayoutSidebar {
        sidebar,
        content: body,
    })
}

fn scaffold_main(body: Markup) -> Markup {
    use crate::components::layout::layout_main;
    layout_main(body)
}

fn routes_menu() -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Website Admin",
        back: Some(SidebarMenuBack {
            title: "Back to All Apps",
            url: "/dashboard/",
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "All Routes",
                url: "/website/",
                ..Default::default()
            }))
        },
    })
}

fn route_detail_menu(id: i64, path: &str) -> Markup {
    let title = format!("Route: {path}");
    let detail_url = format!("/website/{id}/");
    let edit_url = format!("/website/{id}/edit/");
    sidebar_menu(SidebarMenu {
        title: &title,
        back: Some(SidebarMenuBack {
            title: "Back to All Routes",
            url: "/website/",
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "Route Details",
                url: &detail_url,
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Edit Route",
                url: &edit_url,
                ..Default::default()
            }))
        },
    })
}

fn render_pagination<K: SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
    push_url: bool,
) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, push_url);
    let pages: Vec<PaginationPage<'_>> = owned
        .iter()
        .map(|(ellipsis, url, push_url, active, label)| PaginationPage {
            ellipsis: *ellipsis,
            url: url.as_str(),
            push_url: *push_url,
            active: *active,
            label: label.as_str(),
        })
        .collect();
    table_pagination(TablePagination {
        pages: &pages,
        hx_target: K::SELECTOR,
    })
}

#[derive(Clone, Debug)]
pub struct RouteRow {
    pub id: i64,
    pub path: String,
    pub page_name: String,
    pub is_active: bool,
}

#[derive(Generic)]
pub struct RouteListPage {
    pub routes: ObjectList<RouteRow>,
    pub filter_path: String,
    pub path_and_query: String,
}

impl RouteListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                label: "Path",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                label: "Template Node",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                label: "Is Active",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .routes
            .items
            .iter()
            .map(|r| TableRow {
                attrs: row_attr_navigate(&format!("/website/{}/", r.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &r.path,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &r.page_name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: if r.is_active { "yes" } else { "no" },
                        classes: "",
                    }),
                ],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get::<RoutesTableKey>("/website/"),
                    inputs: RoutePathFilterForm::render_inputs(
                        &FormCtx::new().value("Path", self.filter_path.as_str()),
                    ),
                    actions: html! {
                        (container_row("flex gap-2", html! {
                            (button_submit(ButtonSubmit { label: "Apply Filters", ..Default::default() }))
                            (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
                        }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
            (button_link(ButtonLink {
                href: "/website/create/",
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<RoutesTableKey>(
            &self.path_and_query,
            self.routes.number,
            self.routes.num_pages,
            true,
        );
        data_table_list::<RoutesTableKey>("", actions, &headers, &rows, pagination)
    }
}

impl RenderTemplate for RouteListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Website", chrome, routes_menu(), self.render_table())
    }
}

impl RenderAppPane for RouteListPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(routes_menu(), self.render_table())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.render_table())
    }
}

#[derive(Generic)]
pub struct RouteDetailPage {
    pub id: i64,
    pub path: String,
    pub page_name: String,
    pub page_id: i64,
    pub is_active: bool,
    pub theme_label: String,
    pub references: Vec<ManyToManyItem>,
    pub editable: bool,
}

impl RouteDetailPage {
    fn body(&self) -> Markup {
        let builder_url = format!("/website/{}/builder/", self.id);
        detail(html! {
            (container_column("", html! {
                (field_title(FieldTitle { value: &self.path, classes: "" }))
                a class="link link-primary font-semibold mb-4 block" href=(self.path.as_str()) hx-boost="false" {
                    "View Live Page ↗"
                }
                (label_inline_with_classes("Template Page Name", "mt-4 block", html! {
                    (field_text(FieldText { value: &self.page_name, classes: "" }))
                }))
                (label_inline_with_classes("Reference Files", "mt-4 block", html! {
                    @for r in &self.references {
                        span class="badge badge-outline mr-1" { (r.value) }
                    }
                }))
                (label_inline_with_classes("Is Active", "mt-4 block", html! {
                    (if self.is_active { "yes" } else { "no" })
                }))
                (label_inline_with_classes("Theme", "mt-4 block", html! {
                    (field_text(FieldText { value: &self.theme_label, classes: "" }))
                }))
                @if self.editable {
                    a class="btn btn-outline btn-sm mt-2 inline-flex" href=(builder_url) hx-boost="false" {
                        "Edit with GrapesJS"
                    }
                }
            }))
        })
    }
}

impl RenderTemplate for RouteDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Route",
            chrome,
            route_detail_menu(self.id, &self.path),
            self.body(),
        )
    }
}

impl RenderAppPane for RouteDetailPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(route_detail_menu(self.id, &self.path), self.body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.body())
    }
}

#[derive(Generic)]
pub struct RouteFormPage {
    pub id: Option<i64>,
    pub path: String,
    pub page_id: Option<i64>,
    pub page_name: String,
    pub is_active: bool,
    pub theme: String,
    pub theme_choices: Vec<(String, String)>,
    pub references: Vec<ManyToManyItem>,
    pub allow_create_new: bool,
    pub error_path: Option<String>,
    pub error_page: Option<String>,
    pub error_name: Option<String>,
}

impl RouteFormPage {
    fn body(&self) -> Markup {
        let action = match self.id {
            Some(id) => format!("/website/{id}/edit/"),
            None => "/website/create/".into(),
        };
        let page_id_s = self
            .page_id
            .filter(|i| *i > 0)
            .map(|i| i.to_string())
            .unwrap_or_default();
        let delete_url = self.id.map(|id| format!("/website/{id}/delete/"));
        let inputs = if self.allow_create_new {
            let ctx = FormCtx::new()
                .value("Path", self.path.as_str())
                .error("Path", self.error_path.as_deref())
                .kind("Kind", "Existing")
                .value("NewPageName", "")
                .error("name", self.error_name.as_deref())
                .value("PageID", page_id_s.as_str())
                .display("page_name", self.page_name.as_str())
                .error("page", self.error_page.as_deref())
                .m2m("References", &self.references)
                .checked("IsActive", self.is_active)
                .value("Theme", self.theme.as_str())
                .choices("theme", &self.theme_choices);
            RouteCreateForm::render_inputs(&ctx)
        } else {
            let ctx = FormCtx::new()
                .value("Path", self.path.as_str())
                .error("Path", self.error_path.as_deref())
                .value("PageID", page_id_s.as_str())
                .display("page_name", self.page_name.as_str())
                .error("page", self.error_page.as_deref())
                .m2m("References", &self.references)
                .checked("IsActive", self.is_active)
                .value("Theme", self.theme.as_str())
                .choices("theme", &self.theme_choices);
            RouteEditForm::render_inputs(&ctx)
        };
        form(FormOpts {
            title: if self.id.is_some() {
                "Edit route"
            } else {
                "Create route"
            },
            attrs: form_hx_post_main(&action),
            inputs,
            actions: html! {
                (container_row("flex flex-wrap justify-between gap-2 mt-2 items-center", html! {
                    (button_submit(ButtonSubmit {
                        label: if self.id.is_some() { "Save Changes" } else { "Create Route" },
                        ..Default::default()
                    }))
                    @if let Some(url) = &delete_url {
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_website.RoutesDeleteForm",
                            href: url,
                            form_post_url: url,
                            modal_uid: RouteDeleteModalKey::ID,
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    }
                }))
            },
            ..Default::default()
        })
    }

    fn sidebar(&self) -> Markup {
        match self.id {
            Some(id) => route_detail_menu(id, &self.path),
            None => routes_menu(),
        }
    }
}

impl RenderTemplate for RouteFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Route form", chrome, self.sidebar(), self.body())
    }
}

impl RenderAppPane for RouteFormPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(self.sidebar(), self.body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.body())
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub id: i64,
    pub path: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let action = format!("/website/{}/delete/", self.id);
        let target = format!("#{}", RouteDeleteModalKey::ID);
        modal(Modal {
            uid: RouteDeleteModalKey::ID,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm deletion",
                message: "Are you sure you want to delete this route? This action cannot be undone.",
                attrs: form_hx_post_selector(&action, &target),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

/// Full-page GrapesJS builder (no app chrome).
#[derive(Generic)]
pub struct RoutesBuilderPage {
    pub head_html: String,
    pub body_html: String,
}

impl RenderTemplate for RoutesBuilderPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        html! {
            (PreEscaped("<!DOCTYPE html><html><head>"))
            (PreEscaped(self.head_html.as_str()))
            (PreEscaped("</head><body>"))
            (PreEscaped(self.body_html.as_str()))
            (PreEscaped("</body></html>"))
        }
    }
}
