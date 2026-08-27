use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, FieldText,
        FieldTitle, FormOpts, ObjectList, PaginationPage, ShellChrome, SlotCapability,
        SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow,
        breadcrumbs, button_clear, button_modal_form, button_submit, column_sort_url,
        container_column, container_row, data_table_list_refresh, delete_confirmation, detail,
        field_text, field_title, form, form_hx_get_picker_route, form_hx_get_route,
        form_hx_post_selector, form_hx_post_url, modal, modal_keyed, pagination_pages,
        row_attr_navigate_route, row_attr_select_multi, sort_indicator, table_button_filter,
        table_create_button, table_pagination,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    picker::{RenderPickerSelect, picker_create_button},
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_query, modal_edit_post_url},
};

use crate::plugins::finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use crate::plugins::finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs, render_picker_pagination,
};

use super::forms::{
    TaxFilterForm, TaxFilterFormField, TaxForm, TaxFormField, tax_type_choices,
    tax_type_filter_choices,
};
use super::keys::{
    TaxCreateModalKey, TaxDeleteModalKey, TaxEditModalKey, TaxMultiSelectModalKey,
    TaxMultiSelectTableKey, TaxTableKey,
};
use super::routes::{
    TaxCreatePostRouteTag, TaxDefaultRouteTag, TaxDeleteGetRouteTag, TaxDeletePostRouteTag,
    TaxDetailRouteTag, TaxEditGetRouteTag, TaxEditPostRouteTag, TaxMultiSelectRouteTag,
};

fn taxes_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Taxes",
        href: None,
    }])
}

fn tax_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = TaxDefaultRouteTag.url();
    let detail_url = TaxDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Taxes",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Taxes",
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

fn tax_detail_menu(id: i64, name: &str) -> Markup {
    let menu_title = format!("Tax: {name}");
    let detail_url = TaxDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Tax Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(menu_title, &nav, None, html! {})
}

