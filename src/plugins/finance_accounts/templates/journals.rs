use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonDeletePost, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation,
        FieldLink, FieldText, FieldTitle, FormOpts, ObjectList, ShellChrome, SwapKey,
        TableButtonFilter, TableColumnHeader, TableRow, breadcrumbs, button_clear,
        button_delete_post_route, button_modal_form, button_submit, column_sort_url,
        container_column, container_row, data_table_list, data_table_list_refresh,
        delete_confirmation, detail, field_link, field_text, field_title, form,
        form_hx_get_picker_route, form_hx_get_route, form_hx_post_redirect, form_hx_post_url,
        label_inline, modal_keyed, row_attr_navigate_route, row_attr_select, sort_indicator,
        table_button_filter,
    },
    html_form::{FormCtx, HtmlForm},
    picker::RenderPickerSelect,
    template::{RenderAppPane, RenderTemplate},
    web::{modal_create_post_url, modal_edit_post_url},
};

use crate::plugins::finance_accounts::{entities::journal, forms::{
        JournalEntryForm, JournalEntryFormField, JournalFilterForm, JournalFilterFormField,
        JournalCreateForm, JournalCreateFormField, JournalForm, JournalFormField,
    }, keys::{
        JournalCreateModalKey, JournalEditModalKey, JournalEntryCreateModalKey,
        JournalEntrySelectModalKey, JournalEntrySelectTableKey, JournalSelectModalKey,
        JournalSelectTableKey, JournalTableKey,
    }, routes::{
        JournalCreateGetRouteTag, JournalCreatePostRouteTag, JournalDeletePostRouteTag,
        JournalDetailRouteTag, JournalEditGetRouteTag, JournalEditPostRouteTag,
        JournalEntryCreateGetRouteTag, JournalEntryCreatePostRouteTag,
        JournalEntryDeletePostRouteTag, JournalEntryDetailRouteTag,
        JournalListRouteTag, JournalSelectRouteTag,
    }};

use super::common::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs, render_pagination,
    render_picker_pagination,
};
use crate::plugins::finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};

fn journals_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Journals",
        href: None,
    }])
}

fn journal_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = JournalListRouteTag.url();
    let detail_url = JournalDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Journals",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Journals",
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

fn journal_entry_crumbs(journal_id: i64, journal_label: &str, entry_label: &str) -> Markup {
    let list_url = JournalListRouteTag.url();
    let journal_url = JournalDetailRouteTag::new(journal_id).url();
    breadcrumbs(&[
        Crumb {
            label: "Journals",
            href: Some(&list_url),
        },
        Crumb {
            label: journal_label,
            href: Some(&journal_url),
        },
        Crumb {
            label: entry_label,
            href: None,
        },
    ])
}

fn journal_detail_menu(id: i64, name: &str) -> Markup {
    let menu_title = format!("Journal: {name}");
    let detail_url = JournalDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Journal Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(menu_title, &nav, None, html! {})
}

fn journal_entry_detail_menu(entry_id: i64, _journal_id: i64, active: &str) -> Markup {
    detail_sidebar_menu(
        format!("Journal entry #{entry_id}"),
        &[DetailMenuNavItem {
            title: "Entry Detail",
            url: JournalEntryDetailRouteTag::new(entry_id).url(),
            active: active == "detail",
        }],
        None,
        html! {},
    )
}

#[derive(Clone)]
pub struct JournalRow {
    pub id: i64,
    pub name: String,
    pub is_active: bool,
    pub currency_label: String,
    pub journal_type: String,
}

#[derive(Clone)]
pub struct JournalEntryRow {
    pub id: i64,
    pub datetime: String,
    pub source_doc_label: String,
    pub source_doc_instance_name: String,
    pub source_doc_url: String,
    pub journal_name: String,
    pub amount: String,
    pub label: String,
}

fn source_doc_instance_cell(name: &str, url: &str) -> Markup {
    if url.is_empty() {
        field_text(FieldText { value: name, classes: "" })
    } else {
        field_link(FieldLink {
            href: url,
            label: name,
            classes: "",
        })
    }
}

#[derive(Clone)]
pub struct JournalEntryItemRow {
    pub datetime: String,
    pub account_label: String,
    pub amount: String,
}

