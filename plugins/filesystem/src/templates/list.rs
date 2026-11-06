use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::picker::RenderPickerSelect;
use lariv_core::components::{
    ButtonModalForm, DataTable, DataTableDisplay, FieldText, FilterClauseView, HtmlAttrs,
    ListFilterBuilder, ObjectList, ShellChrome, SwapKey, TableButtonFilter, TableColumnHeader,
    TableListContent, TableRow, button_modal_form, column_sort_url, data_table, field_text,
    filter_error_banner, form_hx_get_route, list_filter_builder, render_pagination_push,
    render_picker_pagination, row_attr_navigate_route, row_attr_select, row_attr_select_multi,
    sort_indicator, table_button_bulk_actions, table_button_columns, table_button_filter,
    table_list_content,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::modal_create_href_for_picker;

use crate::create_modals::V_NODE_CREATE;
use crate::keys::{V_NODE_CREATE_MODAL, V_NODE_SELECT_MODAL, V_NODE_SELECT_TABLE, V_NODE_TABLE};
use crate::routes::{
    VNodeBrowseRouteTag, VNodeBulkDeleteGetRouteTag, VNodeBulkDownloadRouteTag,
    VNodeBulkMoveGetRouteTag, VNodeDetailRouteTag, VNodeDownloadRootRouteTag,
    VNodeDownloadRouteTag, VNodeFileSelectInRouteTag, VNodeFileSelectRouteTag, VNodeListRouteTag,
    VNodeMoveSelectInRouteTag, VNodeMoveSelectRouteTag, VNodeNodeSelectInRouteTag,
    VNodeNodeSelectRouteTag, VNodeSelectInRouteTag, VNodeSelectRouteTag,
};

use super::{
    app_scaffold, filesystem_browse_crumbs, filesystem_list_crumbs, main_menu, scaffold_main,
    scaffold_pane, vnode_create_get_url, vnode_menu,
};

fn data_table_filtered(
    key: SwapKey,
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
    errors: &[FilterRowError],
) -> Markup {
    let hx_target = key.selector();
    let list = html! {
        (filter_error_banner(errors))
        (table_list_content(TableListContent {
            headers,
            rows,
            hx_target: &hx_target,
        }))
    };
    let grid = html! {
        (filter_error_banner(errors))
        (lariv_core::components::table::table_grid_content(headers, rows))
    };
    let column_keys: Vec<&str> = headers
        .iter()
        .map(|h| h.key)
        .filter(|k| !k.is_empty())
        .collect();
    let toolbar = if headers.len() > 1 {
        html! {
            (table_button_columns(headers))
            (actions)
        }
    } else {
        actions
    };
    data_table(DataTable {
        uid: key.id(),
        title,
        subtitle: "",
        classes: "w-full",
        actions: toolbar,
        displays: vec![
            DataTableDisplay {
                name: "Grid".into(),
                html: grid,
            },
            DataTableDisplay {
                name: "List".into(),
                html: list,
            },
        ],
        default_view: "List",
        pagination,
        oob: false,
        refresh_url,
        column_keys: &column_keys,
        instance_id: "",
        body_only: false,
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
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
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

    /// Alpine helpers on the selection root (outside the swapped table).
    fn selection_root_js() -> &'static str {
        "Alpine.$data($el.closest('[data-vnode-selection]'))"
    }

    fn selection_x_data() -> String {
        let list_prefix = VNodeListRouteTag.path();
        let list_url = lariv_core::http::trailing_slash(&list_prefix);
        let bulk_delete = lariv_core::http::trailing_slash(&VNodeBulkDeleteGetRouteTag.path());
        let bulk_move = lariv_core::http::trailing_slash(&VNodeBulkMoveGetRouteTag.path());
        let bulk_download = lariv_core::http::trailing_slash(&VNodeBulkDownloadRouteTag.path());
        r#"{
            selected: {},
            toggle(id) {
                const k = String(id);
                if (this.selected[k]) delete this.selected[k];
                else this.selected[k] = true;
            },
            setVisible(ids, on) {
                for (const id of ids) {
                    const k = String(id);
                    if (on) this.selected[k] = true;
                    else delete this.selected[k];
                }
            },
            allVisibleSelected(ids) {
                return ids.length > 0 && ids.every(id => !!this.selected[String(id)]);
            },
            someVisibleSelected(ids) {
                return ids.some(id => !!this.selected[String(id)]);
            },
            selectedIds() {
                return Object.keys(this.selected).filter(k => this.selected[k]);
            },
            returnTo() {
                const path = window.location.pathname + window.location.search;
                return path.startsWith('__LIST_PREFIX__') ? path : '__LIST_URL__';
            },
            bulkDeleteHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '__BULK_DELETE__?ids=' + ids.join(',')
                    + '&return=' + encodeURIComponent(this.returnTo());
            },
            bulkMoveHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '__BULK_MOVE__?ids=' + ids.join(',')
                    + '&return=' + encodeURIComponent(this.returnTo());
            },
            bulkDownloadHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '__BULK_DOWNLOAD__?ids=' + ids.join(',');
            },
            requestBulkDelete(el) {
                const href = this.bulkDeleteHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, { target: 'body', swap: 'beforeend', source: el });
            },
            requestBulkMove(el) {
                const href = this.bulkMoveHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                });
            },
            requestBulkDownload() {
                const href = this.bulkDownloadHref();
                if (href === '#') return;
                window.location.assign(href);
            }
        }"#
        .replace("__LIST_PREFIX__", &list_prefix)
        .replace("__LIST_URL__", &list_url)
        .replace("__BULK_DELETE__", &bulk_delete)
        .replace("__BULK_MOVE__", &bulk_move)
        .replace("__BULK_DOWNLOAD__", &bulk_download)
    }

    fn wrap_with_selection(&self, table: Markup) -> Markup {
        html! {
            (PreEscaped(format!(
                r#"<div data-vnode-selection x-data="{}">"#,
                lariv_core::components::attrs::escape_attr(&Self::selection_x_data()),
            )))
            (table)
            (PreEscaped("</div>"))
        }
    }

    pub fn render_table(&self) -> Markup {
        let sel = Self::selection_root_js();
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let modified_sort = column_sort_url(&self.path_and_query, "Modified", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let modified_label = format!("Modified{}", sort_indicator(&self.sort, "Modified"));

        let visible_ids: Vec<i64> = self.items.items.iter().map(|n| n.id).collect();
        let visible_ids_js = format!(
            "[{}]",
            visible_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let select_all_label = format!(
            r#"<label class="flex justify-center" @click.stop=""><input type="checkbox" class="checkbox checkbox-sm" @change="{sel}.setVisible({ids}, $event.target.checked)" :checked="{sel}.allVisibleSelected({ids})" x-effect="$el.indeterminate = {sel}.someVisibleSelected({ids}) && !{sel}.allVisibleSelected({ids})" /></label>"#,
            sel = sel,
            ids = visible_ids_js,
        );

        let headers = [
            TableColumnHeader {
                key: "Select",
                label: &select_all_label,
                sort_url: None,
                push_url: true,
            },
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
                        PreEscaped(format!(
                            r#"<label class="flex justify-center" @click.stop=""><input type="checkbox" class="checkbox checkbox-sm" @change="{sel}.toggle({id})" :checked="!!{sel}.selected['{id}']" /></label>"#,
                            sel = sel,
                            id = n.id,
                        ))
                        .into(),
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
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let filter_attrs = if self.parent_id == 0 {
            form_hx_get_route(VNodeListRouteTag)
        } else {
            lariv_core::components::swap::form_hx_get_for_url(
                &VNodeBrowseRouteTag::new(self.parent_id).url(),
            )
        };
        let filter_panel = list_filter_builder(ListFilterBuilder {
            fields: &self.fields,
            rows: &self.filters,
            page_size: self.page_size,
            sort: &self.sort,
            attrs: filter_attrs,
            apply_label: "Apply Filters",
            extra: html! {},
        });
        let create_href = vnode_create_get_url(self.parent_id);
        let download_btn = if self.parent_id == 0 {
            lariv_core::components::button_download_route(
                VNodeDownloadRootRouteTag,
                "Download Zip",
                "btn-outline btn-sm",
            )
        } else {
            lariv_core::components::button_download_route(
                VNodeDownloadRouteTag::new(self.parent_id),
                "Download Zip",
                "btn-outline btn-sm",
            )
        };
        let bulk_item = |label: &str, classes: &str, on_click: &str| {
            format!(
                r#"<button type="button" class="btn {classes} btn-sm justify-start w-full" x-bind:class="{sel}.selectedIds().length >= 1 ? '' : 'btn-disabled pointer-events-none opacity-50'" @click="{sel}.{on_click}($el); $el.closest('details')?.removeAttribute('open')">{label}</button>"#,
                classes = classes,
                sel = sel,
                on_click = on_click,
                label = label,
            )
        };
        let bulk_items = format!(
            "{}{}{}",
            bulk_item("Move selected", "btn-ghost", "requestBulkMove"),
            bulk_item("Download zip", "btn-ghost", "requestBulkDownload"),
            bulk_item(
                "Delete selected",
                "btn-ghost text-error",
                "requestBulkDelete"
            ),
        );
        let bulk_actions = table_button_bulk_actions(html! {
            (PreEscaped(bulk_items))
        });
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: filter_panel,
                ..Default::default()
            }))
            (download_btn)
            (bulk_actions)
            (button_modal_form(ButtonModalForm {
                name: V_NODE_CREATE.form_name,
                href: &create_href,
                form_post_url: "",
                modal_uid: V_NODE_CREATE_MODAL.id(),
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination_push(
            V_NODE_TABLE,
            &self.path_and_query,
            self.items.number,
            self.items.num_pages,
            true,
        );
        // Bare table only — selection Alpine state lives outside so pagination swaps
        // do not reset checkboxes.
        data_table_filtered(
            V_NODE_TABLE,
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }

    fn body(&self) -> Markup {
        self.wrap_with_selection(self.render_table())
    }
}

impl RenderAppPane for VNodeListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(self.menu(), self.crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.body())
    }
}

impl RenderTemplate for VNodeListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Filesystem — Lariv",
            chrome,
            self.menu(),
            self.crumbs(),
            self.body(),
        )
    }
}

