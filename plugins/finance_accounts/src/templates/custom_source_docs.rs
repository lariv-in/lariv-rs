use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, Crumb, DetailHeader, FieldLink, FieldText, FieldTextarea,
    FilterClauseView, ListFilterBuilder, ObjectList, ShellChrome, SwapKey, TableButtonFilter,
    TableColumnHeader, TableRow, breadcrumbs, button_link_route, button_modal_form, button_submit,
    column_sort_url, container_column, container_row, data_table_list, detail, detail_header,
    field_link, field_text, field_textarea, form, form_hx_get_route, form_hx_post_main_url,
    form_hx_post_route, form_hx_post_url, label, list_filter_builder, modal_keyed, sort_indicator,
    table_button_filter,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::modal_create_post_url;

use crate::{
    forms::{
        CustomSourceDocForm, CustomSourceDocFormField, CustomSourceDocIndexCreateForm,
        CustomSourceDocIndexCreateFormField,
    },
    keys::{
        CustomSourceDocCreateModalKey, CustomSourceDocDeleteModalKey,
        CustomSourceDocIndexCreateModalKey, CustomSourceDocIndexTableKey, CustomSourceDocTableKey,
    },
    routes::{
        CustomSourceDocCreateGetRouteTag, CustomSourceDocCreatePostRouteTag,
        CustomSourceDocDeleteGetRouteTag, CustomSourceDocDeletePostRouteTag,
        CustomSourceDocDetailRouteTag, CustomSourceDocEditGetRouteTag,
        CustomSourceDocIndexCreateGetRouteTag, CustomSourceDocIndexCreatePostRouteTag,
        CustomSourceDocIndexRouteTag, JournalCustomSourceDocsRouteTag, JournalDetailRouteTag,
        JournalEntryDetailRouteTag, JournalListRouteTag,
    },
};

use super::common::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs, render_pagination,
};
use super::journals::journal_detail_menu;

fn custom_source_doc_crumbs(
    journal_id: i64,
    journal_name: &str,
    current: &str,
    current_is_list: bool,
) -> Markup {
    let list_url = JournalListRouteTag.url();
    let journal_url = JournalDetailRouteTag::new(journal_id).url();
    let docs_url = JournalCustomSourceDocsRouteTag::new(journal_id).url();
    if current_is_list {
        breadcrumbs(&[
            Crumb {
                label: "Journals",
                href: Some(&list_url),
            },
            Crumb {
                label: journal_name,
                href: Some(&journal_url),
            },
            Crumb {
                label: current,
                href: None,
            },
        ])
    } else {
        breadcrumbs(&[
            Crumb {
                label: "Journals",
                href: Some(&list_url),
            },
            Crumb {
                label: journal_name,
                href: Some(&journal_url),
            },
            Crumb {
                label: "Custom source docs",
                href: Some(&docs_url),
            },
            Crumb {
                label: current,
                href: None,
            },
        ])
    }
}

#[derive(Clone, Generic)]
pub struct CustomSourceDocRow {
    pub id: i64,
    pub title: String,
    pub created_by: String,
    pub created_at: String,
    pub journal_entry_id: i64,
}

#[derive(Generic)]
pub struct CustomSourceDocListPage {
    pub journal_id: i64,
    pub journal_name: String,
    pub rows: Vec<CustomSourceDocRow>,
    pub can_create: bool,
}

impl CustomSourceDocListPage {
    fn menu(&self) -> Markup {
        journal_detail_menu(self.journal_id, &self.journal_name, "custom-source-docs")
    }

