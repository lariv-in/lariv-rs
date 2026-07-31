//! Filesystem plugin pages (Maud), ported from Go `p_filesystem` `pages_*.go`.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonDownload, ButtonLink, ButtonModalForm, ButtonSubmit, DeleteConfirmation, FieldText,
        FieldTitle, FormOpts, InputText, ObjectList, PaginationPage, ShellChrome,
        ShellScaffold, SidebarMenu, SidebarMenuBack, SidebarMenuItem, SlotCapability, SlotRegistrar, SwapKey,
        TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_link, button_submit,
        column_sort_url, container_column, container_row, data_table_list, detail, field_text,
        field_title, form, form_hx_get, form_hx_post_main, input_text, label_inline, modal,
        modal_keyed, pagination_pages, row_attr_navigate, row_attr_select, shell_scaffold,
        sidebar_menu, sidebar_menu_item, sort_indicator, table_button_filter, table_pagination,
    },
    capability::define_register_items,
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

use super::forms::{
    MoveForm, VNodeEditForm, VNodeForm, VNodeMultiUploadForm, VNodeZipUploadForm,
};
use super::keys::{
    VNodeDeleteModalKey, VNodeSelectModalKey, VNodeSelectTableKey, VNodeTableKey,
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
        FormIdx: VNodeFormPageTag => VNodeFormPage,
        MoveIdx: VNodeMoveFormPageTag => VNodeMoveFormPage,
        MultiIdx: VNodeMultiUploadFormPageTag => VNodeMultiUploadFormPage,
        ZipIdx: VNodeZipUploadFormPageTag => VNodeZipUploadFormPage,
        SelectIdx: VNodeSelectPageTag => VNodeSelectPage,
        ConfirmIdx: VNodeConfirmDeletePageTag => VNodeConfirmDeletePage,
    ]
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
    crate::components::layout::layout_sidebar(crate::components::LayoutSidebar {
        sidebar,
        content: body,
    })
}

fn scaffold_main(body: Markup) -> Markup {
    crate::components::layout::layout_main(body)
}

/// Main sidebar (Go `pages_menu.go` `MainMenu`): browsing the filesystem root.
fn main_menu() -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Filesystem",
        back: Some(SidebarMenuBack {
            title: "Back to Home",
            url: "/dashboard/",
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "All Files",
                url: "/filesystem",
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Create Item",
                url: "/filesystem/create",
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Bulk Upload",
                url: "/filesystem/upload",
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Upload Zip",
                url: "/filesystem/zip-upload",
                ..Default::default()
            }))
        },
    })
}

/// Sidebar for a specific node (Go `pages_menu.go` `VNodeMenu`). `active` selects
/// which entry (if any) is highlighted (`"detail"`, `"edit"`, `"move"`, `"browse"`).
fn vnode_menu(id: i64, name: &str, is_directory: bool, active: &str) -> Markup {
    let menu_title = format!("Item: {name}");
    let detail_url = format!("/filesystem/{id}");
    let edit_url = format!("/filesystem/{id}/edit");
    let move_url = format!("/filesystem/{id}/move");
    let browse_url = format!("/filesystem/browse/{id}");
    let create_url = format!("/filesystem/create/in/{id}");
    let upload_url = format!("/filesystem/upload/in/{id}");
    let zip_upload_url = format!("/filesystem/zip-upload/in/{id}");
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        back: Some(SidebarMenuBack {
            title: "Back to All Files",
            url: "/filesystem",
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "View Details",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Edit",
                url: &edit_url,
                active: active == "edit",
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Move",
                url: &move_url,
                active: active == "move",
                ..Default::default()
            }))
            @if is_directory {
                (sidebar_menu_item(SidebarMenuItem {
                    title: "Browse Contents",
                    url: &browse_url,
                    active: active == "browse",
                    ..Default::default()
                }))
                (sidebar_menu_item(SidebarMenuItem {
                    title: "Add New Item",
                    url: &create_url,
                    ..Default::default()
                }))
                (sidebar_menu_item(SidebarMenuItem {
                    title: "Bulk Upload",
                    url: &upload_url,
                    ..Default::default()
                }))
                (sidebar_menu_item(SidebarMenuItem {
                    title: "Upload Zip",
                    url: &zip_upload_url,
                    ..Default::default()
                }))
            }
        },
    })
}

