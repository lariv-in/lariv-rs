//! Maud templates for website route admin and builder pages.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        ButtonClear, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, FieldText,
        FieldTitle, FormOpts, LayoutMain, LayoutSidebar, ManyToManyItem, Modal, ObjectList,
        PaginationPage, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem, SidebarNavLink,
        SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader,
        TablePagination, TableRow, breadcrumbs, button_clear, button_modal_form, button_submit,
        column_sort_url, container_column, container_row, data_table_list_refresh,
        delete_confirmation, detail, field_text, field_title, form, form_hx_get_route,
        form_hx_post_route, form_hx_post_url, label_inline_with_classes, layout_main,
        layout_sidebar, modal, modal_keyed, pagination_pages, row_attr_navigate_route,
        shell_scaffold, sidebar_menu, sidebar_menu_item_pane, sidebar_nav_items_pane,
        sort_indicator, table_button_filter, table_pagination,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    plugins::website::forms::{
        PageSource, PageSourceField, RouteCreateForm, RouteCreateFormField, RouteEditForm,
        RouteEditFormField, RoutePathFilterForm, RoutePathFilterFormField,
    },
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_url, modal_edit_post_url},
};

use super::keys::{RouteCreateModalKey, RouteDeleteModalKey, RouteEditModalKey, RoutesTableKey};
use super::routes::{
    WebsiteBuilderRouteTag, WebsiteRoutesCreateGetRouteTag, WebsiteRoutesCreatePostRouteTag,
    WebsiteRoutesDeleteGetRouteTag, WebsiteRoutesDeletePostRouteTag, WebsiteRoutesDetailRouteTag,
    WebsiteRoutesEditGetRouteTag, WebsiteRoutesEditPostRouteTag, WebsiteRoutesListRouteTag,
};

define_register_items! {
    plugin: WebsiteTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        RouteListIdx: RouteListPageTag => RouteListPage,
        RouteDetailIdx: RouteDetailPageTag => RouteDetailPage,
        RouteEditModalIdx: RouteEditModalPageTag => RouteEditModalPage,
        RouteCreateModalIdx: RouteCreateModalPageTag => RouteCreateModalPage,
        ConfirmDeleteIdx: WebsiteConfirmDeletePageTag => ConfirmDeletePage,
        BuilderIdx: RoutesBuilderPageTag => RoutesBuilderPage,
    ]
}

define_register_items! {
    plugin: WebsiteTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn app_scaffold(
    _title: &str,
    chrome: &ShellChrome,
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn website_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Website",
        href: None,
    }])
}