    fn table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Title",
                label: "Title",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "CreatedBy",
                label: "Created by",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "CreatedAt",
                label: "Created at",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "JournalEntry",
                label: "Journal entry",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .rows
            .iter()
            .map(|row| {
                let entry_href = JournalEntryDetailRouteTag::new(row.journal_entry_id).url();
                let entry_label = format!("#{}", row.journal_entry_id);
                TableRow {
                    attrs: lariv_core::components::row_attr_navigate_route(
                        CustomSourceDocDetailRouteTag::new(row.id),
                    ),
                    cells: vec![
                        field_text(FieldText {
                            value: &row.title,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &row.created_by,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &row.created_at,
                            classes: "",
                        }),
                        field_link(FieldLink {
                            href: &entry_href,
                            label: &entry_label,
                            classes: "",
                        }),
                    ],
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let actions = if self.can_create {
            let create_url = CustomSourceDocCreateGetRouteTag::new(self.journal_id).url();
            let create_post = CustomSourceDocCreatePostRouteTag::new(self.journal_id).path();
            button_modal_form(ButtonModalForm {
                name: "p_finance_accounts.CustomSourceDocForm",
                href: &create_url,
                form_post_url: &create_post,
                modal_uid: CustomSourceDocCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            })
        } else {
            html! {}
        };
        data_table_list::<CustomSourceDocTableKey>(
            "Custom source docs",
            actions,
            &headers,
            &rows,
            html! {},
        )
    }

    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (self.table())
                }))
            }))
        }
    }

    fn crumbs(&self) -> Markup {
        custom_source_doc_crumbs(
            self.journal_id,
            &self.journal_name,
            "Custom source docs",
            true,
        )
    }
}

impl RenderAppPane for CustomSourceDocListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_entity_sidebar_crumbs(self.menu(), self.crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(self.crumbs(), self.body())
    }
}

impl RenderTemplate for CustomSourceDocListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold_with_sidebar(
            "Custom source docs",
            chrome,
            self.menu(),
            self.crumbs(),
            self.body(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct CustomSourceDocLineRow {
    pub account_id: i64,
    pub account_label: String,
    pub debit: String,
    pub credit: String,
}

#[derive(Generic)]
pub struct CustomSourceDocDetailPage {
    pub id: i64,
    pub journal_id: i64,
    pub journal_name: String,
    pub title: String,
    pub content: String,
    pub created_by: String,
    pub created_at: String,
    pub journal_entry_id: i64,
    pub lines: Vec<CustomSourceDocLineRow>,
    pub can_mutate: bool,
}

impl CustomSourceDocDetailPage {
    fn menu(&self) -> Markup {
        journal_detail_menu(self.journal_id, &self.journal_name, "custom-source-docs")
    }

    fn lines_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Account",
                label: "Account",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Debit",
                label: "Debit",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Credit",
                label: "Credit",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .lines
            .iter()
            .map(|line| {
                let account_href = crate::routes::AccountDetailRouteTag::new(line.account_id).url();
                TableRow {
                    attrs: lariv_core::components::HtmlAttrs::new(),
                    cells: vec![
                        field_link(FieldLink {
                            href: &account_href,
                            label: &line.account_label,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &line.debit,
                            classes: "tabular-nums",
                        }),
                        field_text(FieldText {
                            value: &line.credit,
                            classes: "tabular-nums",
                        }),
                    ],
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        data_table_list::<CustomSourceDocTableKey>("Lines", html! {}, &headers, &rows, html! {})
    }

    fn body(&self) -> Markup {
        let entry_href = JournalEntryDetailRouteTag::new(self.journal_entry_id).url();
        let entry_label = format!("#{}", self.journal_entry_id);
        let delete_url = CustomSourceDocDeleteGetRouteTag::new(self.id).url();
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.title,
                        actions: if self.can_mutate {
                            html! {
                                (container_row("flex gap-2", html! {
                                    (button_link_route(
                                        CustomSourceDocEditGetRouteTag::new(self.id),
                                        "Edit",
                                        "btn-outline",
                                    ))
                                    (button_modal_form(ButtonModalForm {
                                        label: "Delete",
                                        icon_name: Some("trash"),
                                        name: "p_finance_accounts.CustomSourceDocDeleteForm",
                                        href: &delete_url,
                                        form_post_url: &delete_url,
                                        modal_uid: CustomSourceDocDeleteModalKey::ID,
                                        classes: "btn-error",
                                        ..Default::default()
                                    }))
                                }))
                            }
                        } else {
                            html! {}
                        },
                    }))
                    (label("Content", field_textarea(FieldTextarea { value: &self.content, classes: "" })))
                    (label("Created by", field_text(FieldText { value: &self.created_by, classes: "" })))
                    (label("Created at", field_text(FieldText { value: &self.created_at, classes: "" })))
                    (label(
                        "Journal entry",
                        field_link(FieldLink {
                            href: &entry_href,
                            label: &entry_label,
                            classes: "",
                        }),
                    ))
                    div class="mt-6" {
                        (self.lines_table())
                    }
                }))
            }))
        }
    }

    fn crumbs(&self) -> Markup {
        custom_source_doc_crumbs(self.journal_id, &self.journal_name, &self.title, false)
    }
}

