//! Filesystem plugin pages (Maud), go`.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonLink, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, FieldText,
        FieldTitle, FormOpts, LayoutMain, LayoutSidebar, ObjectList, PaginationPage,
        ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem, SidebarMenuModalForm,
        SidebarNavLink, SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter,
        TableColumnHeader, TablePagination, TableRow, breadcrumbs, button_link, button_modal_form,
        button_submit, column_sort_url, container_column, container_row, data_table_list,
        data_table_list_refresh, detail, field_text, field_title, form, form_hx_get_route,
        form_hx_post_main, form_hx_post_url, label_inline, layout_main, layout_sidebar,
        modal, modal_keyed, pagination_pages, row_attr_navigate_route, row_attr_select,
        shell_scaffold, sidebar_menu, sidebar_menu_item_pane, sidebar_menu_modal_form_item,
        sidebar_nav_items_pane, sort_indicator, table_button_filter, table_pagination,
    },
    capability::define_register_items,
    html_form::{FormCtx, HtmlForm},
    http::{ProvideRequestCaps},
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_url, modal_edit_post_url},
};

use super::forms::{
    MoveForm, MoveFormField, VNodeEditForm, VNodeEditFormField, VNodeEditFormFlag, VNodeForm,
    VNodeFormField, VNodeFormFlag, VNodeKind, VNodeKindField, VNodeMultiUploadForm,
    VNodeMultiUploadFormField, VNodeNameFilterForm, VNodeNameFilterFormField, VNodeZipUploadForm,
    VNodeZipUploadFormField,
};
use super::keys::{
    VNodeCreateModalKey, VNodeDeleteModalKey, VNodeEditModalKey, VNodeMultiUploadModalKey,
    VNodeSelectModalKey, VNodeSelectTableKey, VNodeTableKey, VNodeZipUploadModalKey,
};
use super::routes::{
    VNodeBrowseRouteTag, VNodeCreateGetInRouteTag, VNodeCreateGetRouteTag,
    VNodeCreatePostInRouteTag, VNodeCreatePostRouteTag, VNodeDeleteGetRouteTag,
    VNodeDeletePostRouteTag, VNodeDetailRouteTag, VNodeDownloadRootRouteTag,
    VNodeDownloadRouteTag, VNodeEditGetRouteTag, VNodeEditPostRouteTag,
    VNodeListRouteTag, VNodeMoveGetRouteTag, VNodeMovePostRouteTag, VNodeMoveSelectInRouteTag,
    VNodeMoveSelectRouteTag, VNodeSelectInRouteTag, VNodeSelectRouteTag,
    VNodeUploadGetInRouteTag, VNodeUploadGetRouteTag, VNodeUploadPostInRouteTag,
    VNodeUploadPostRouteTag, VNodeZipUploadGetInRouteTag, VNodeZipUploadGetRouteTag,
    VNodeZipUploadPostInRouteTag, VNodeZipUploadPostRouteTag,
};

define_register_items! {
    plugin: FilesystemTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ListIdx: VNodeListPageTag => VNodeListPage,
        DetailIdx: VNodeDetailPageTag => VNodeDetailPage,
        EditModalIdx: VNodeEditModalPageTag => VNodeEditModalPage,
        MoveIdx: VNodeMoveFormPageTag => VNodeMoveFormPage,
        CreateModalIdx: VNodeCreateModalPageTag => VNodeCreateModalPage,
        MultiUploadModalIdx: VNodeMultiUploadModalPageTag => VNodeMultiUploadModalPage,
        ZipUploadModalIdx: VNodeZipUploadModalPageTag => VNodeZipUploadModalPage,
        SelectIdx: VNodeSelectPageTag => VNodeSelectPage,
        ConfirmIdx: VNodeConfirmDeletePageTag => VNodeConfirmDeletePage,
    ]
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

fn scaffold_pane(sidebar: Markup, crumbs: Markup, body: Markup) -> crate::components::AppLayoutHtml {
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

fn filesystem_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Filesystem",
        href: None,
    }])
}