fn journal_filter_form(
    name: &str,
    is_active: bool,
    currency_id: &str,
    journal_type: &str,
) -> Markup {
    let jt_choices = crate::plugins::finance_accounts::forms::journal_type_filter_choices();
    form(FormOpts {
        attrs: form_hx_get_route::<JournalTableKey, JournalListRouteTag>(JournalListRouteTag),
        inputs: JournalFilterForm::render_inputs(
            &FormCtx::form::<JournalFilterForm>()
                .value(JournalFilterFormField::Name, name)
                .value(JournalFilterFormField::IsActive, if is_active { "on" } else { "" })
                .value(JournalFilterFormField::CurrencyId, currency_id)
                .value(JournalFilterFormField::JournalType, journal_type)
                .choices(JournalFilterFormField::JournalType, &jt_choices),
        ),
        actions: html! {
            (container_row("flex gap-2", html! {
                (button_submit(ButtonSubmit { label: "Apply Filters", ..Default::default() }))
                (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
            }))
        },
        ..Default::default()
    })
}

#[derive(Generic)]
pub struct JournalListPage {
    pub journals: ObjectList<JournalRow>,
    pub filter_name: String,
    pub filter_is_active: bool,
    pub filter_currency_id: String,
    pub filter_journal_type: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl JournalListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let active_sort = column_sort_url(&self.path_and_query, "Active", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let active_label = format!("Active{}", sort_indicator(&self.sort, "Active"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Active",
                label: &active_label,
                sort_url: Some(&active_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Currency",
                label: "Currency",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .journals
            .items
            .iter()
            .map(|j| {
                let active = if j.is_active { "Yes" } else { "No" };
                TableRow {
                    attrs: row_attr_navigate_route(JournalDetailRouteTag::new(j.id)),
                    cells: vec![
                        field_text(FieldText { value: &j.name, classes: "" }),
                        field_text(FieldText { value: active, classes: "" }),
                        field_text(FieldText { value: &j.currency_label, classes: "" }),
                        field_text(FieldText { value: &j.journal_type, classes: "" }),
                    ],
                }
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: journal_filter_form(
                    &self.filter_name,
                    self.filter_is_active,
                    &self.filter_currency_id,
                    &self.filter_journal_type,
                ),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_accounts.JournalCreateForm",
                    href: &JournalCreateGetRouteTag.url(),
                    form_post_url: &JournalCreateGetRouteTag.path(),
                    modal_uid: JournalCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<JournalTableKey>(
            &self.path_and_query,
            self.journals.number,
            self.journals.num_pages,
        );
        data_table_list_refresh::<JournalTableKey>(
            "Journals",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for JournalListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, journals_list_crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(journals_list_crumbs(), self.body())
    }
}

impl RenderTemplate for JournalListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Journals",
            chrome,
            journals_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct JournalDetailPage {
    pub id: i64,
    pub name: String,
    pub is_active: bool,
    pub is_mutable: bool,
    pub currency_id: i64,
    pub currency_label: String,
    pub journal_type: String,
    pub entries: ObjectList<JournalEntryRow>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl JournalDetailPage {
    pub fn render_entries_table(&self) -> Markup {
        self.entries_table()
    }

    fn entries_table(&self) -> Markup {
        let id_sort = column_sort_url(&self.path_and_query, "ID", &self.sort);
        let datetime_sort = column_sort_url(&self.path_and_query, "DateTime", &self.sort);
        let id_label = format!("ID{}", sort_indicator(&self.sort, "ID"));
        let datetime_label = format!("Date & time{}", sort_indicator(&self.sort, "DateTime"));
        let headers = [
            TableColumnHeader {
                key: "ID",
                label: &id_label,
                sort_url: Some(&id_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "DateTime",
                label: &datetime_label,
                sort_url: Some(&datetime_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "SourceDocumentType",
                label: "Source document type",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "SourceDocument",
                label: "Source document",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Amount",
                label: "Amount",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .entries
            .items
            .iter()
            .map(|e| TableRow {
                attrs: row_attr_navigate_route(JournalEntryDetailRouteTag::new(e.id)),
                cells: vec![
                    field_text(FieldText { value: &e.id.to_string(), classes: "" }),
                    field_text(FieldText { value: &e.datetime, classes: "" }),
                    field_text(FieldText { value: &e.source_doc_label, classes: "" }),
                    source_doc_instance_cell(&e.source_doc_instance_name, &e.source_doc_url),
                    field_text(FieldText { value: &e.amount, classes: "" }),
                ],
            })
            .collect();
        let mut actions = html! {};
        if self.can_edit {
            actions = html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_accounts.JournalEntryCreateForm",
                    href: &JournalEntryCreateGetRouteTag::new(self.id).url(),
                    form_post_url: &JournalEntryCreateGetRouteTag::new(self.id).path(),
                    modal_uid: JournalEntryCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        data_table_list_refresh::<JournalTableKey>(
            "Journal entries",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        let active = if self.is_active { "Active" } else { "Inactive" };
        let mutable = if self.is_mutable { "Mutable" } else { "Immutable" };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (field_text(FieldText {
                        value: &format!("{active} · {mutable} · {}", self.journal_type),
                        classes: "text-base-content/70",
                    }))
                    (label_inline("Currency", field_text(FieldText { value: &self.currency_label, classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_finance_accounts.JournalEditForm",
                                href: &JournalEditGetRouteTag::new(self.id).url(),
                                form_post_url: &JournalEditPostRouteTag::new(self.id).path(),
                                modal_uid: JournalEditModalKey::ID,
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                        }))
                    }
                    div class="mt-6" {
                        (self.entries_table())
                    }
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        journal_detail_menu(self.id, &self.name)
    }
}

impl RenderAppPane for JournalDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = journal_crumbs(self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(journal_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for JournalDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = journal_crumbs(self.id, &self.name, None);
        app_scaffold_with_sidebar("Journal", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct JournalEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub is_active: bool,
    pub is_mutable: bool,
    pub currency_id: String,
    pub currency_display: String,
    pub journal_type: String,
    pub error: String,
}

impl JournalEditModalPage {
    pub fn from_model(j: &journal::Model, form_name: String, currency_display: String) -> Self {
        Self {
            id: j.id,
            form_name,
            name: j.name.clone(),
            is_active: j.is_active,
            is_mutable: j.is_mutable,
            currency_id: j.currency_id.to_string(),
            currency_display,
            journal_type: j.journal_type.to_string(),
            error: String::new(),
        }
    }
}

impl RenderTemplate for JournalEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<JournalEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit journal" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<JournalEditModalKey>(&modal_edit_post_url(
                        JournalEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: JournalForm::render_inputs(
                        &FormCtx::form::<JournalForm>()
                            .value(JournalFormField::Name, &self.name)
                            .value(JournalFormField::IsActive, if self.is_active { "on" } else { "" })
                            .value(JournalFormField::IsMutable, if self.is_mutable { "on" } else { "" })
                            .value(JournalFormField::CurrencyId, &self.currency_id)
                            .display(JournalFormField::CurrencyId, &self.currency_display)
                            .value(JournalFormField::JournalType, &self.journal_type)
                            .choices(
                                JournalFormField::JournalType,
                                &crate::plugins::finance_accounts::forms::journal_type_choices(),
                            ),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_delete_post_route(
                            JournalDeletePostRouteTag::new(self.id),
                            ButtonDeletePost {
                                label: "Delete",
                                confirm: "Permanently delete this journal?",
                                classes: "btn-error",
                            },
                        ))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct JournalCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub is_active: bool,
    pub currency_id: String,
    pub currency_display: String,
    pub journal_type: String,
    pub error: String,
}

impl RenderTemplate for JournalCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_accounts.JournalCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<JournalCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Journal",
                subtitle: "Create a new journal",
                attrs: form_hx_post_url::<JournalCreateModalKey>(
                    &modal_create_post_url(
                        JournalCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    ),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: JournalCreateForm::render_inputs(
                    &FormCtx::form::<JournalCreateForm>()
                        .value(JournalCreateFormField::Name, &self.name)
                        .value(JournalCreateFormField::IsActive, if self.is_active { "on" } else { "" })
                        .value(JournalCreateFormField::CurrencyId, &self.currency_id)
                        .display(JournalCreateFormField::CurrencyId, &self.currency_display)
                        .value(JournalCreateFormField::JournalType, &self.journal_type)
                        .choices(
                            JournalCreateFormField::JournalType,
                            &crate::plugins::finance_accounts::forms::journal_type_choices(),
                        ),
                ),
                actions: html! {
                    (container_row("flex justify-end gap-2 mt-2", html! {
                        (button_submit(ButtonSubmit {
                            label: "Save Journal",
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
pub struct JournalSelectPage {
    pub journals: ObjectList<JournalRow>,
    pub filter_name: String,
    pub filter_is_active: bool,
    pub filter_currency_id: String,
    pub filter_journal_type: String,
    pub sort: String,
    pub path_and_query: String,
    pub target_input: String,
}

impl RenderPickerSelect<JournalSelectTableKey, JournalSelectModalKey> for JournalSelectPage {
    fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Currency",
                label: "Currency",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .journals
            .items
            .iter()
            .map(|j| TableRow {
                attrs: row_attr_select(&self.target_input, &j.id.to_string(), &j.name),
                cells: vec![
                    field_text(FieldText { value: &j.name, classes: "" }),
                    field_text(FieldText { value: &j.currency_label, classes: "" }),
                    field_text(FieldText { value: &j.journal_type, classes: "" }),
                ],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_picker_route::<
                        JournalSelectTableKey,
                        JournalSelectModalKey,
                        JournalSelectRouteTag,
                    >(JournalSelectRouteTag),
                    inputs: html! {
                        (JournalFilterForm::render_inputs(
                            &FormCtx::form::<JournalFilterForm>()
                                .value(JournalFilterFormField::Name, &self.filter_name)
                                .value(
                                    JournalFilterFormField::IsActive,
                                    if self.filter_is_active { "on" } else { "" },
                                )
                                .value(JournalFilterFormField::CurrencyId, &self.filter_currency_id)
                                .value(JournalFilterFormField::JournalType, &self.filter_journal_type)
                                .choices(
                                    JournalFilterFormField::JournalType,
                                    &crate::plugins::finance_accounts::forms::journal_type_filter_choices(),
                                ),
                        ))
                        input type="hidden" name="target_input" value=(self.target_input) {}
                    },
                    actions: html! {
                        (container_row("flex gap-2", html! {
                            (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                            (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
                        }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        let pagination = render_picker_pagination::<JournalSelectModalKey>(
            &self.path_and_query,
            self.journals.number,
            self.journals.num_pages,
        );
        data_table_list_refresh::<JournalSelectTableKey>(
            "Select Journal",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for JournalSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Generic)]
pub struct JournalEntryCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub journal_id: i64,
    pub journal_name: String,
    pub datetime: String,
    pub source_doc_id: String,
    pub source_doc_display: String,
    pub error: String,
}

impl JournalEntryCreateModalPage {
    pub fn new(
        form_name: String,
        refresh_table: String,
        journal_id: i64,
        journal_name: String,
        datetime: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            journal_id,
            journal_name,
            datetime,
            source_doc_id: String::new(),
            source_doc_display: String::new(),
            error: String::new(),
        }
    }
}

impl RenderTemplate for JournalEntryCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_accounts.JournalEntryCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<JournalEntryCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Journal Entry",
                subtitle: &format!("New entry — {}", self.journal_name),
                attrs: form_hx_post_url::<JournalEntryCreateModalKey>(
                    &modal_create_post_url(
                        JournalEntryCreatePostRouteTag::new(self.journal_id),
                        form_name,
                        &self.refresh_table,
                    ),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: JournalEntryForm::render_inputs(
                    &FormCtx::form::<JournalEntryForm>()
                        .value(JournalEntryFormField::Datetime, &self.datetime)
                        .value(JournalEntryFormField::SourceDocId, &self.source_doc_id)
                        .display(JournalEntryFormField::SourceDocId, &self.source_doc_display),
                ),
                actions: html! {
                    (container_row("flex justify-end gap-2 mt-2", html! {
                        (button_submit(ButtonSubmit {
                            label: "Save Entry",
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
pub struct JournalEntryDetailPage {
    pub id: i64,
    pub datetime: String,
    pub journal_id: i64,
    pub journal_label: String,
    pub source_doc_label: String,
    pub source_doc_instance_name: String,
    pub source_doc_url: String,
    pub items: Vec<JournalEntryItemRow>,
    pub can_delete: bool,
}

impl JournalEntryDetailPage {
    fn items_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {  key: "DateTime",label: "Date & time", sort_url: None, push_url: false },
            TableColumnHeader {  key: "Account",label: "Account", sort_url: None, push_url: false },
            TableColumnHeader {  key: "Amount",label: "Amount", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self
            .items
            .iter()
            .map(|item| TableRow {
                attrs: crate::components::HtmlAttrs::new(),
                cells: vec![
                    field_text(FieldText { value: &item.datetime, classes: "" }),
                    field_text(FieldText { value: &item.account_label, classes: "" }),
                    field_text(FieldText { value: &item.amount, classes: "tabular-nums" }),
                ],
            })
            .collect();
        data_table_list::<JournalTableKey>(
            "Line items",
            html! {},
            &headers,
            &rows,
            html! {},
        )
    }

    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &format!("Journal entry #{}", self.id), classes: "" }))
                    (field_text(FieldText {
                        value: &self.datetime,
                        classes: "text-base-content/70",
                    }))
                    (label_inline("Journal", field_text(FieldText { value: &self.journal_label, classes: "" })))
                    (label_inline("Source document type", field_text(FieldText { value: &self.source_doc_label, classes: "" })))
                    (label_inline(
                        "Source document",
                        source_doc_instance_cell(&self.source_doc_instance_name, &self.source_doc_url),
                    ))
                    div class="mt-6" {
                        (self.items_table())
                    }
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        journal_entry_detail_menu(self.id, self.journal_id, "detail")
    }
}

impl RenderAppPane for JournalEntryDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let entry_label = format!("#{}", self.id);
        let crumbs = journal_entry_crumbs(self.journal_id, &self.journal_label, &entry_label);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        let entry_label = format!("#{}", self.id);
        layout_main_with_crumbs(
            journal_entry_crumbs(self.journal_id, &self.journal_label, &entry_label),
            self.body(),
        )
    }
}

impl RenderTemplate for JournalEntryDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let entry_label = format!("#{}", self.id);
        let crumbs = journal_entry_crumbs(self.journal_id, &self.journal_label, &entry_label);
        app_scaffold_with_sidebar(
            "Journal Entry",
            chrome,
            self.menu(),
            crumbs,
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct JournalEntryDeletePage {
    pub id: i64,
    pub journal_id: i64,
    pub journal_label: String,
    pub can_delete: bool,
    pub error: Option<String>,
}

impl JournalEntryDeletePage {
    fn body(&self) -> Markup {
        if let Some(err) = &self.error {
            return html! {
                div class="container mx-auto my-4" {
                    h2 class="text-xl font-bold text-error" { "Delete unavailable" }
                    div class="alert alert-error my-2 text-sm" { (err) }
                }
            };
        }
        let message = format!(
            "Are you sure you want to delete journal entry #{}? Related invoices, payments, \
             credit notes, and any linked journal entries will also be deleted.",
            self.id
        );
        delete_confirmation(DeleteConfirmation {
            title: "Delete journal entry",
            message: &message,
            attrs: form_hx_post_redirect(JournalEntryDeletePostRouteTag::new(self.id)).set(
                "hx-confirm",
                "This permanently deletes the journal entry and all related invoices, payments, \
                 credit notes, and linked journal entries. This cannot be undone. Continue?",
            ),
            ..Default::default()
        })
    }

    fn menu(&self) -> Markup {
        journal_entry_detail_menu(self.id, self.journal_id, "delete")
    }
}

impl RenderAppPane for JournalEntryDeletePage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let entry_label = format!("#{}", self.id);
        let crumbs = journal_entry_crumbs(self.journal_id, &self.journal_label, &entry_label);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        let entry_label = format!("#{}", self.id);
        layout_main_with_crumbs(
            journal_entry_crumbs(self.journal_id, &self.journal_label, &entry_label),
            self.body(),
        )
    }
}

impl RenderTemplate for JournalEntryDeletePage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let entry_label = format!("#{}", self.id);
        let crumbs = journal_entry_crumbs(self.journal_id, &self.journal_label, &entry_label);
        app_scaffold_with_sidebar(
            "Delete Journal Entry",
            chrome,
            self.menu(),
            crumbs,
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct JournalEntrySelectPage {
    pub entries: ObjectList<JournalEntryRow>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<JournalEntrySelectTableKey, JournalEntrySelectModalKey>
    for JournalEntrySelectPage
{
    fn render_table(&self) -> Markup {
        let id_sort = column_sort_url(&self.path_and_query, "ID", &self.sort);
        let datetime_sort = column_sort_url(&self.path_and_query, "DateTime", &self.sort);
        let id_label = format!("ID{}", sort_indicator(&self.sort, "ID"));
        let datetime_label = format!("Date & time{}", sort_indicator(&self.sort, "DateTime"));
        let headers = [
            TableColumnHeader {
                key: "ID",
                label: &id_label,
                sort_url: Some(&id_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "DateTime",
                label: &datetime_label,
                sort_url: Some(&datetime_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Journal",
                label: "Journal",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .entries
            .items
            .iter()
            .map(|e| {
                let select_label = if e.label.is_empty() {
                    format!("{} · {}", e.journal_name, e.datetime)
                } else {
                    e.label.clone()
                };
                TableRow {
                    attrs: row_attr_select(&self.target_input, &e.id.to_string(), &select_label),
                    cells: vec![
                        field_text(FieldText { value: &e.id.to_string(), classes: "" }),
                        field_text(FieldText { value: &e.datetime, classes: "" }),
                        field_text(FieldText { value: &e.journal_name, classes: "" }),
                    ],
                }
            })
            .collect();
        let pagination = render_picker_pagination::<JournalEntrySelectModalKey>(
            &self.path_and_query,
            self.entries.number,
            self.entries.num_pages,
        );
        data_table_list_refresh::<JournalEntrySelectTableKey>(
            "Select Journal Entry",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for JournalEntrySelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}