impl RenderAppPane for CustomSourceDocDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_entity_sidebar_crumbs(self.menu(), self.crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(self.crumbs(), self.body())
    }
}

impl RenderTemplate for CustomSourceDocDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold_with_sidebar(
            "Custom source doc",
            chrome,
            self.menu(),
            self.crumbs(),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct CustomSourceDocFormPage {
    pub journal_id: i64,
    pub journal_name: String,
    pub heading: String,
    pub post_path: String,
    pub title: String,
    pub content: String,
    pub lines: String,
    pub error: String,
    pub can_save: bool,
}

impl CustomSourceDocFormPage {
    fn menu(&self) -> Markup {
        journal_detail_menu(self.journal_id, &self.journal_name, "custom-source-docs")
    }

    fn body(&self) -> Markup {
        let inputs = CustomSourceDocForm::render_inputs(
            &FormCtx::form::<CustomSourceDocForm>(CsrfToken::current())
                .value(CustomSourceDocFormField::Title, &self.title)
                .value(CustomSourceDocFormField::Content, &self.content)
                .value(CustomSourceDocFormField::Lines, &self.lines),
        );
        html! {
            (detail(html! {
                (container_column("", html! {
                    @if self.can_save {
                        (form(&CsrfToken::current(), lariv_core::components::FormOpts {
                            title: &self.heading,
                            subtitle: &self.journal_name,
                            attrs: form_hx_post_main_url(&self.post_path),
                            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                            inputs,
                            actions: html! {
                                (container_row("flex gap-2 mt-2", html! {
                                    (button_submit(ButtonSubmit {
                                        label: "Save",
                                        classes: "btn-primary",
                                        ..Default::default()
                                    }))
                                }))
                            },
                            ..Default::default()
                        }))
                    } @else {
                        div class="alert alert-warning" {
                            @if self.error.is_empty() {
                                "This journal is immutable."
                            } @else {
                                (self.error)
                            }
                        }
                    }
                }))
            }))
        }
    }

    fn crumbs(&self) -> Markup {
        custom_source_doc_crumbs(self.journal_id, &self.journal_name, &self.heading, false)
    }
}

impl RenderAppPane for CustomSourceDocFormPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_entity_sidebar_crumbs(self.menu(), self.crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(self.crumbs(), self.body())
    }
}

impl RenderTemplate for CustomSourceDocFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold_with_sidebar(
            "Custom source doc",
            chrome,
            self.menu(),
            self.crumbs(),
            self.body(),
        )
    }
}

#[derive(Clone, Generic)]
pub struct CustomSourceDocDeleteItem {
    pub kind: String,
    pub label: String,
    pub url: String,
}

#[derive(Generic)]
pub struct CustomSourceDocDeleteModalPage {
    pub id: i64,
    pub items: Vec<CustomSourceDocDeleteItem>,
    pub can_delete: bool,
    pub error: Option<String>,
}