fn filesystem_browse_crumbs(parent_name: &str) -> Markup {
    let list_url = VNodeListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Filesystem",
            href: Some(&list_url),
        },
        Crumb {
            label: parent_name,
            href: None,
        },
    ])
}

fn filesystem_item_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = VNodeListRouteTag.url();
    let detail_url = VNodeDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Filesystem",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Filesystem",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: Some(&detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

/// Main sidebar: browsing the filesystem root.
fn main_menu(current_path: &str) -> Markup {
    let list_url = VNodeListRouteTag.url();
    let create_url = VNodeCreateGetRouteTag.url();
    let upload_url = VNodeUploadGetRouteTag.url();
    let zip_url = VNodeZipUploadGetRouteTag.url();
    let nav = [SidebarNavLink {
        key: "list",
        title: "All Files",
        url: &list_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "Filesystem",
        children: html! {
            (sidebar_nav_items_pane(&nav, current_path))
            (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                label: "Create Item",
                href: &create_url,
                name: "p_filesystem.VNodeCreateForm",
                ..Default::default()
            }))
            (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                label: "Bulk Upload",
                href: &upload_url,
                name: "p_filesystem.VNodeMultiUploadForm",
                ..Default::default()
            }))
            (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                label: "Upload Zip",
                href: &zip_url,
                name: "p_filesystem.VNodeZipUploadForm",
                ..Default::default()
            }))
        },
    })
}

/// Sidebar for a specific node. `active` selects
/// which entry (if any) is highlighted (`"detail"`, `"move"`, `"browse"`).
fn vnode_menu(id: i64, name: &str, is_directory: bool, active: &str) -> Markup {
    let menu_title = format!("Item: {name}");
    let detail_url = VNodeDetailRouteTag::new(id).url();
    let move_url = VNodeMoveGetRouteTag::new(id).url();
    let browse_url = VNodeBrowseRouteTag::new(id).url();
    let create_get_url = VNodeCreateGetInRouteTag::new(id).url();
    let upload_get_url = VNodeUploadGetInRouteTag::new(id).url();
    let zip_upload_get_url = VNodeZipUploadGetInRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "View Details",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Move",
                url: &move_url,
                active: active == "move",
                ..Default::default()
            }))
            @if is_directory {
                (sidebar_menu_item_pane(SidebarMenuItem {
                    title: "Browse Contents",
                    url: &browse_url,
                    active: active == "browse",
                    ..Default::default()
                }))
                (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                    label: "Add New Item",
                    href: &create_get_url,
                    name: "p_filesystem.VNodeCreateForm",
                    ..Default::default()
                }))
                (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                    label: "Bulk Upload",
                    href: &upload_get_url,
                    name: "p_filesystem.VNodeMultiUploadForm",
                    ..Default::default()
                }))
                (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                    label: "Upload Zip",
                    href: &zip_upload_get_url,
                    name: "p_filesystem.VNodeZipUploadForm",
                    ..Default::default()
                }))
            }
        },
    })
}

