use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, Crumb, FieldText, FieldTitle, FilterClauseView, FormOpts,
    ListFilterBuilder, ObjectList, ShellChrome, SwapKey, TableButtonFilter, TableColumnHeader,
    TableRow, breadcrumbs, button_modal_form, button_submit, column_sort_url, container_column,
    container_row, detail, field_text, field_title, form, form_hx_get_picker_route,
    form_hx_get_route, form_hx_post_url, label, list_filter_builder, modal_keyed,
    row_attr_navigate_route, row_attr_select, sort_indicator, table_button_filter,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::picker::RenderPickerSelect;
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use crate::{
    entities::currency,
    forms::{CurrencyForm, CurrencyFormField},
    keys::{
        CurrencyCreateModalKey, CurrencyDeleteModalKey, CurrencyEditModalKey,
        CurrencySelectModalKey, CurrencySelectTableKey, CurrencyTableKey,
    },
    routes::{
        CurrencyCreateGetRouteTag, CurrencyCreatePostRouteTag, CurrencyDeleteGetRouteTag,
        CurrencyDetailRouteTag, CurrencyEditGetRouteTag, CurrencyEditPostRouteTag,
        CurrencyListRouteTag, CurrencySelectRouteTag,
    },
};

use super::common::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs, render_pagination,
    render_picker_pagination,
};
use crate::accounting_detail_menu::{DetailMenuNavItem, detail_sidebar_menu};

fn currencies_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Currencies",
        href: None,
    }])
}

fn currency_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = CurrencyListRouteTag.url();
    let detail_url = CurrencyDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Currencies",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Currencies",
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

fn currency_detail_menu(id: i64, name: &str) -> Markup {
    let menu_title = format!("Currency: {name}");
    let detail_url = CurrencyDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Currency Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(menu_title, &nav, None, html! {})
}

#[derive(Clone)]
pub struct CurrencyRow {
    pub id: i64,
    pub code: i32,
    pub name: String,
    pub symbol: String,
    pub minor_unit: i32,
}

fn currency_filter_form(page: &CurrencyListPage) -> Markup {
    list_filter_builder(ListFilterBuilder {
        fields: &page.fields,
        rows: &page.filters,
        page_size: page.page_size,
        sort: &page.sort,
        attrs: form_hx_get_route::<CurrencyTableKey, CurrencyListRouteTag>(CurrencyListRouteTag),
        apply_label: "Apply Filters",
        extra: html! {},
    })
}