impl RenderTemplate for CustomSourceDocDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let body = if !self.can_delete {
            let err = self.error.as_deref().unwrap_or("Delete is unavailable.");
            html! {
                h3 class="font-bold text-lg" { "Delete unavailable" }
                div class="alert alert-error my-2 text-sm" { (err) }
            }
        } else {
            let item_list = html! {
                p class="my-2" { "The following items will be permanently deleted:" }
                (container_column("", html! {
                    @for item in &self.items {
                        (label(
                            &item.kind,
                            if item.url.is_empty() {
                                field_text(FieldText { value: &item.label, classes: "" })
                            } else {
                                field_link(FieldLink {
                                    href: &item.url,
                                    label: &item.label,
                                    classes: "",
                                })
                            },
                        ))
                    }
                }))
            };
            form(
                &CsrfToken::current(),
                lariv_core::components::FormOpts {
                    title: "Delete custom source doc",
                    subtitle: "This cannot be undone.",
                    attrs: form_hx_post_route::<CustomSourceDocDeleteModalKey, _>(
                        CustomSourceDocDeletePostRouteTag::new(self.id),
                    ),
                    form_error: self.error.as_deref(),
                    inputs: item_list,
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Delete",
                                classes: "btn-error",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                },
            )
        };
        modal_keyed::<CustomSourceDocDeleteModalKey>("", body)
    }
}

#[derive(Clone)]
pub struct CustomSourceDocIndexRow {
    pub id: i64,
    pub title: String,
    pub journal_id: i64,
    pub journal_name: String,
    pub created_by: String,
    pub created_at: String,
    pub journal_entry_id: i64,
}

fn custom_source_doc_index_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Custom source docs",
        href: None,
    }])
}

fn custom_source_doc_index_filter_form(page: &CustomSourceDocIndexPage) -> Markup {
    list_filter_builder(ListFilterBuilder {
        fields: &page.fields,
        rows: &page.filters,
        page_size: page.page_size,
        sort: &page.sort,
        attrs: form_hx_get_route::<CustomSourceDocIndexTableKey, CustomSourceDocIndexRouteTag>(
            CustomSourceDocIndexRouteTag,
        ),
        apply_label: "Apply Filters",
        extra: html! {},
    })
}

#[derive(Generic)]
pub struct CustomSourceDocIndexPage {
    pub docs: ObjectList<CustomSourceDocIndexRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub can_create: bool,
    pub page_size: u32,
}