fn vnode_filter_form<K: SwapKey, R: crate::http::FragmentGet<K> + crate::http::RouteUrl + Copy + Default>(name: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: VNodeNameFilterForm::render_inputs(
            &FormCtx::form::<VNodeNameFilterForm>()
                .value(VNodeNameFilterFormField::Name, name),
        ),
        actions: html! {
            (container_row(
                "flex gap-2",
                html! {
                    (button_submit(ButtonSubmit {
                        label: "Apply Filters",
                        ..Default::default()
                    }))
                    (crate::components::button_clear(crate::components::ButtonClear {
                        label: "Clear",
                        ..Default::default()
                    }))
                },
            ))
        },
        ..Default::default()
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

/// Row payload for [`VNodeListPage`].
#[derive(Clone)]
pub struct VNodeRow {
    pub id: i64,
    pub name: String,
    pub is_directory: bool,
    pub size_display: String,
    pub items_display: String,
    pub updated_at: String,
}

/// Root list (`/filesystem`) or a directory's contents (`/filesystem/browse/{id}`).
/// `parent_id == 0` means the filesystem root.
#[derive(Generic)]
pub struct VNodeListPage {
    pub parent_id: i64,
    pub parent_name: String,
    pub items: ObjectList<VNodeRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
}

impl VNodeListPage {
    fn menu(&self) -> Markup {
        if self.parent_id == 0 {
            main_menu(&self.path_and_query)
        } else {
            vnode_menu(self.parent_id, &self.parent_name, true, "browse")
        }
    }

    fn crumbs(&self) -> Markup {
        if self.parent_id == 0 {
            filesystem_list_crumbs()
        } else {
            filesystem_browse_crumbs(&self.parent_name)
        }
    }

    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let modified_sort = column_sort_url(&self.path_and_query, "Modified", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let modified_label = format!("Modified{}", sort_indicator(&self.sort, "Modified"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Size",
                label: "Size",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Items",
                label: "Items",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Modified",
                label: &modified_label,
                sort_url: Some(&modified_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .items
            .items
            .iter()
            .map(|n| {
                let row_attrs = if n.is_directory {
                    row_attr_navigate_route(VNodeBrowseRouteTag::new(n.id))
                } else {
                    row_attr_navigate_route(VNodeDetailRouteTag::new(n.id))
                };
                TableRow {
                    attrs: row_attrs,
                    cells: vec![
                        field_text(FieldText {
                            value: &n.name,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: if n.is_directory { "Directory" } else { "File" },
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &n.size_display,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &n.items_display,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &n.updated_at,
                            classes: "",
                        }),
                    ],
                }
            })
            .collect();
        let filter_panel = if self.parent_id == 0 {
            vnode_filter_form::<VNodeTableKey, VNodeListRouteTag>(&self.filter_name)
        } else {
            form(FormOpts {
                attrs: crate::components::swap::form_hx_get_for_url::<VNodeTableKey>(
                    &VNodeBrowseRouteTag::new(self.parent_id).url(),
                ),
                inputs: VNodeNameFilterForm::render_inputs(
                    &FormCtx::form::<VNodeNameFilterForm>()
                        .value(VNodeNameFilterFormField::Name, &self.filter_name),
                ),
                actions: html! {
                    (container_row(
                        "flex gap-2",
                        html! {
                            (button_submit(ButtonSubmit {
                                label: "Apply Filters",
                                ..Default::default()
                            }))
                            (crate::components::button_clear(crate::components::ButtonClear {
                                label: "Clear",
                                ..Default::default()
                            }))
                        },
                    ))
                },
                ..Default::default()
            })
        };
        let (create_href, create_path) = if self.parent_id == 0 {
            (VNodeCreateGetRouteTag.url(), VNodeCreateGetRouteTag.path())
        } else {
            (
                VNodeCreateGetInRouteTag::new(self.parent_id).url(),
                VNodeCreateGetInRouteTag::new(self.parent_id).path(),
            )
        };
        let download_btn = if self.parent_id == 0 {
            crate::components::button_download_route(VNodeDownloadRootRouteTag, 
                "Download Zip", "btn-outline btn-sm",
            )
        } else {
            crate::components::button_download_route(VNodeDownloadRouteTag::new(self.parent_id), 
                "Download Zip", "btn-outline btn-sm",
            )
        };
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: filter_panel,
                ..Default::default()
            }))
            (download_btn)
            (button_modal_form(ButtonModalForm {
                name: "p_filesystem.VNodeCreateForm",
                href: &create_href,
                form_post_url: &create_path,
                modal_uid: VNodeCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<VNodeTableKey>(
            &self.path_and_query,
            self.items.number,
            self.items.num_pages,
            true,
        );
        data_table_list_refresh::<VNodeTableKey>(
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl crate::template::RenderAppPane for VNodeListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(self.menu(), self.crumbs(), self.render_table())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.render_table())
    }
}

impl RenderTemplate for VNodeListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Filesystem — Lariv",
            chrome,
            self.menu(),
            self.crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct VNodeDetailPage {
    pub id: i64,
    pub name: String,
    pub is_directory: bool,
    pub item_type: String,
    pub size_display: String,
    pub items_display: String,
    pub path: String,
    pub updated_at: String,
}

impl VNodeDetailPage {
    fn pane_body(&self) -> Markup {
        let browse_url = VNodeBrowseRouteTag::new(self.id).url();
        let edit_get = VNodeEditGetRouteTag::new(self.id).url();
        let edit_post = VNodeEditPostRouteTag::new(self.id).path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (label_inline("Type", field_text(FieldText {
                        value: &self.item_type,
                        classes: "",
                    })))
                    (label_inline("Size", field_text(FieldText {
                        value: &self.size_display,
                        classes: "",
                    })))
                    @if self.is_directory {
                        (label_inline("Items", field_text(FieldText {
                            value: &self.items_display,
                            classes: "",
                        })))
                    }
                    (label_inline("Path", field_text(FieldText {
                        value: &self.path,
                        classes: "",
                    })))
                    (label_inline("Modified", field_text(FieldText {
                        value: &self.updated_at,
                        classes: "",
                    })))
                    (container_row(
                        "flex gap-2 mt-4",
                        html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_filesystem.VNodeEditForm",
                                href: &edit_get,
                                form_post_url: &edit_post,
                                modal_uid: VNodeEditModalKey::ID,
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                            @if self.is_directory {
                                (button_link(ButtonLink {
                                    label: "Browse Contents",
                                    href: &browse_url,
                                    icon_name: Some("folder-open"),
                                    ..Default::default()
                                }))
                            } @else {
                                (crate::components::button_download_route(VNodeDownloadRouteTag::new(self.id), 
                                    "Download", "",
                                ))
                            }
                        },
                    ))
                },
            ))
        })
    }
}