#[derive(Generic)]
pub struct CurrencyListPage {
    pub currencies: ObjectList<CurrencyRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl CurrencyListPage {
    pub fn render_table(&self) -> Markup {
        let code_sort = column_sort_url(&self.path_and_query, "Code", &self.sort);
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let symbol_sort = column_sort_url(&self.path_and_query, "Symbol", &self.sort);
        let minor_unit_sort = column_sort_url(&self.path_and_query, "MinorUnit", &self.sort);
        let code_label = format!("Code{}", sort_indicator(&self.sort, "Code"));
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let symbol_label = format!("Symbol{}", sort_indicator(&self.sort, "Symbol"));
        let minor_unit_label = format!("Minor unit{}", sort_indicator(&self.sort, "MinorUnit"));
        let headers = [
            TableColumnHeader {
                key: "Code",
                label: &code_label,
                sort_url: Some(&code_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Symbol",
                label: &symbol_label,
                sort_url: Some(&symbol_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "MinorUnit",
                label: &minor_unit_label,
                sort_url: Some(&minor_unit_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .currencies
            .items
            .iter()
            .map(|c| TableRow {
                attrs: row_attr_navigate_route(CurrencyDetailRouteTag::new(c.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &c.code.to_string(),
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.symbol,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.minor_unit.to_string(),
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: currency_filter_form(self),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_accounts.CurrencyCreateForm",
                    href: &CurrencyCreateGetRouteTag.url(),
                    form_post_url: &CurrencyCreateGetRouteTag.path(),
                    modal_uid: CurrencyCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<CurrencyTableKey>(
            &self.path_and_query,
            self.currencies.number,
            self.currencies.num_pages,
        );
        super::data_table_with_filter_errors::<CurrencyTableKey>(
            "Currencies",
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

impl RenderAppPane for CurrencyListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, currencies_list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(currencies_list_crumbs(), self.body())
    }
}

impl RenderTemplate for CurrencyListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Currencies",
            chrome,
            currencies_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct CurrencyDetailPage {
    pub id: i64,
    pub code: i32,
    pub name: String,
    pub symbol: String,
    pub minor_unit: i32,
    pub can_edit: bool,
}

impl CurrencyDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (field_text(FieldText {
                        value: &format!("ISO 4217 code {}", self.code),
                        classes: "text-base-content/70",
                    }))
                    (label("Symbol", field_text(FieldText { value: &self.symbol, classes: "" })))
                    (label("Minor unit", field_text(FieldText { value: &self.minor_unit.to_string(), classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_finance_accounts.CurrencyEditForm",
                                href: &CurrencyEditGetRouteTag::new(self.id).url(),
                                form_post_url: &CurrencyEditPostRouteTag::new(self.id).path(),
                                modal_uid: CurrencyEditModalKey::ID,
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
        currency_detail_menu(self.id, &self.name)
    }
}

impl RenderAppPane for CurrencyDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = currency_crumbs(self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(currency_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for CurrencyDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = currency_crumbs(self.id, &self.name, None);
        app_scaffold_with_sidebar("Currency", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct CurrencyEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub minor_unit: String,
    pub error: String,
}

impl CurrencyEditModalPage {
    pub fn from_model(c: &currency::Model, form_name: String) -> Self {
        Self {
            id: c.id,
            form_name,
            code: c.code.to_string(),
            name: c.name.clone(),
            symbol: c.symbol.clone(),
            minor_unit: c.minor_unit.to_string(),
            error: String::new(),
        }
    }
}

impl RenderTemplate for CurrencyEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = CurrencyDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<CurrencyEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit currency" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<CurrencyEditModalKey>(&modal_edit_post_url(
                        CurrencyEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CurrencyForm::render_inputs(
                        &FormCtx::form::<CurrencyForm>(CsrfToken::current())
                            .value(CurrencyFormField::Code, &self.code)
                            .value(CurrencyFormField::Name, &self.name)
                            .value(CurrencyFormField::Symbol, &self.symbol)
                            .value(CurrencyFormField::MinorUnit, &self.minor_unit),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_finance_accounts.CurrencyDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: CurrencyDeleteModalKey::ID,
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
pub struct CurrencyCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub minor_unit: String,
    pub error: String,
}

impl RenderTemplate for CurrencyCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_accounts.CurrencyCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<CurrencyCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create Currency",
                    subtitle: "Create a new currency",
                    attrs: form_hx_post_url::<CurrencyCreateModalKey>(&modal_create_post_url(
                        CurrencyCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CurrencyForm::render_inputs(
                        &FormCtx::form::<CurrencyForm>(CsrfToken::current())
                            .value(CurrencyFormField::Code, &self.code)
                            .value(CurrencyFormField::Name, &self.name)
                            .value(CurrencyFormField::Symbol, &self.symbol)
                            .value(CurrencyFormField::MinorUnit, &self.minor_unit),
                    ),
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save Currency",
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

#[derive(Generic)]
pub struct CurrencySelectPage {
    pub currencies: ObjectList<CurrencyRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub target_input: String,
    pub page_size: u32,
}

impl RenderPickerSelect<CurrencySelectTableKey, CurrencySelectModalKey> for CurrencySelectPage {
    fn render_table(&self) -> Markup {
        let code_sort = column_sort_url(&self.path_and_query, "Code", &self.sort);
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let symbol_sort = column_sort_url(&self.path_and_query, "Symbol", &self.sort);
        let code_label = format!("Code{}", sort_indicator(&self.sort, "Code"));
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let symbol_label = format!("Symbol{}", sort_indicator(&self.sort, "Symbol"));
        let headers = [
            TableColumnHeader {
                key: "Code",
                label: &code_label,
                sort_url: Some(&code_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Symbol",
                label: &symbol_label,
                sort_url: Some(&symbol_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .currencies
            .items
            .iter()
            .map(|c| {
                let label = format!("{} — {}", c.code, c.name);
                TableRow {
                    attrs: row_attr_select(&self.target_input, &c.id.to_string(), &label),
                    cells: vec![
                        field_text(FieldText {
                            value: &c.code.to_string(),
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &c.name,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &c.symbol,
                            classes: "",
                        }),
                    ],
                    key: String::new(),
                    children: Vec::new(),
                }
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
                        CurrencySelectTableKey,
                        CurrencySelectModalKey,
                        CurrencySelectRouteTag,
                    >(CurrencySelectRouteTag),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(target_input) {}
                    },
                }),
                ..Default::default()
            }))
        };
        let pagination = render_picker_pagination::<CurrencySelectModalKey>(
            &self.path_and_query,
            self.currencies.number,
            self.currencies.num_pages,
        );
        super::data_table_with_filter_errors::<CurrencySelectTableKey>(
            "Select Currency",
            actions,
            &headers,
            &rows,
            pagination,
            "",
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for CurrencySelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}