/// Directory or file picker option.
#[derive(Clone)]
pub struct VNodeOption {
    pub id: i64,
    pub name: String,
    pub is_directory: bool,
}

/// Shared picker modal for `ParentID`, `DestinationID`, and file foreign keys.
#[derive(Generic)]
pub struct VNodeSelectPage {
    pub items: ObjectList<VNodeOption>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub browse_base: String,
    pub parent_id: i64,
    pub up_parent_id: i64,
    pub current_path: String,
    pub exclude_id: i64,
    pub only_directories: bool,
    /// Files and folders can both be chosen. Folders still have an Open button.
    pub select_any: bool,
    /// Many-to-many mode: toggle selection without closing the modal.
    pub multi: bool,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

fn form_hx_get_picker_url(key: SwapKey, url: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "GET")
        .set("hx-get", url)
        .set("hx-target", key.selector())
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "false")
}
impl VNodeSelectPage {
    fn is_file_select(&self) -> bool {
        self.browse_base.contains("file-select")
    }

    fn is_move_select(&self) -> bool {
        self.browse_base.contains("move-select")
    }

    fn title(&self) -> &'static str {
        if self.select_any {
            "Select a file or folder"
        } else if self.only_directories {
            "Select a Folder"
        } else {
            "Select a File"
        }
    }

    fn browse_route_url(&self, parent_id: i64) -> String {
        let target = (!self.target_input.is_empty()).then_some(self.target_input.as_str());
        let exclude = (self.exclude_id != 0).then_some(self.exclude_id);
        let multi = self.multi.then_some(1u8);
        if self.select_any {
            return if parent_id == 0 {
                VNodeNodeSelectRouteTag
                    .with_query()
                    .query_opt("target_input", target)
                    .query_opt("exclude_id", exclude)
                    .query_opt("multi", multi)
                    .build()
            } else {
                VNodeNodeSelectInRouteTag::new(parent_id)
                    .with_query()
                    .query_opt("target_input", target)
                    .query_opt("exclude_id", exclude)
                    .query_opt("multi", multi)
                    .build()
            };
        }
        match (self.is_move_select(), self.is_file_select(), parent_id) {
            (true, _, 0) => VNodeMoveSelectRouteTag
                .with_query()
                .query_opt("target_input", target)
                .query_opt("exclude_id", exclude)
                .query_opt("multi", multi)
                .build(),
            (true, _, pid) => VNodeMoveSelectInRouteTag::new(pid)
                .with_query()
                .query_opt("target_input", target)
                .query_opt("exclude_id", exclude)
                .query_opt("multi", multi)
                .build(),
            (_, true, 0) => VNodeFileSelectRouteTag
                .with_query()
                .query_opt("target_input", target)
                .query_opt("exclude_id", exclude)
                .query_opt("multi", multi)
                .build(),
            (_, true, pid) => VNodeFileSelectInRouteTag::new(pid)
                .with_query()
                .query_opt("target_input", target)
                .query_opt("exclude_id", exclude)
                .query_opt("multi", multi)
                .build(),
            (_, _, 0) => VNodeSelectRouteTag
                .with_query()
                .query_opt("target_input", target)
                .query_opt("exclude_id", exclude)
                .query_opt("multi", multi)
                .build(),
            (_, _, pid) => VNodeSelectInRouteTag::new(pid)
                .with_query()
                .query_opt("target_input", target)
                .query_opt("exclude_id", exclude)
                .query_opt("multi", multi)
                .build(),
        }
    }

    fn browse_hx_attrs(&self, parent_id: i64) -> HtmlAttrs {
        HtmlAttrs::new()
            .set("hx-get", self.browse_route_url(parent_id))
            .set("hx-target", V_NODE_SELECT_MODAL.selector())
            .set("hx-swap", "outerHTML")
            .set("hx-push-url", "false")
    }

    fn browse_button(&self, parent_id: i64, label: &str) -> Markup {
        let attrs = self
            .browse_hx_attrs(parent_id)
            .set("type", "button")
            .set("class", "btn btn-ghost btn-xs")
            .set("@click.stop", "");
        maud::PreEscaped(format!("<button{}>{label}</button>", attrs.as_string()))
    }
}