impl crate::template::RenderAppPane for VNodeDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            vnode_menu(self.id, &self.name, self.is_directory, "detail"),
            filesystem_item_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            filesystem_item_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
}

impl RenderTemplate for VNodeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            vnode_menu(self.id, &self.name, self.is_directory, "detail"),
            filesystem_item_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
}

/// Edit modal for filesystem nodes. Create uses [`VNodeCreateModalPage`].
#[derive(Generic)]
pub struct VNodeEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub is_directory: bool,
    pub has_file: bool,
    pub error: String,
}

impl RenderTemplate for VNodeEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = VNodeDeleteGetRouteTag::new(self.id).url();
        let file_label = if self.has_file { "Replace File" } else { "File" };
        let show_file_field = !self.is_directory;
        let ctx = FormCtx::form::<VNodeEditForm>()
            .value(VNodeEditFormField::Name, self.name.as_str())
            .flag(VNodeEditFormFlag::ShowFile, show_file_field)
            .label(VNodeEditFormField::File, file_label);
        let inputs = VNodeEditForm::render_inputs(&ctx);
        modal_keyed::<VNodeEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit item" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<VNodeEditModalKey>(&modal_edit_post_url(
                        VNodeEditPostRouteTag::new(self.id),
                        &self.form_name,
                    ))
                    .set("hx-encoding", "multipart/form-data"),
                    enctype: Some("multipart/form-data"),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: container_column("", inputs),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_filesystem.VNodeDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: VNodeDeleteModalKey::ID,
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
pub struct VNodeCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub is_directory: bool,
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeCreateModalPage {
    fn post_url(&self, form_name: &str) -> String {
        if self.parent_id == 0 {
            modal_create_post_url(VNodeCreatePostRouteTag, form_name, &self.refresh_table)
        } else {
            modal_create_post_url(
                VNodeCreatePostInRouteTag::new(self.parent_id),
                form_name,
                &self.refresh_table,
            )
        }
    }

    fn inputs(&self) -> Markup {
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let parent_val = if self.parent_id == 0 {
            ""
        } else {
            parent_id_s.as_str()
        };
        let ctx = FormCtx::form::<VNodeForm>()
            .value(VNodeFormField::Name, self.name.as_str())
            .flag(VNodeFormFlag::CreateMode, true)
            .kind::<VNodeKind>("File")
            .value(VNodeFormField::ParentId, parent_val)
            .display(VNodeFormField::ParentId, self.parent_display.as_str())
            .label(VNodeKindField::File, "File");
        container_column("", VNodeForm::render_inputs(&ctx))
    }
}

