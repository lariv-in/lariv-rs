use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonDeletePost, ButtonModalForm, ButtonSubmit, Crumb, FieldCheckbox,
        FieldDate, FieldText, FieldTitle, FormOpts, ObjectList, PaginationPage, ShellChrome,
        SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader,
        TablePagination, TableRow, breadcrumbs, button_clear, button_delete_post_route,
        button_modal_form, button_submit, column_sort_url, container_column, container_row,
        data_table_list_refresh, detail, field_checkbox, field_date, field_text, field_title, form,
        form_hx_get_route, form_hx_post_url, label, modal_keyed, pagination_pages,
        row_attr_navigate_route, row_attr_select, sort_indicator, table_button_filter,
        table_pagination,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    picker::RenderPickerSelect,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_url, modal_edit_post_url},
};

use crate::plugins::finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use crate::plugins::finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use super::forms::{
    FiscalYearFilterForm, FiscalYearFilterFormField, FiscalYearForm, FiscalYearFormField,
};
use super::keys::{
    FiscalYearCreateModalKey, FiscalYearEditModalKey, FiscalYearSelectModalKey,
    FiscalYearSelectTableKey, FiscalYearTableKey,
};
use super::routes::{
    FiscalYearCreateGetRouteTag, FiscalYearCreatePostRouteTag, FiscalYearDefaultRouteTag,
    FiscalYearDeletePostRouteTag, FiscalYearDetailRouteTag, FiscalYearEditGetRouteTag,
    FiscalYearEditPostRouteTag, FiscalYearSelectRouteTag,
};

fn fiscal_years_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Fiscal Years",
        href: None,
    }])
}

fn fiscal_year_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = FiscalYearDefaultRouteTag.url();
    let detail_url = FiscalYearDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Fiscal Years",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Fiscal Years",
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

fn fiscal_year_detail_menu(id: i64, name: &str) -> Markup {
    let menu_title = format!("Fiscal year: {name}");
    let detail_url = FiscalYearDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Fiscal Year Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(menu_title, &nav, None, html! {})
}

crate::define_register_items! {
    plugin: FinanceFiscalYearTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        FiscalYearListIdx: FiscalYearListPageTag => FiscalYearListPage,
        FiscalYearDetailIdx: FiscalYearDetailPageTag => FiscalYearDetailPage,
        FiscalYearEditModalIdx: FiscalYearEditModalPageTag => FiscalYearEditModalPage,
        FiscalYearCreateModalIdx: FiscalYearCreateModalPageTag => FiscalYearCreateModalPage,
        FiscalYearSelectIdx: FiscalYearSelectPageTag => FiscalYearSelectPage,
    ]
}

crate::define_register_items! {
    plugin: FinanceFiscalYearTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn fiscal_year_filter_form(code: &str, name: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<FiscalYearTableKey, FiscalYearDefaultRouteTag>(
            FiscalYearDefaultRouteTag,
        ),
        inputs: FiscalYearFilterForm::render_inputs(
            &FormCtx::form::<FiscalYearFilterForm>()
                .value(FiscalYearFilterFormField::Code, code)
                .value(FiscalYearFilterFormField::Name, name),
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

fn fiscal_year_select_filter_form(code: &str, name: &str, target_input: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<FiscalYearSelectTableKey, FiscalYearSelectRouteTag>(
            FiscalYearSelectRouteTag,
        )
        .set("hx-push-url", "false"),
        inputs: html! {
            (FiscalYearFilterForm::render_inputs(
                &FormCtx::form::<FiscalYearFilterForm>()
                    .value(FiscalYearFilterFormField::Code, code)
                    .value(FiscalYearFilterFormField::Name, name),
            ))
            input type="hidden" name="target_input" value=(target_input) {}
        },
        actions: html! {
            (container_row("flex gap-2", html! {
                (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
            }))
        },
        ..Default::default()
    })
}

fn render_pagination<K: SwapKey>(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
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
        hx_target: K::SELECTOR,
    })
}

