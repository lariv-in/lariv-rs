use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, Crumb, DetailHeader, FieldLink, FieldText, FilterClauseView,
    FormOpts, ListFilterBuilder, ObjectList, ShellChrome, SwapKey, TableButtonFilter,
    TableColumnHeader, TableRow, breadcrumbs, button_modal_form, button_submit, column_sort_url,
    container_column, container_row, detail, detail_header, field_link, field_text, form,
    form_hx_get_picker_route, form_hx_get_route, form_hx_post_url, label, list_filter_builder,
    modal_keyed, row_attr_navigate_route, row_attr_select_extra, sort_indicator,
    table_button_filter,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::picker::RenderPickerSelect;
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use crate::{
    accounting_detail_menu::{DetailMenuNavItem, detail_sidebar_menu},
    forms::{CustomSourceDocTemplateForm, CustomSourceDocTemplateFormField},
    keys::{
        CustomSourceDocIndexCreateModalKey, CustomSourceDocTemplateCreateModalKey,
        CustomSourceDocTemplateDeleteModalKey, CustomSourceDocTemplateEditModalKey,
        CustomSourceDocTemplateSelectModalKey, CustomSourceDocTemplateSelectTableKey,
        CustomSourceDocTemplateTableKey,
    },
    routes::{
        CustomSourceDocIndexCreateGetRouteTag, CustomSourceDocIndexCreatePostRouteTag,
        CustomSourceDocTemplateCreateGetRouteTag, CustomSourceDocTemplateCreatePostRouteTag,
        CustomSourceDocTemplateDeleteGetRouteTag, CustomSourceDocTemplateDetailRouteTag,
        CustomSourceDocTemplateEditGetRouteTag, CustomSourceDocTemplateEditPostRouteTag,
        CustomSourceDocTemplateListRouteTag, CustomSourceDocTemplateSelectRouteTag,
    },
};

use super::common::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs, render_pagination,
    render_picker_pagination,
};

fn list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Transaction Templates",
        href: None,
    }])
}

fn detail_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = CustomSourceDocTemplateListRouteTag.url();
    let detail_url = CustomSourceDocTemplateDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Transaction Templates",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Transaction Templates",
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

fn detail_menu(id: i64, name: &str) -> Markup {
    let menu_title = format!("Template: {name}");
    let detail_url = CustomSourceDocTemplateDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Template Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(menu_title, &nav, None, html! {})
}

#[derive(Clone)]
pub struct CustomSourceDocTemplateRow {
    pub id: i64,
    pub name: String,
    pub remarks: String,
    pub variables: String,
}

fn list_filter_form(page: &CustomSourceDocTemplateListPage) -> Markup {
    list_filter_builder(ListFilterBuilder {
        fields: &page.fields,
        rows: &page.filters,
        page_size: page.page_size,
        sort: &page.sort,
        attrs: form_hx_get_route::<
            CustomSourceDocTemplateTableKey,
            CustomSourceDocTemplateListRouteTag,
        >(CustomSourceDocTemplateListRouteTag),
        apply_label: "Apply Filters",
        extra: html! {},
    })
}