impl RenderTemplate for VNodeCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_filesystem.VNodeCreateForm"
        } else {
            self.form_name.as_str()
        };
        let post_url = self.post_url(form_name);
        modal_keyed::<VNodeCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Item",
                subtitle: "Add a new file or folder",
                classes: "@container",
                attrs: form_hx_post_url::<VNodeCreateModalKey>(&post_url)
                    .set("hx-encoding", "multipart/form-data"),
                enctype: Some("multipart/form-data"),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: self.inputs(),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Save",
                        classes: "btn-primary",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            }),
        )
    }
}

/// Move form: pick a new parent directory.
#[derive(Generic)]
pub struct VNodeMoveFormPage {
    pub id: i64,
    pub name: String,
    pub is_directory: bool,
    pub destination_id: i64,
    pub destination_display: String,
    pub error: String,
}

impl VNodeMoveFormPage {
    fn crumbs(&self) -> Markup {
        filesystem_item_crumbs(self.id, &self.name, Some("Move"))
    }

    fn pane_body(&self) -> Markup {
        let destination_id_s = if self.destination_id == 0 {
            String::new()
        } else {
            self.destination_id.to_string()
        };
        let select_url = VNodeMoveSelectRouteTag.with_query().query("exclude_id", self.id).build_with_query();
        let ctx = FormCtx::form::<MoveForm>()
            .value(MoveFormField::DestinationId, destination_id_s.as_str())
            .display(MoveFormField::DestinationId, self.destination_display.as_str())
            .url(MoveFormField::DestinationId, select_url.as_str());
        form(FormOpts {
            title: "Move Item",
            subtitle: &format!("Choose a new location for \"{}\"", self.name),
            attrs: form_hx_post_main(VNodeMovePostRouteTag::new(self.id)),
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: MoveForm::render_inputs(&ctx),
            actions: html! {
                (button_submit(ButtonSubmit {
                    label: "Move",
                    ..Default::default()
                }))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for VNodeMoveFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            vnode_menu(self.id, &self.name, self.is_directory, "move"),
            self.crumbs(),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.pane_body())
    }
}

impl RenderTemplate for VNodeMoveFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("Move {} — Lariv", self.name),
            chrome,
            vnode_menu(self.id, &self.name, self.is_directory, "move"),
            self.crumbs(),
            self.pane_body(),
        )
    }
}

/// Multi-file upload modal.
#[derive(Generic)]
pub struct VNodeMultiUploadModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeMultiUploadModalPage {
    fn post_url(&self, form_name: &str) -> String {
        if self.parent_id == 0 {
            modal_create_post_url(VNodeUploadPostRouteTag, form_name, &self.refresh_table)
        } else {
            modal_create_post_url(
                VNodeUploadPostInRouteTag::new(self.parent_id),
                form_name,
                &self.refresh_table,
            )
        }
    }
}

impl RenderTemplate for VNodeMultiUploadModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_filesystem.VNodeMultiUploadForm"
        } else {
            self.form_name.as_str()
        };
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let ctx = FormCtx::form::<VNodeMultiUploadForm>()
            .value(VNodeMultiUploadFormField::ParentId, parent_id_s.as_str())
            .display(VNodeMultiUploadFormField::ParentId, self.parent_display.as_str());
        modal_keyed::<VNodeMultiUploadModalKey>(
            "",
            form(FormOpts {
                title: "Bulk Upload",
                subtitle: "Upload multiple files at once",
                attrs: form_hx_post_url::<VNodeMultiUploadModalKey>(&self.post_url(form_name))
                    .set("hx-encoding", "multipart/form-data"),
                enctype: Some("multipart/form-data"),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: VNodeMultiUploadForm::render_inputs(&ctx),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Upload",
                        classes: "btn-primary",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            }),
        )
    }
}