fn website_route_crumbs(id: i64, path: &str, action: Option<&str>) -> Markup {
    let list_url = WebsiteRoutesListRouteTag.url();
    let detail_url = WebsiteRoutesDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Website",
                href: Some(&list_url),
            },
            Crumb {
                label: "Routes",
                href: Some(&list_url),
            },
            Crumb {
                label: path,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Website",
                href: Some(&list_url),
            },
            Crumb {
                label: "Routes",
                href: Some(&list_url),
            },
            Crumb {
                label: path,
                href: Some(&detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

fn routes_menu(current_path: &str) -> Markup {
    let routes_url = WebsiteRoutesListRouteTag.url();
    let links = [SidebarNavLink {
        key: "routes",
        title: "All Routes",
        url: &routes_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "Website Admin",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

fn route_detail_menu(id: i64, path: &str, active: &str) -> Markup {
    let title = format!("Route: {path}");
    let detail_url = WebsiteRoutesDetailRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Route Details",
                url: &detail_url,
                active: active == "detail",
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
    pub sort: String,
    pub path_and_query: String,
}

impl RouteListPage {
    pub fn render_table(&self) -> Markup {
        let path_sort = column_sort_url(&self.path_and_query, "Path", &self.sort);
        let active_sort = column_sort_url(&self.path_and_query, "IsActive", &self.sort);
        let path_label = format!("Path{}", sort_indicator(&self.sort, "Path"));
        let active_label = format!("Is Active{}", sort_indicator(&self.sort, "IsActive"));
        let headers = [
            TableColumnHeader {
                key: "Path",
                label: &path_label,
                sort_url: Some(&path_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "TemplateNode",
                label: "Template Node",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "IsActive",
                label: &active_label,
                sort_url: Some(&active_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .routes
            .items
            .iter()
            .map(|r| TableRow {
                attrs: row_attr_navigate_route(WebsiteRoutesDetailRouteTag::new(r.id)),
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
                    attrs: form_hx_get_route::<RoutesTableKey, WebsiteRoutesListRouteTag>(WebsiteRoutesListRouteTag),
                    inputs: RoutePathFilterForm::render_inputs(
                        &FormCtx::form::<RoutePathFilterForm>()
                            .value(RoutePathFilterFormField::Path, self.filter_path.as_str()),
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
            (button_modal_form(ButtonModalForm {
                name: "p_website.RouteCreateForm",
                href: &WebsiteRoutesCreateGetRouteTag.url(),
                form_post_url: &WebsiteRoutesCreateGetRouteTag.path(),
                modal_uid: RouteCreateModalKey::ID,
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
        data_table_list_refresh::<RoutesTableKey>(
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for RouteListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Website",
            chrome,
            routes_menu(&self.path_and_query),
            website_list_crumbs(),
            self.render_table(),
        )
    }
}

impl RenderAppPane for RouteListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            routes_menu(&self.path_and_query),
            website_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(website_list_crumbs(), self.render_table())
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
        let builder_url = WebsiteBuilderRouteTag::new(self.id).url();
        let edit_get = WebsiteRoutesEditGetRouteTag::new(self.id).url();
        let edit_post = WebsiteRoutesEditPostRouteTag::new(self.id).path();
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
                (container_row("flex flex-wrap gap-2 mt-4", html! {
                    (button_modal_form(ButtonModalForm {
                        name: "p_website.RouteEditForm",
                        href: &edit_get,
                        form_post_url: &edit_post,
                        modal_uid: RouteEditModalKey::ID,
                        label: "Edit",
                        classes: "btn-outline",
                        ..Default::default()
                    }))
                    @if self.editable {
                        a class="btn btn-outline btn-sm inline-flex" href=(builder_url) hx-boost="false" {
                            "Edit with GrapesJS"
                        }
                    }
                }))
            }))
        })
    }
}

impl RenderTemplate for RouteDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Route",
            chrome,
            route_detail_menu(self.id, &self.path, "detail"),
            website_route_crumbs(self.id, &self.path, None),
            self.body(),
        )
    }
}

impl RenderAppPane for RouteDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            route_detail_menu(self.id, &self.path, "detail"),
            website_route_crumbs(self.id, &self.path, None),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(website_route_crumbs(self.id, &self.path, None), self.body())
    }
}

/// Edit route modal. Create uses [`RouteCreateModalPage`].
#[derive(Generic)]
pub struct RouteEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub path: String,
    pub page_id: Option<i64>,
    pub page_name: String,
    pub is_active: bool,
    pub theme: String,
    pub theme_choices: Vec<(String, String)>,
    pub references: Vec<ManyToManyItem>,
    pub error_path: Option<String>,
    pub error_page: Option<String>,
}

impl RenderTemplate for RouteEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let page_id_s = self
            .page_id
            .filter(|i| *i > 0)
            .map(|i| i.to_string())
            .unwrap_or_default();
        let delete_url = WebsiteRoutesDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::form::<RouteEditForm>()
            .value(RouteEditFormField::Path, self.path.as_str())
            .error(RouteEditFormField::Path, self.error_path.as_deref())
            .value(RouteEditFormField::PageId, page_id_s.as_str())
            .display(RouteEditFormField::PageId, self.page_name.as_str())
            .error(RouteEditFormField::PageId, self.error_page.as_deref())
            .m2m(RouteEditFormField::References, &self.references)
            .checked(RouteEditFormField::IsActive, self.is_active)
            .value(RouteEditFormField::Theme, self.theme.as_str())
            .choices(RouteEditFormField::Theme, &self.theme_choices);
        modal_keyed::<RouteEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit route" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<RouteEditModalKey>(&modal_edit_post_url(
                        WebsiteRoutesEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    inputs: RouteEditForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_website.RoutesDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: RouteDeleteModalKey::ID,
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct RouteCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub path: String,
    pub page_id: Option<i64>,
    pub page_name: String,
    pub is_active: bool,
    pub theme: String,
    pub theme_choices: Vec<(String, String)>,
    pub references: Vec<ManyToManyItem>,
    pub error_path: Option<String>,
    pub error_page: Option<String>,
    pub error_name: Option<String>,
}

impl RouteCreateModalPage {
    fn inputs(&self) -> Markup {
        let page_id_s = self
            .page_id
            .filter(|i| *i > 0)
            .map(|i| i.to_string())
            .unwrap_or_default();
        let ctx = FormCtx::form::<RouteCreateForm>()
            .value(RouteCreateFormField::Path, self.path.as_str())
            .error(RouteCreateFormField::Path, self.error_path.as_deref())
            .kind::<PageSource>("Existing")
            .value(PageSourceField::NewPageName, "")
            .error(PageSourceField::NewPageName, self.error_name.as_deref())
            .value(PageSourceField::PageId, page_id_s.as_str())
            .display(PageSourceField::PageId, self.page_name.as_str())
            .error(PageSourceField::PageId, self.error_page.as_deref())
            .m2m(RouteCreateFormField::References, &self.references)
            .checked(RouteCreateFormField::IsActive, self.is_active)
            .value(RouteCreateFormField::Theme, self.theme.as_str())
            .choices(RouteCreateFormField::Theme, &self.theme_choices);
        RouteCreateForm::render_inputs(&ctx)
    }

    fn form_error(&self) -> Option<&str> {
        self.error_path
            .as_deref()
            .or(self.error_page.as_deref())
            .or(self.error_name.as_deref())
    }
}

impl RenderTemplate for RouteCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_website.RouteCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<RouteCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create route",
                attrs: crate::components::swap::form_hx_post_for_url::<RouteCreateModalKey>(
                    &modal_create_post_url(
                        WebsiteRoutesCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    ),
                ),
                form_error: self.form_error(),
                inputs: self.inputs(),
                actions: html! {
                    (container_row("flex flex-wrap justify-end gap-2 mt-2 items-center", html! {
                        (button_submit(ButtonSubmit {
                            label: "Create Route",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                    }))
                },
                ..Default::default()
            }),
        )
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub id: i64,
    pub path: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal(Modal {
            uid: RouteDeleteModalKey::ID,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm deletion",
                message: "Are you sure you want to delete this route? This action cannot be undone.",
                attrs: form_hx_post_route::<RouteDeleteModalKey, WebsiteRoutesDeletePostRouteTag>(
                    WebsiteRoutesDeletePostRouteTag::new(self.id),
                ),
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
