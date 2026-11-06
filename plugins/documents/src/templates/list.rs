use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::picker::RenderPickerSelect;
use lariv_core::components::{
    DataTable, DataTableDisplay, FieldText, FilterClauseView, ListFilterBuilder, ObjectList,
    ShellChrome, SwapKey, TableButtonFilter, TableColumnHeader, TableListContent, TableRow,
    data_table, data_table_list_refresh, field_text, filter_error_banner, form_hx_get_route,
    list_filter_builder, modal_keyed, render_pagination, render_picker_pagination,
    row_attr_navigate_route, row_attr_select, sort_indicator, table_button_columns,
    table_button_filter, table_create_button, table_list_content,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::keys::{DOCUMENT_SELECT_MODAL, DOCUMENT_SELECT_TABLE, DOCUMENT_TABLE};
use crate::routes::{DocumentDefaultRouteTag, DocumentDetailRouteTag};

use super::create::DOCUMENT_CREATE;
use super::{app_scaffold, document_menu, documents_list_crumbs, scaffold_main, scaffold_pane};

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

#[derive(Clone)]
pub struct DocumentRow {
    pub id: i64,
    pub document_type: String,
    pub name: String,
    pub number: String,
}

#[derive(Generic)]
pub struct DocumentListPage {
    pub documents: ObjectList<DocumentRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl DocumentListPage {
    pub fn render_table(&self) -> Markup {
        let type_sort =
            lariv_core::components::column_sort_url(&self.path_and_query, "Type", &self.sort);
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: "Name",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Number",
                label: "Number",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .documents
            .items
            .iter()
            .map(|doc| TableRow {
                attrs: row_attr_navigate_route(DocumentDetailRouteTag::new(doc.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &doc.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &doc.document_type,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &doc.number,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(
                        DocumentDefaultRouteTag,
                    ),
                    apply_label: "Apply Filters",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::DocumentsMutate>(),
        ) {
            actions = html! {
                (actions)
                (table_create_button(DOCUMENT_TABLE, DOCUMENT_CREATE,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination(
            DOCUMENT_TABLE,
            &self.path_and_query,
            self.documents.number,
            self.documents.num_pages,
        );
        data_table_filtered(
            DOCUMENT_TABLE,
            "Documents",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderAppPane for DocumentListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            document_menu(&self.path_and_query),
            documents_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(documents_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for DocumentListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Documents — Lariv",
            chrome,
            document_menu(&self.path_and_query),
            documents_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone)]
pub struct DocumentOption {
    pub id: i64,
    pub label: String,
}

#[derive(Generic)]
pub struct DocumentSelectPage {
    pub documents: ObjectList<DocumentOption>,
    pub target_input: String,
    pub path_and_query: String,
    pub page_size: u32,
    pub title: String,
}

impl RenderPickerSelect for DocumentSelectPage {
    fn render_table(&self) -> Markup {
        let rows: Vec<TableRow> = self
            .documents
            .items
            .iter()
            .map(|doc| TableRow {
                attrs: row_attr_select(&self.target_input, &doc.id.to_string(), &doc.label),
                cells: vec![field_text(FieldText {
                    value: &doc.label,
                    classes: "",
                })],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let _ = self.page_size;
        let title = format!("Select {}", self.title);
        data_table_list_refresh(
            DOCUMENT_SELECT_TABLE,
            &title,
            html! {},
            &[TableColumnHeader {
                key: "Document",
                label: "Document",
                sort_url: None,
                push_url: false,
            }],
            &rows,
            {
                render_picker_pagination(
                    DOCUMENT_SELECT_TABLE,
                    &self.path_and_query,
                    self.documents.number,
                    self.documents.num_pages,
                )
            },
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for DocumentSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(DOCUMENT_SELECT_MODAL, "", self.render_table())
    }
}