/// Zip-upload modal: replaces the target directory's contents.
#[derive(Generic)]
pub struct VNodeZipUploadModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeZipUploadModalPage {
    fn post_url(&self, form_name: &str) -> String {
        if self.parent_id == 0 {
            modal_create_post_url(VNodeZipUploadPostRouteTag, form_name, &self.refresh_table)
        } else {
            modal_create_post_url(
                VNodeZipUploadPostInRouteTag::new(self.parent_id),
                form_name,
                &self.refresh_table,
            )
        }
    }
}

impl RenderTemplate for VNodeZipUploadModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_filesystem.VNodeZipUploadForm"
        } else {
            self.form_name.as_str()
        };
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let ctx = FormCtx::form::<VNodeZipUploadForm>()
            .value(VNodeZipUploadFormField::ParentId, parent_id_s.as_str())
            .display(VNodeZipUploadFormField::ParentId, self.parent_display.as_str());
        modal_keyed::<VNodeZipUploadModalKey>(
            "",
            form(FormOpts {
                title: "Upload Zip",
                subtitle: "Replaces the contents of the destination folder",
                attrs: form_hx_post_url::<VNodeZipUploadModalKey>(&self.post_url(form_name))
                    .set("hx-encoding", "multipart/form-data"),
                enctype: Some("multipart/form-data"),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: VNodeZipUploadForm::render_inputs(&ctx),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Upload",
                        classes: "btn-primary",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            }),
        )
    }
}

/// Directory picker option.
#[derive(Clone)]
pub struct VNodeOption {
    pub id: i64,
    pub name: String,
}

/// Shared directory-picker modal for `ParentID` and `DestinationID` foreign keys
///.
#[derive(Generic)]
pub struct VNodeSelectPage {
    pub items: ObjectList<VNodeOption>,
    pub filter_name: String,
    pub target_input: String,
    pub browse_base: String,
    pub parent_id: i64,
    pub current_path: String,
    pub exclude_id: i64,
    pub sort: String,
    pub path_and_query: String,
}

impl VNodeSelectPage {
    fn browse_route_url(&self, parent_id: i64) -> String {
        let is_move = self.browse_base.contains("move-select");
        let target = (!self.target_input.is_empty()).then_some(self.target_input.as_str());
        let exclude = (self.exclude_id != 0).then_some(self.exclude_id);
        match (is_move, parent_id) {
            (false, 0) => VNodeSelectRouteTag.with_query().query_opt("target_input", target).query_opt("exclude_id", exclude).build(),
            (false, pid) => VNodeSelectInRouteTag::new(pid).with_query().query_opt("target_input", target).query_opt("exclude_id", exclude).build(),
            (true, 0) => VNodeMoveSelectRouteTag.with_query().query_opt("target_input", target).query_opt("exclude_id", exclude).build(),
            (true, pid) => VNodeMoveSelectInRouteTag::new(pid).with_query().query_opt("target_input", target).query_opt("exclude_id", exclude).build(),
        }
    }