fn vnode_filter_form<K: SwapKey>(name: &str, action: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get::<K>(action),
        inputs: html! {
            (input_text(InputText {
                label: "Name",
                name: "Name",
                value: name,
                ..Default::default()
            }))
        },
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

/// Row payload for [`VNodeListPage`] (Go list/browse table columns).
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
            main_menu()
        } else {
            vnode_menu(self.parent_id, &self.parent_name, true, "browse")
        }
    }

    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                label: "Type",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                label: "Size",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                label: "Items",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                label: "Modified",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .items
            .items
            .iter()
            .map(|n| {
                let href = if n.is_directory {
                    format!("/filesystem/browse/{}", n.id)
                } else {
                    format!("/filesystem/{}", n.id)
                };
                TableRow {
                    attrs: row_attr_navigate(&href),
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
        let create_url = if self.parent_id == 0 {
            "/filesystem/create".to_string()
        } else {
            format!("/filesystem/create/in/{}", self.parent_id)
        };
        let download_url = if self.parent_id == 0 {
            "/filesystem/download".to_string()
        } else {
            format!("/filesystem/{}/download", self.parent_id)
        };
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: vnode_filter_form::<VNodeTableKey>(&self.filter_name, &format!(
                    "/filesystem{}",
                    if self.parent_id == 0 {
                        String::new()
                    } else {
                        format!("/browse/{}", self.parent_id)
                    }
                )),
                ..Default::default()
            }))
            (crate::components::button_download(ButtonDownload {
                label: "Download Zip",
                href: &download_url,
                classes: "btn-outline btn-sm",
                ..Default::default()
            }))
            (button_link(ButtonLink {
                href: &create_url,
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
        data_table_list::<VNodeTableKey>("", actions, &headers, &rows, pagination)
    }
}

impl crate::template::RenderAppPane for VNodeListPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(self.menu(), self.render_table())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.render_table())
    }
}

impl RenderTemplate for VNodeListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Filesystem — Lariv", chrome, self.menu(), self.render_table())
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
        let download_url = format!("/filesystem/{}/download", self.id);
        let browse_url = format!("/filesystem/browse/{}", self.id);
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
                            @if self.is_directory {
                                (button_link(ButtonLink {
                                    label: "Browse Contents",
                                    href: &browse_url,
                                    icon_name: Some("folder-open"),
                                    ..Default::default()
                                }))
                            } @else {
                                (crate::components::button_download(ButtonDownload {
                                    label: "Download",
                                    href: &download_url,
                                    ..Default::default()
                                }))
                            }
                        },
                    ))
                },
            ))
        })
    }
}

impl crate::template::RenderAppPane for VNodeDetailPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(
            vnode_menu(self.id, &self.name, self.is_directory, "detail"),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for VNodeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            vnode_menu(self.id, &self.name, self.is_directory, "detail"),
            self.pane_body(),
        )
    }
}