#[derive(Clone)]
pub struct FiscalYearRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub start: String,
    pub end: String,
    pub is_active: bool,
}

#[derive(Generic)]
pub struct FiscalYearListPage {
    pub fiscal_years: ObjectList<FiscalYearRow>,
    pub filter_code: String,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl FiscalYearListPage {
    pub fn render_table(&self) -> Markup {
        let code_sort = column_sort_url(&self.path_and_query, "Code", &self.sort);
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let start_sort = column_sort_url(&self.path_and_query, "Start", &self.sort);
        let end_sort = column_sort_url(&self.path_and_query, "End", &self.sort);
        let active_sort = column_sort_url(&self.path_and_query, "Active", &self.sort);
        let code_label = format!("Code{}", sort_indicator(&self.sort, "Code"));
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let start_label = format!("Start{}", sort_indicator(&self.sort, "Start"));
        let end_label = format!("End{}", sort_indicator(&self.sort, "End"));
        let active_label = format!("Active{}", sort_indicator(&self.sort, "Active"));
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
                key: "Start",
                label: &start_label,
                sort_url: Some(&start_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "End",
                label: &end_label,
                sort_url: Some(&end_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Active",
                label: &active_label,
                sort_url: Some(&active_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .fiscal_years
            .items
            .iter()
            .map(|fy| TableRow {
                attrs: row_attr_navigate_route(FiscalYearDetailRouteTag::new(fy.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &fy.code,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &fy.name,
                        classes: "",
                    }),
                    field_date(FieldDate {
                        value: &fy.start,
                        classes: "",
                    }),
                    field_date(FieldDate {
                        value: &fy.end,
                        classes: "",
                    }),
                    field_checkbox(FieldCheckbox {
                        checked: fy.is_active,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: fiscal_year_filter_form(&self.filter_code, &self.filter_name),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_fiscal_year.FiscalYearCreateForm",
                    href: &FiscalYearCreateGetRouteTag.url(),
                    form_post_url: &FiscalYearCreateGetRouteTag.path(),
                    modal_uid: FiscalYearCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<FiscalYearTableKey>(
            &self.path_and_query,
            self.fiscal_years.number,
            self.fiscal_years.num_pages,
        );
        data_table_list_refresh::<FiscalYearTableKey>(
            "Fiscal Years",
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

impl RenderAppPane for FiscalYearListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &self.path_and_query,
            fiscal_years_list_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(fiscal_years_list_crumbs(), self.body())
    }
}

impl RenderTemplate for FiscalYearListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Fiscal Years",
            chrome,
            fiscal_years_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct FiscalYearDetailPage {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub start: String,
    pub end: String,
    pub is_active: bool,
    pub can_edit: bool,
}

impl FiscalYearDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label("Code", field_text(FieldText { value: &self.code, classes: "" })))
                    (label("Start", field_date(FieldDate { value: &self.start, classes: "" })))
                    (label("End", field_date(FieldDate { value: &self.end, classes: "" })))
                    (label("Active", field_checkbox(FieldCheckbox {
                        checked: self.is_active,
                        classes: "",
                    })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_finance_fiscal_year.FiscalYearEditForm",
                                href: &FiscalYearEditGetRouteTag::new(self.id).url(),
                                form_post_url: &FiscalYearEditPostRouteTag::new(self.id).path(),
                                modal_uid: FiscalYearEditModalKey::ID,
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
        fiscal_year_detail_menu(self.id, &self.name)
    }
}

impl RenderAppPane for FiscalYearDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = fiscal_year_crumbs(self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(fiscal_year_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for FiscalYearDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = fiscal_year_crumbs(self.id, &self.name, None);
        app_scaffold_with_sidebar("Fiscal Year", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct FiscalYearEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub code: String,
    pub name: String,
    pub start: String,
    pub end: String,
    pub is_active: bool,
    pub error: String,
}

impl RenderTemplate for FiscalYearEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<FiscalYearEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit fiscal year" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<FiscalYearEditModalKey>(&modal_edit_post_url(
                        FiscalYearEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: FiscalYearForm::render_inputs(
                        &FormCtx::form::<FiscalYearForm>()
                            .value(FiscalYearFormField::Code, &self.code)
                            .value(FiscalYearFormField::Name, &self.name)
                            .value(FiscalYearFormField::Start, &self.start)
                            .value(FiscalYearFormField::End, &self.end)
                            .value(
                                FiscalYearFormField::IsActive,
                                if self.is_active { "on" } else { "" },
                            ),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_delete_post_route(
                            FiscalYearDeletePostRouteTag::new(self.id),
                            ButtonDeletePost {
                                label: "Delete",
                                confirm: "Permanently delete this fiscal year?",
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
pub struct FiscalYearCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub code: String,
    pub name: String,
    pub start: String,
    pub end: String,
    pub is_active: bool,
    pub error: String,
}

impl RenderTemplate for FiscalYearCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_fiscal_year.FiscalYearCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<FiscalYearCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Fiscal Year",
                subtitle: "Create a new fiscal year",
                attrs: form_hx_post_url::<FiscalYearCreateModalKey>(&modal_create_post_url(
                    FiscalYearCreatePostRouteTag,
                    form_name,
                    &self.refresh_table,
                )),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: FiscalYearForm::render_inputs(
                    &FormCtx::form::<FiscalYearForm>()
                        .value(FiscalYearFormField::Code, &self.code)
                        .value(FiscalYearFormField::Name, &self.name)
                        .value(FiscalYearFormField::Start, &self.start)
                        .value(FiscalYearFormField::End, &self.end)
                        .value(
                            FiscalYearFormField::IsActive,
                            if self.is_active { "on" } else { "" },
                        ),
                ),
                actions: html! {
                    (container_row("flex justify-end gap-2 mt-2", html! {
                        (button_submit(ButtonSubmit {
                            label: "Save Fiscal Year",
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
pub struct FiscalYearSelectPage {
    pub fiscal_years: ObjectList<FiscalYearRow>,
    pub filter_code: String,
    pub filter_name: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<FiscalYearSelectTableKey, FiscalYearSelectModalKey>
    for FiscalYearSelectPage
{
    fn render_table(&self) -> Markup {
        let code_sort = column_sort_url(&self.path_and_query, "Code", &self.sort);
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let start_sort = column_sort_url(&self.path_and_query, "Start", &self.sort);
        let end_sort = column_sort_url(&self.path_and_query, "End", &self.sort);
        let code_label = format!("Code{}", sort_indicator(&self.sort, "Code"));
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let start_label = format!("Start{}", sort_indicator(&self.sort, "Start"));
        let end_label = format!("End{}", sort_indicator(&self.sort, "End"));
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
                key: "Start",
                label: &start_label,
                sort_url: Some(&start_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "End",
                label: &end_label,
                sort_url: Some(&end_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .fiscal_years
            .items
            .iter()
            .map(|fy| TableRow {
                attrs: row_attr_select(&self.target_input, &fy.id.to_string(), &fy.name),
                cells: vec![
                    field_text(FieldText {
                        value: &fy.code,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &fy.name,
                        classes: "",
                    }),
                    field_date(FieldDate {
                        value: &fy.start,
                        classes: "",
                    }),
                    field_date(FieldDate {
                        value: &fy.end,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let pagination = render_pagination::<FiscalYearSelectTableKey>(
            &self.path_and_query,
            self.fiscal_years.number,
            self.fiscal_years.num_pages,
        );
        data_table_list_refresh::<FiscalYearSelectTableKey>(
            "Select Fiscal Year",
            table_button_filter(TableButtonFilter {
                panel: fiscal_year_select_filter_form(
                    &self.filter_code,
                    &self.filter_name,
                    &self.target_input,
                ),
                ..Default::default()
            }),
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for FiscalYearSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}