#[derive(Generic)]
pub struct CustomSourceDocTemplateListPage {
    pub templates: ObjectList<CustomSourceDocTemplateRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl CustomSourceDocTemplateListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Variables",
                label: "Variables",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Remarks",
                label: "Remarks",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .templates
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_navigate_route(CustomSourceDocTemplateDetailRouteTag::new(t.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &t.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: if t.variables.trim().is_empty() {
                            "—"
                        } else {
                            t.variables.as_str()
                        },
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: if t.remarks.trim().is_empty() {
                            "—"
                        } else {
                            t.remarks.as_str()
                        },
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_form(self),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_accounts.CustomSourceDocTemplateCreateForm",
                    href: &CustomSourceDocTemplateCreateGetRouteTag.url(),
                    form_post_url: &CustomSourceDocTemplateCreateGetRouteTag.path(),
                    modal_uid: CustomSourceDocTemplateCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<CustomSourceDocTemplateTableKey>(
            &self.path_and_query,
            self.templates.number,
            self.templates.num_pages,
        );
        super::data_table_with_filter_errors::<CustomSourceDocTemplateTableKey>(
            "Transaction Templates",
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

impl RenderAppPane for CustomSourceDocTemplateListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(list_crumbs(), self.body())
    }
}

impl RenderTemplate for CustomSourceDocTemplateListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Transaction Templates",
            chrome,
            list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct CustomSourceDocTemplateDetailPage {
    pub id: i64,
    pub name: String,
    pub remarks: String,
    pub attached_vnode_href: String,
    pub attached_vnode_label: String,
    pub variables: String,
    pub journal_lines: String,
    pub can_edit: bool,
}

impl CustomSourceDocTemplateDetailPage {
    fn header_actions(&self) -> Markup {
        if !self.can_edit {
            return html! {};
        }
        let create_url = format!(
            "{}?template_id={}",
            CustomSourceDocIndexCreateGetRouteTag.url(),
            self.id
        );
        let create_post = CustomSourceDocIndexCreatePostRouteTag.path();
        let edit_url = CustomSourceDocTemplateEditGetRouteTag::new(self.id).url();
        let edit_post = CustomSourceDocTemplateEditPostRouteTag::new(self.id).path();
        html! {
            (button_modal_form(ButtonModalForm {
                name: "p_finance_accounts.CustomSourceDocIndexCreateForm",
                href: &create_url,
                form_post_url: &create_post,
                modal_uid: CustomSourceDocIndexCreateModalKey::ID,
                label: "New transaction",
                classes: "btn-primary",
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "p_finance_accounts.CustomSourceDocTemplateEditForm",
                href: &edit_url,
                form_post_url: &edit_post,
                modal_uid: CustomSourceDocTemplateEditModalKey::ID,
                label: "Edit",
                classes: "btn-outline",
                ..Default::default()
            }))
        }
    }

    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions: self.header_actions(),
                    }))
                    (label("Remarks", field_text(FieldText {
                        value: if self.remarks.trim().is_empty() { "—" } else { self.remarks.as_str() },
                        classes: "",
                    })))
                    (label(
                        "Attached file",
                        if self.attached_vnode_href.is_empty() {
                            field_text(FieldText {
                                value: "—",
                                classes: "",
                            })
                        } else {
                            field_link(FieldLink {
                                href: &self.attached_vnode_href,
                                label: &self.attached_vnode_label,
                                classes: "",
                            })
                        },
                    ))
                    (label("Variables", field_text(FieldText {
                        value: if self.variables.trim().is_empty() { "—" } else { self.variables.as_str() },
                        classes: "",
                    })))
                    (label("Journal entry template", field_text(FieldText {
                        value: &self.journal_lines,
                        classes: "",
                    })))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        detail_menu(self.id, &self.name)
    }
}

impl RenderAppPane for CustomSourceDocTemplateDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = detail_crumbs(self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(detail_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for CustomSourceDocTemplateDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = detail_crumbs(self.id, &self.name, None);
        app_scaffold_with_sidebar(
            "Transaction Template",
            chrome,
            self.menu(),
            crumbs,
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct CustomSourceDocTemplateSelectPage {
    pub templates: ObjectList<CustomSourceDocTemplateRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub target_input: String,
    pub page_size: u32,
}

impl RenderPickerSelect<CustomSourceDocTemplateSelectTableKey, CustomSourceDocTemplateSelectModalKey>
    for CustomSourceDocTemplateSelectPage
{
    fn render_table(&self) -> Markup {
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
                key: "Remarks",
                label: "Remarks",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .templates
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_select_extra(
                    &self.target_input,
                    &t.id.to_string(),
                    &t.name,
                    &[("title", t.name.as_str()), ("content", t.remarks.as_str())],
                ),
                cells: vec![
                    field_text(FieldText {
                        value: &t.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: if t.remarks.trim().is_empty() {
                            "—"
                        } else {
                            t.remarks.as_str()
                        },
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let target_input = self.target_input.clone();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_picker_route::<
                        CustomSourceDocTemplateSelectTableKey,
                        CustomSourceDocTemplateSelectModalKey,
                        CustomSourceDocTemplateSelectRouteTag,
                    >(CustomSourceDocTemplateSelectRouteTag),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(target_input) {}
                    },
                }),
                ..Default::default()
            }))
        };
        let pagination = render_picker_pagination::<CustomSourceDocTemplateSelectModalKey>(
            &self.path_and_query,
            self.templates.number,
            self.templates.num_pages,
        );
        super::data_table_with_filter_errors::<CustomSourceDocTemplateSelectTableKey>(
            "Select Transaction Template",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for CustomSourceDocTemplateSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Generic)]
pub struct CustomSourceDocTemplateEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub remarks: String,
    pub attached_vnode_id: String,
    pub attached_vnode_display: String,
    pub variables: Vec<String>,
    pub journal_lines: String,
    pub error: String,
}

impl RenderTemplate for CustomSourceDocTemplateEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = CustomSourceDocTemplateDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<CustomSourceDocTemplateEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit transaction template" }
                (form(&CsrfToken::current(), FormOpts {
                    classes: "@container",
                    attrs: form_hx_post_url::<CustomSourceDocTemplateEditModalKey>(
                        &modal_edit_post_url(
                            CustomSourceDocTemplateEditPostRouteTag::new(self.id),
                            &self.form_name,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CustomSourceDocTemplateForm::render_inputs(
                        &FormCtx::form::<CustomSourceDocTemplateForm>(CsrfToken::current())
                            .value(CustomSourceDocTemplateFormField::Name, &self.name)
                            .value(CustomSourceDocTemplateFormField::Remarks, &self.remarks)
                            .value(
                                CustomSourceDocTemplateFormField::AttachedVnodeId,
                                &self.attached_vnode_id,
                            )
                            .display(
                                CustomSourceDocTemplateFormField::AttachedVnodeId,
                                &self.attached_vnode_display,
                            )
                            .list(CustomSourceDocTemplateFormField::Variables, &self.variables)
                            .value(
                                CustomSourceDocTemplateFormField::JournalLines,
                                &self.journal_lines,
                            ),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_finance_accounts.CustomSourceDocTemplateDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: CustomSourceDocTemplateDeleteModalKey::ID,
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
pub struct CustomSourceDocTemplateCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub remarks: String,
    pub attached_vnode_id: String,
    pub attached_vnode_display: String,
    pub variables: Vec<String>,
    pub journal_lines: String,
    pub error: String,
}

impl RenderTemplate for CustomSourceDocTemplateCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_accounts.CustomSourceDocTemplateCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<CustomSourceDocTemplateCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create Transaction Template",
                    subtitle: "Define variables and journal entry lines",
                    classes: "@container",
                    attrs: form_hx_post_url::<CustomSourceDocTemplateCreateModalKey>(
                        &modal_create_post_url(
                            CustomSourceDocTemplateCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CustomSourceDocTemplateForm::render_inputs(
                        &FormCtx::form::<CustomSourceDocTemplateForm>(CsrfToken::current())
                            .value(CustomSourceDocTemplateFormField::Name, &self.name)
                            .value(CustomSourceDocTemplateFormField::Remarks, &self.remarks)
                            .value(
                                CustomSourceDocTemplateFormField::AttachedVnodeId,
                                &self.attached_vnode_id,
                            )
                            .display(
                                CustomSourceDocTemplateFormField::AttachedVnodeId,
                                &self.attached_vnode_display,
                            )
                            .list(CustomSourceDocTemplateFormField::Variables, &self.variables)
                            .value(
                                CustomSourceDocTemplateFormField::JournalLines,
                                &self.journal_lines,
                            ),
                    ),
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save Template",
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