crate::define_register_items! {
    plugin: FinanceTaxesTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        TaxListIdx: TaxListPageTag => TaxListPage,
        TaxDetailIdx: TaxDetailPageTag => TaxDetailPage,
        TaxEditModalIdx: TaxEditModalPageTag => TaxEditModalPage,
        TaxCreateModalIdx: TaxCreateModalPageTag => TaxCreateModalPage,
        TaxMultiSelectIdx: TaxMultiSelectPageTag => TaxMultiSelectPage,
        ConfirmDeleteIdx: TaxConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

crate::define_register_items! {
    plugin: FinanceTaxesTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn tax_filter_form(name: &str, tax_type: &str) -> Markup {
    let type_choices = tax_type_filter_choices();
    form(FormOpts {
        attrs: form_hx_get_route::<TaxTableKey, TaxDefaultRouteTag>(TaxDefaultRouteTag),
        inputs: TaxFilterForm::render_inputs(
            &FormCtx::form::<TaxFilterForm>()
                .value(TaxFilterFormField::Name, name)
                .value(TaxFilterFormField::TaxType, tax_type)
                .choices(TaxFilterFormField::TaxType, &type_choices),
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

fn render_pagination(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, true);
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
        hx_target: TaxTableKey::SELECTOR,
    })
}

#[derive(Clone)]
pub struct TaxRow {
    pub id: i64,
    pub name: String,
    pub tax_type: String,
    pub percentage: String,
    pub account_label: String,
}

#[derive(Generic)]
pub struct TaxListPage {
    pub taxes: ObjectList<TaxRow>,
    pub filter_name: String,
    pub filter_tax_type: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl TaxListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let percentage_sort = column_sort_url(&self.path_and_query, "Percentage", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let percentage_label = format!("Percentage{}", sort_indicator(&self.sort, "Percentage"));
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
                key: "Percentage",
                label: &percentage_label,
                sort_url: Some(&percentage_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Account",
                label: "Account",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .taxes
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_navigate_route(TaxDetailRouteTag::new(t.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &t.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &t.tax_type,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &t.percentage,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &t.account_label,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: tax_filter_form(&self.filter_name, &self.filter_tax_type),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (table_create_button::<TaxTableKey, TaxCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination(
            &self.path_and_query,
            self.taxes.number,
            self.taxes.num_pages,
        );
        data_table_list_refresh::<TaxTableKey>(
            "Taxes",
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

impl RenderAppPane for TaxListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, taxes_list_crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(taxes_list_crumbs(), self.body())
    }
}

impl RenderTemplate for TaxListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Taxes",
            chrome,
            taxes_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct TaxDetailPage {
    pub id: i64,
    pub name: String,
    pub tax_type: String,
    pub percentage: String,
    pub account_label: String,
    pub can_edit: bool,
}

impl TaxDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (crate::components::label("Type", field_text(FieldText { value: &self.tax_type, classes: "" })))
                    (crate::components::label("Percentage", field_text(FieldText { value: &self.percentage, classes: "" })))
                    (crate::components::label("Account", field_text(FieldText { value: &self.account_label, classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_taxes.TaxEditForm",
                                href: &TaxEditGetRouteTag::new(self.id).url(),
                                form_post_url: &TaxEditPostRouteTag::new(self.id).path(),
                                modal_uid: TaxEditModalKey::ID,
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                        }))
                    }
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        tax_detail_menu(self.id, &self.name)
    }
}

impl RenderAppPane for TaxDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = tax_crumbs(self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(tax_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for TaxDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = tax_crumbs(self.id, &self.name, None);
        app_scaffold_with_sidebar("Tax", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct TaxEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub tax_type: String,
    pub percentage: String,
    pub account_id: String,
    pub account_display: String,
    pub error: String,
}

impl RenderTemplate for TaxEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let choices = tax_type_choices();
        let delete_url = TaxDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<TaxEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit tax" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<TaxEditModalKey>(&modal_edit_post_url(
                        TaxEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TaxForm::render_inputs(
                        &FormCtx::form::<TaxForm>()
                            .value(TaxFormField::Name, &self.name)
                            .value(TaxFormField::TaxType, &self.tax_type)
                            .value(TaxFormField::Percentage, &self.percentage)
                            .value(TaxFormField::AccountId, &self.account_id)
                            .display(TaxFormField::AccountId, &self.account_display)
                            .choices(TaxFormField::TaxType, &choices),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_finance_taxes.TaxDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: TaxDeleteModalKey::ID,
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
pub struct TaxCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub tax_type: String,
    pub percentage: String,
    pub account_id: String,
    pub account_display: String,
    pub error: String,
}

impl RenderTemplate for TaxCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_taxes.TaxCreateForm"
        } else {
            self.form_name.as_str()
        };
        let choices = tax_type_choices();
        modal_keyed::<TaxCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Tax",
                subtitle: "Create a new tax",
                classes: "@container",
                attrs: form_hx_post_url::<TaxCreateModalKey>(&modal_create_post_query(
                    TaxCreatePostRouteTag,
                    form_name,
                    &self.refresh_table,
                    &self.target_input,
                )),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: TaxForm::render_inputs(
                    &FormCtx::form::<TaxForm>()
                        .value(TaxFormField::Name, &self.name)
                        .value(TaxFormField::TaxType, &self.tax_type)
                        .value(TaxFormField::Percentage, &self.percentage)
                        .value(TaxFormField::AccountId, &self.account_id)
                        .display(TaxFormField::AccountId, &self.account_display)
                        .choices(TaxFormField::TaxType, &choices),
                ),
                actions: html! {
                    (container_row("flex justify-end gap-2 mt-2", html! {
                        (button_submit(ButtonSubmit {
                            label: "Save Tax",
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
pub struct TaxMultiSelectPage {
    pub taxes: ObjectList<TaxRow>,
    pub filter_name: String,
    pub filter_tax_type: String,
    pub sort: String,
    pub path_and_query: String,
    pub target_input: String,
    pub can_edit: bool,
}

impl RenderPickerSelect<TaxMultiSelectTableKey, TaxMultiSelectModalKey> for TaxMultiSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "Taxes"
        } else {
            self.target_input.as_str()
        };
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let percentage_sort = column_sort_url(&self.path_and_query, "Percentage", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let percentage_label = format!("Percentage{}", sort_indicator(&self.sort, "Percentage"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Percentage",
                label: &percentage_label,
                sort_url: Some(&percentage_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .taxes
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_select_multi(target, &t.id.to_string(), &t.name),
                cells: vec![
                    field_text(FieldText {
                        value: &t.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &t.tax_type,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &t.percentage,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let type_choices = tax_type_filter_choices();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_picker_route::<
                        TaxMultiSelectTableKey,
                        TaxMultiSelectModalKey,
                        TaxMultiSelectRouteTag,
                    >(TaxMultiSelectRouteTag),
                    inputs: html! {
                        (TaxFilterForm::render_inputs(
                            &FormCtx::form::<TaxFilterForm>()
                                .value(TaxFilterFormField::Name, &self.filter_name)
                                .value(TaxFilterFormField::TaxType, &self.filter_tax_type)
                                .choices(TaxFilterFormField::TaxType, &type_choices),
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
        if self.can_edit {
            actions = html! {
                (actions)
                (picker_create_button::<TaxCreateModalKey>(
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_picker_pagination::<TaxMultiSelectModalKey>(
            &self.path_and_query,
            self.taxes.number,
            self.taxes.num_pages,
        );
        data_table_list_refresh::<TaxMultiSelectTableKey>(
            "Select Taxes",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for TaxMultiSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", TaxDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            TaxDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = TaxDeletePostRouteTag::new(self.id).url();
        modal(crate::components::Modal {
            uid,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