impl CustomSourceDocIndexPage {
    pub fn render_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let created_sort = column_sort_url(&self.path_and_query, "CreatedAt", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let created_label = format!("Created at{}", sort_indicator(&self.sort, "CreatedAt"));
        let headers = [
            TableColumnHeader {
                key: "Title",
                label: &title_label,
                sort_url: Some(&title_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Journal",
                label: "Journal",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "CreatedBy",
                label: "Created by",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "CreatedAt",
                label: &created_label,
                sort_url: Some(&created_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "JournalEntry",
                label: "Journal entry",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .docs
            .items
            .iter()
            .map(|row| {
                let journal_href = JournalDetailRouteTag::new(row.journal_id).url();
                let entry_href = JournalEntryDetailRouteTag::new(row.journal_entry_id).url();
                let entry_label = format!("#{}", row.journal_entry_id);
                let journal_cell = if row.journal_id > 0 {
                    field_link(FieldLink {
                        href: &journal_href,
                        label: &row.journal_name,
                        classes: "",
                    })
                } else {
                    field_text(FieldText {
                        value: &row.journal_name,
                        classes: "",
                    })
                };
                TableRow {
                    attrs: lariv_core::components::row_attr_navigate_route(
                        CustomSourceDocDetailRouteTag::new(row.id),
                    ),
                    cells: vec![
                        field_text(FieldText {
                            value: &row.title,
                            classes: "",
                        }),
                        journal_cell,
                        field_text(FieldText {
                            value: &row.created_by,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &row.created_at,
                            classes: "",
                        }),
                        field_link(FieldLink {
                            href: &entry_href,
                            label: &entry_label,
                            classes: "",
                        }),
                    ],
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: custom_source_doc_index_filter_form(self),
                ..Default::default()
            }))
        };
        if self.can_create {
            let create_url = CustomSourceDocIndexCreateGetRouteTag.url();
            let create_post = CustomSourceDocIndexCreatePostRouteTag.path();
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_accounts.CustomSourceDocIndexCreateForm",
                    href: &create_url,
                    form_post_url: &create_post,
                    modal_uid: CustomSourceDocIndexCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<CustomSourceDocIndexTableKey>(
            &self.path_and_query,
            self.docs.number,
            self.docs.num_pages,
        );
        super::data_table_with_filter_errors::<CustomSourceDocIndexTableKey>(
            "Custom source docs",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for CustomSourceDocIndexPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &self.path_and_query,
            custom_source_doc_index_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(custom_source_doc_index_crumbs(), self.body())
    }
}

impl RenderTemplate for CustomSourceDocIndexPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Custom source docs",
            chrome,
            custom_source_doc_index_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct CustomSourceDocCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub journal_id: i64,
    pub journal_name: String,
    pub title: String,
    pub content: String,
    pub lines: String,
    pub error: String,
    pub can_save: bool,
}

impl RenderTemplate for CustomSourceDocCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_accounts.CustomSourceDocForm"
        } else {
            self.form_name.as_str()
        };
        let body = if self.can_save {
            let inputs = CustomSourceDocForm::render_inputs(
                &FormCtx::form::<CustomSourceDocForm>(CsrfToken::current())
                    .value(CustomSourceDocFormField::Title, &self.title)
                    .value(CustomSourceDocFormField::Content, &self.content)
                    .value(CustomSourceDocFormField::Lines, &self.lines),
            );
            form(
                &CsrfToken::current(),
                lariv_core::components::FormOpts {
                    title: "New custom source doc",
                    subtitle: &self.journal_name,
                    attrs: form_hx_post_url::<CustomSourceDocCreateModalKey>(
                        &modal_create_post_url(
                            CustomSourceDocCreatePostRouteTag::new(self.journal_id),
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs,
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                },
            )
        } else {
            let err = if self.error.is_empty() {
                "This journal is immutable."
            } else {
                self.error.as_str()
            };
            html! {
                h3 class="font-bold text-lg" { "New custom source doc" }
                div class="alert alert-warning my-2 text-sm" { (err) }
            }
        };
        modal_keyed::<CustomSourceDocCreateModalKey>("", body)
    }
}

#[derive(Generic)]
pub struct CustomSourceDocIndexCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub journal_id: String,
    pub journal_display: String,
    pub title: String,
    pub content: String,
    pub lines: String,
    pub error: String,
}

impl RenderTemplate for CustomSourceDocIndexCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_accounts.CustomSourceDocIndexCreateForm"
        } else {
            self.form_name.as_str()
        };
        let inputs = CustomSourceDocIndexCreateForm::render_inputs(
            &FormCtx::form::<CustomSourceDocIndexCreateForm>(CsrfToken::current())
                .value(
                    CustomSourceDocIndexCreateFormField::JournalId,
                    &self.journal_id,
                )
                .display(
                    CustomSourceDocIndexCreateFormField::JournalId,
                    &self.journal_display,
                )
                .value(CustomSourceDocIndexCreateFormField::Title, &self.title)
                .value(CustomSourceDocIndexCreateFormField::Content, &self.content)
                .value(CustomSourceDocIndexCreateFormField::Lines, &self.lines),
        );
        modal_keyed::<CustomSourceDocIndexCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                lariv_core::components::FormOpts {
                    title: "New custom source doc",
                    subtitle: "",
                    attrs: form_hx_post_url::<CustomSourceDocIndexCreateModalKey>(
                        &modal_create_post_url(
                            CustomSourceDocIndexCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs,
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}