/// Create/edit form for filesystem nodes. `id == 0` is create.
#[derive(Generic)]
pub struct VNodeFormPage {
    pub id: i64,
    pub name: String,
    pub is_directory: bool,
    pub is_edit: bool,
    pub has_file: bool,
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeFormPage {
    fn menu(&self) -> Markup {
        if self.is_edit {
            vnode_menu(self.id, &self.name, self.is_directory, "edit")
        } else if self.parent_id == 0 {
            main_menu()
        } else {
            vnode_menu(self.parent_id, &self.parent_display, true, "")
        }
    }

    fn pane_body(&self) -> Markup {
        let action = if self.is_edit {
            format!("/filesystem/{}/edit", self.id)
        } else if self.parent_id == 0 {
            "/filesystem/create".to_string()
        } else {
            format!("/filesystem/create/in/{}", self.parent_id)
        };
        let delete_url = format!("/filesystem/{}/delete", self.id);
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let file_label = if self.has_file { "Replace File" } else { "File" };
        let show_file_field = !self.is_edit || !self.is_directory;
        let inputs = if self.is_edit {
            let ctx = FormCtx::new()
                .value("Name", self.name.as_str())
                .flag("show_file", show_file_field)
                .label("File", file_label);
            VNodeEditForm::render_inputs(&ctx)
        } else {
            let parent_val = if self.parent_id == 0 {
                ""
            } else {
                parent_id_s.as_str()
            };
            let ctx = FormCtx::new()
                .value("Name", self.name.as_str())
                .flag("create_mode", true)
                .kind("Kind", "File")
                .value("ParentID", parent_val)
                .display("parent", self.parent_display.as_str())
                .label("File", file_label);
            VNodeForm::render_inputs(&ctx)
        };
        form(FormOpts {
            title: if self.is_edit { "Edit Item" } else { "Create Item" },
            subtitle: if self.is_edit {
                "Update item details"
            } else {
                "Add a new file or folder"
            },
            classes: "@container",
            attrs: form_hx_post_main(&action).set("hx-encoding", "multipart/form-data"),
            enctype: Some("multipart/form-data"),
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: container_column("", inputs),
            actions: html! {
                (container_row(
                    "flex flex-wrap justify-between gap-2 mt-2 items-center",
                    html! {
                        (container_row(
                            "flex justify-end gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save",
                                    ..Default::default()
                                }))
                                @if self.is_edit {
                                    (crate::components::button_modal_form(ButtonModalForm {
                                        label: "Delete",
                                        icon_name: Some("trash"),
                                        name: "p_filesystem.VNodeDeleteForm",
                                        href: &delete_url,
                                        form_post_url: &delete_url,
                                        modal_uid: VNodeDeleteModalKey::ID,
                                        classes: "btn-error",
                                        ..Default::default()
                                    }))
                                }
                            },
                        ))
                    },
                ))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for VNodeFormPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(self.menu(), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for VNodeFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let title = if self.is_edit {
            "Edit item — Lariv"
        } else {
            "Create item — Lariv"
        };
        app_scaffold(title, chrome, self.menu(), self.pane_body())
    }
}

/// Move form (Go `VNodeMoveForm`): pick a new parent directory.
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
    fn pane_body(&self) -> Markup {
        let action = format!("/filesystem/{}/move", self.id);
        let destination_id_s = if self.destination_id == 0 {
            String::new()
        } else {
            self.destination_id.to_string()
        };
        let select_url = format!("/filesystem/move-select?exclude_id={}", self.id);
        let ctx = FormCtx::new()
            .value("DestinationID", destination_id_s.as_str())
            .display("destination", self.destination_display.as_str())
            .url("DestinationID", select_url.as_str());
        form(FormOpts {
            title: "Move Item",
            subtitle: &format!("Choose a new location for \"{}\"", self.name),
            attrs: form_hx_post_main(&action),
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
    fn render_pane(&self) -> Markup {
        scaffold_pane(
            vnode_menu(self.id, &self.name, self.is_directory, "move"),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for VNodeMoveFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("Move {} — Lariv", self.name),
            chrome,
            vnode_menu(self.id, &self.name, self.is_directory, "move"),
            self.pane_body(),
        )
    }
}

/// Multi-file upload form (Go `VNodeMultiUploadForm`).
#[derive(Generic)]
pub struct VNodeMultiUploadFormPage {
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeMultiUploadFormPage {
    fn menu(&self) -> Markup {
        if self.parent_id == 0 {
            main_menu()
        } else {
            vnode_menu(self.parent_id, &self.parent_display, true, "")
        }
    }

    fn pane_body(&self) -> Markup {
        let action = if self.parent_id == 0 {
            "/filesystem/upload".to_string()
        } else {
            format!("/filesystem/upload/in/{}", self.parent_id)
        };
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let ctx = FormCtx::new()
            .value("ParentID", parent_id_s.as_str())
            .display("parent", self.parent_display.as_str());
        form(FormOpts {
            title: "Bulk Upload",
            subtitle: "Upload multiple files at once",
            attrs: form_hx_post_main(&action).set("hx-encoding", "multipart/form-data"),
            enctype: Some("multipart/form-data"),
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: VNodeMultiUploadForm::render_inputs(&ctx),
            actions: html! {
                (button_submit(ButtonSubmit {
                    label: "Upload",
                    ..Default::default()
                }))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for VNodeMultiUploadFormPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(self.menu(), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for VNodeMultiUploadFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Bulk Upload — Lariv", chrome, self.menu(), self.pane_body())
    }
}

/// Zip-upload form (Go `VNodeZipUploadForm`): replaces the target directory's contents.
#[derive(Generic)]
pub struct VNodeZipUploadFormPage {
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeZipUploadFormPage {
    fn menu(&self) -> Markup {
        if self.parent_id == 0 {
            main_menu()
        } else {
            vnode_menu(self.parent_id, &self.parent_display, true, "")
        }
    }

    fn pane_body(&self) -> Markup {
        let action = if self.parent_id == 0 {
            "/filesystem/zip-upload".to_string()
        } else {
            format!("/filesystem/zip-upload/in/{}", self.parent_id)
        };
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let ctx = FormCtx::new()
            .value("ParentID", parent_id_s.as_str())
            .display("parent", self.parent_display.as_str());
        form(FormOpts {
            title: "Upload Zip",
            subtitle: "Replaces the contents of the destination folder",
            attrs: form_hx_post_main(&action).set("hx-encoding", "multipart/form-data"),
            enctype: Some("multipart/form-data"),
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: VNodeZipUploadForm::render_inputs(&ctx),
            actions: html! {
                (button_submit(ButtonSubmit {
                    label: "Upload",
                    ..Default::default()
                }))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for VNodeZipUploadFormPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(self.menu(), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for VNodeZipUploadFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Upload Zip — Lariv", chrome, self.menu(), self.pane_body())
    }
}

/// Directory picker option (Go `registry.Pair`-style FK option).
#[derive(Clone)]
pub struct VNodeOption {
    pub id: i64,
    pub name: String,
}

/// Shared directory-picker modal for `ParentID` and `DestinationID` foreign keys
/// (Go `ParentSelectionTable` / `DestinationSelectionTable`).
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
    fn browse_query(&self, parent_id: i64) -> String {
        let mut url = if parent_id == 0 {
            self.browse_base.clone()
        } else {
            format!("{}/in/{}", self.browse_base, parent_id)
        };
        let mut params = Vec::new();
        if !self.target_input.is_empty() {
            params.push(format!("target_input={}", self.target_input));
        }
        if self.exclude_id != 0 {
            params.push(format!("exclude_id={}", self.exclude_id));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }
        url
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
                let browse_url = self.browse_query(n.id);
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
                    attrs: form_hx_get::<VNodeSelectTableKey>(&self.browse_query(self.parent_id))
                        .set("hx-push-url", "false"),
                    inputs: html! {
                        (input_text(InputText {
                            label: "Name",
                            name: "Name",
                            value: &self.filter_name,
                            ..Default::default()
                        }))
                    },
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

/// Confirmation modal for deleting a filesystem node (Go `VNodeDeleteForm`).
#[derive(Generic)]
pub struct VNodeConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub action: String,
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
                attrs: crate::components::form_hx_post_selector(&self.action, &target),
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
    use super::VNodeFormPage;
    use crate::template::RenderAppPane;

    #[test]
    fn create_shows_kind_radios() {
        let page = VNodeFormPage {
            id: 0,
            name: String::new(),
            is_directory: false,
            is_edit: false,
            has_file: false,
            parent_id: 0,
            parent_display: String::new(),
            error: String::new(),
        };
        let html = page.render_main().into_string();
        assert!(html.contains("name=\"Kind\""), "create: {html}");
        assert!(html.contains("type=\"radio\""), "create: {html}");
        assert!(html.contains("type=\"file\""), "create: {html}");
        assert!(html.contains("name=\"ParentID\""), "create: {html}");
    }

    #[test]
    fn edit_dir_hides_file() {
        let page = VNodeFormPage {
            id: 1,
            name: "docs".into(),
            is_directory: true,
            is_edit: true,
            has_file: false,
            parent_id: 0,
            parent_display: String::new(),
            error: String::new(),
        };
        let html = page.render_main().into_string();
        assert!(!html.contains("type=\"radio\""), "edit dir: {html}");
        assert!(!html.contains("type=\"file\""), "edit dir: {html}");
        assert!(html.contains("name=\"Name\""), "edit dir: {html}");
    }

    #[test]
    fn edit_file_shows_replace_file() {
        let page = VNodeFormPage {
            id: 2,
            name: "a.txt".into(),
            is_directory: false,
            is_edit: true,
            has_file: true,
            parent_id: 0,
            parent_display: String::new(),
            error: String::new(),
        };
        let html = page.render_main().into_string();
        assert!(html.contains("type=\"file\""), "edit file: {html}");
        assert!(html.contains("Replace File"), "edit file: {html}");
    }
}