    pub fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "ParentID"
        } else {
            self.target_input.as_str()
        };
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .items
            .items
            .iter()
            .filter(|n| n.id != self.exclude_id)
            .map(|n| {
                let browse_url = self.browse_route_url(n.id);
                let attrs = row_attr_select(target, &n.id.to_string(), &n.name).set("hx-get", browse_url);
                TableRow {
                    attrs,
                    cells: vec![field_text(FieldText {
                        value: &n.name,
                        classes: "",
                    })],
                }
            })
            .collect();
        let root_select = row_attr_select(target, "0", "Filesystem root");
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: crate::components::swap::form_hx_get_for_url::<VNodeSelectTableKey>(
                        &self.browse_route_url(self.parent_id),
                    )
                        .set("hx-push-url", "false"),
                    inputs: VNodeNameFilterForm::render_inputs(
                        &FormCtx::form::<VNodeNameFilterForm>()
                            .value(VNodeNameFilterFormField::Name, &self.filter_name),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Apply",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<VNodeSelectTableKey>(
            &self.path_and_query,
            self.items.number,
            self.items.num_pages,
            false,
        );
        let use_here_label = format!("Use here: {}", self.current_path);
        html! {
            div class="mb-2" {
                @if self.parent_id != 0 {
                    (maud::PreEscaped(format!(
                        r#"<div class="btn btn-sm btn-outline mb-2"{}>"#,
                        root_select.as_string()
                    )))
                    (use_here_label)
                    (maud::PreEscaped("</div>"))
                }
            }
            (data_table_list::<VNodeSelectTableKey>(
                "Select a Folder",
                actions,
                &headers,
                &rows,
                pagination,
            ))
        }
    }
}

impl RenderTemplate for VNodeSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<VNodeSelectModalKey>("", self.render_table())
    }
}

/// Confirmation modal for deleting a filesystem node.
#[derive(Generic)]
pub struct VNodeConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
}

impl RenderTemplate for VNodeConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", VNodeDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            VNodeDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        modal(crate::components::Modal {
            uid,
            children: crate::components::delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: crate::components::form_hx_post_selector(
                    &VNodeDeletePostRouteTag::new(self.id).url(),
                    &target,
                ),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

define_register_items! {
    plugin: FilesystemTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

#[cfg(test)]
mod vnode_form_page_tests {
    use super::{VNodeCreateModalPage, VNodeDetailPage, VNodeEditModalPage};
    use crate::template::{RenderAppPane, RenderTemplate};

    #[test]
    fn create_modal_shows_kind_radios() {
        let page = VNodeCreateModalPage {
            form_name: String::new(),
            refresh_table: String::new(),
            name: String::new(),
            is_directory: false,
            parent_id: 0,
            parent_display: String::new(),
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(html.contains("name=\"Kind\""), "create: {html}");
        assert!(html.contains("type=\"radio\""), "create: {html}");
        assert!(html.contains("type=\"file\""), "create: {html}");
        assert!(html.contains("name=\"ParentID\""), "create: {html}");
        assert!(html.contains("multipart/form-data"), "create: {html}");
    }

    #[test]
    fn detail_page_shows_breadcrumbs() {
        let page = VNodeDetailPage {
            id: 42,
            name: "docs".into(),
            is_directory: true,
            item_type: "Directory".into(),
            size_display: "—".into(),
            items_display: "3".into(),
            path: "/docs".into(),
            updated_at: "2026-01-01".into(),
        };
        let html = page.render_main().into_markup().into_string();
        assert!(html.contains(r#"class="breadcrumbs"#), "detail: {html}");
        assert!(html.contains("/filesystem"), "detail: {html}");
        assert!(html.contains(">Filesystem</a>"), "detail: {html}");
        assert!(html.contains("<span>docs</span>"), "detail: {html}");
    }

    #[test]
    fn edit_dir_hides_file() {
        let page = VNodeEditModalPage {
            id: 1,
            form_name: "p_filesystem.VNodeEditForm".into(),
            name: "docs".into(),
            is_directory: true,
            has_file: false,
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(!html.contains("type=\"radio\""), "edit dir: {html}");
        assert!(!html.contains("type=\"file\""), "edit dir: {html}");
        assert!(html.contains("name=\"Name\""), "edit dir: {html}");
    }

    #[test]
    fn edit_file_shows_replace_file() {
        let page = VNodeEditModalPage {
            id: 2,
            form_name: "p_filesystem.VNodeEditForm".into(),
            name: "a.txt".into(),
            is_directory: false,
            has_file: true,
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(html.contains("type=\"file\""), "edit file: {html}");
        assert!(html.contains("Replace File"), "edit file: {html}");
    }
}