impl RenderPickerSelect for VNodeSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "ParentID"
        } else {
            self.target_input.as_str()
        };
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Actions",
                label: "",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .items
            .items
            .iter()
            .filter(|n| n.id != self.exclude_id)
            .map(|n| {
                let selectable = self.select_any || self.only_directories || !n.is_directory;
                let attrs = if selectable {
                    if self.multi {
                        row_attr_select_multi(target, &n.id.to_string(), &n.name)
                    } else {
                        row_attr_select(target, &n.id.to_string(), &n.name)
                    }
                } else {
                    self.browse_hx_attrs(n.id)
                };
                let open = if n.is_directory {
                    self.browse_button(n.id, "Open")
                } else {
                    html! {}
                };
                TableRow {
                    attrs,
                    cells: vec![
                        field_text(FieldText {
                            value: &n.name,
                            classes: "",
                        }),
                        open,
                    ],
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let root_select = row_attr_select(target, "0", "Filesystem root");
        let create_href = modal_create_href_for_picker(
            &vnode_create_get_url(self.parent_id),
            V_NODE_CREATE.form_name,
            &self.target_input,
        );
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_picker_url(V_NODE_SELECT_MODAL,
                        &self.browse_route_url(self.parent_id),
                    ),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(self.target_input);
                        @if self.exclude_id != 0 {
                            input type="hidden" name="exclude_id" value=(self.exclude_id);
                        }
                        @if self.multi {
                            input type="hidden" name="multi" value="1";
                        }
                    },
                }),
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "",
                href: &create_href,
                form_post_url: "",
                modal_uid: V_NODE_CREATE_MODAL.id(),
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_picker_pagination(
            V_NODE_SELECT_MODAL,
            &self.path_and_query,
            self.items.number,
            self.items.num_pages,
        );
        let use_here_label = format!("Use here: {}", self.current_path);
        html! {
            @if self.parent_id != 0 {
                div class="mb-2 flex flex-wrap gap-2" {
                    (self.browse_button(self.up_parent_id, "Up"))
                    @if self.only_directories {
                        (maud::PreEscaped(format!(
                            r#"<div class="btn btn-sm btn-outline"{}>"#,
                            root_select.as_string()
                        )))
                        (use_here_label)
                        (maud::PreEscaped("</div>"))
                    }
                    @if self.select_any {
                        (maud::PreEscaped(format!(
                            r#"<div class="btn btn-sm btn-outline"{}>"#,
                            row_attr_select(
                                target,
                                &self.parent_id.to_string(),
                                &self.current_path
                            )
                            .as_string()
                        )))
                        "Select this folder"
                        (maud::PreEscaped("</div>"))
                    }
                }
            }
            (data_table_filtered(V_NODE_SELECT_TABLE,
                self.title(),
                actions,
                &headers,
                &rows,
                pagination,
                &self.path_and_query,
                &self.filter_errors,
            ))
        }
    }
}

impl RenderTemplate for VNodeSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(V_NODE_SELECT_MODAL).into_inner()
    }
}
