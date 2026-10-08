use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonClear, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldDate,
    FieldText, FieldTextarea, FormOpts, ObjectList, PaginationPage, ShellChrome, SwapKey,
    TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_clear,
    button_modal_form, button_submit, column_sort_url, container_column, container_row,
    data_table_list_refresh, delete_confirmation, detail, detail_header, field_date, field_text,
    field_textarea, form, form_hx_get_route, form_hx_post_selector, form_hx_post_url, label, modal,
    modal_keyed, pagination_pages, row_attr_navigate, sort_indicator, table_button_filter,
    table_pagination, with_list_filter_common,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::modal_create_post_url;

use crate::{
    crumbs::{holiday_crumbs, holidays_list_crumbs},
    forms::{HolidayFilterForm, HolidayFilterFormField, HolidayForm, HolidayFormField},
    keys::{HolidayCreateModalKey, HolidayDeleteModalKey, HolidayEditModalKey, HolidayTableKey},
    routes::{
        HolidayCreateGetRouteTag, HolidayCreatePostRouteTag, HolidayDeleteGetRouteTag,
        HolidayDeletePostRouteTag, HolidayEditGetRouteTag, HolidayEditPostRouteTag,
        HolidayListRouteTag,
    },
    templates::{app_scaffold, hr_menu, scaffold_main, scaffold_pane},
};

#[derive(Clone)]
pub struct HolidayRow {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub date: String,
    pub detail_href: String,
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

fn holiday_form_inputs(title: &str, description: &str, date: &str) -> Markup {
    HolidayForm::render_inputs(
        &FormCtx::form::<HolidayForm>(CsrfToken::current())
            .value(HolidayFormField::Title, title)
            .value(HolidayFormField::Description, description)
            .value(HolidayFormField::Date, date),
    )
}

#[derive(Generic)]
pub struct HolidayListPage {
    pub rows: ObjectList<HolidayRow>,
    pub filter_title: String,
    pub filter_date: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl HolidayListPage {
    pub fn render_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let date_sort = column_sort_url(&self.path_and_query, "Date", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let date_label = format!("Date{}", sort_indicator(&self.sort, "Date"));
        let headers = [
            TableColumnHeader {
                key: "Title",
                label: &title_label,
                sort_url: Some(&title_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Date",
                label: &date_label,
                sort_url: Some(&date_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Description",
                label: "Description",
                sort_url: None,
                push_url: false,
            },
        ];
        let table_rows: Vec<TableRow> = self
            .rows
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row_attr_navigate(&row.detail_href),
                cells: vec![
                    field_text(FieldText {
                        value: &row.title,
                        classes: "",
                    }),
                    field_date(FieldDate {
                        value: &row.date,
                        classes: "",
                    }),
                    field_textarea(FieldTextarea {
                        value: &row.description,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<HolidayTableKey, HolidayListRouteTag>(
                        HolidayListRouteTag,
                    ),
                    inputs: with_list_filter_common(
                        HolidayFilterForm::render_inputs(
                            &FormCtx::form::<HolidayFilterForm>(CsrfToken::current())
                                .value(HolidayFilterFormField::Title, &self.filter_title)
                                .value(HolidayFilterFormField::Date, &self.filter_date),
                        ),
                        self.page_size,
                    ),
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
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<super::super::routes::HolidayMutate>(
            ),
        ) {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.HolidayCreateForm",
                    href: &HolidayCreateGetRouteTag.url(),
                    form_post_url: &HolidayCreateGetRouteTag.path(),
                    modal_uid: HolidayCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        data_table_list_refresh::<HolidayTableKey>(
            "Holidays",
            actions,
            &headers,
            &table_rows,
            render_pagination::<HolidayTableKey>(
                &self.path_and_query,
                self.rows.number,
                self.rows.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for HolidayListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Holidays — Lariv",
            chrome,
            hr_menu("holidays"),
            holidays_list_crumbs(),
            self.render_table(),
        )
    }
}

impl RenderAppPane for HolidayListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("holidays"),
            holidays_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(holidays_list_crumbs(), self.render_table())
    }
}

#[derive(Generic)]
pub struct HolidayDetailPage {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub date: String,
}

impl HolidayDetailPage {
    fn body(&self) -> Markup {
        let actions = if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<super::super::routes::HolidayMutate>(
            ),
        ) {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.HolidayEditForm",
                    href: &HolidayEditGetRouteTag::new(self.id).url(),
                    form_post_url: &HolidayEditPostRouteTag::new(self.id).path(),
                    modal_uid: HolidayEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.HolidayDeleteForm",
                    href: &HolidayDeleteGetRouteTag::new(self.id).url(),
                    form_post_url: &HolidayDeleteGetRouteTag::new(self.id).path(),
                    modal_uid: HolidayDeleteModalKey::ID,
                    label: "Delete",
                    classes: "btn-outline btn-error btn-sm",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.title,
                        actions,
                    }))
                    (label("Title", field_text(FieldText { value: &self.title, classes: "" })))
                    (label("Date", field_date(FieldDate { value: &self.date, classes: "" })))
                    (label("Description", field_textarea(FieldTextarea { value: &self.description, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderTemplate for HolidayDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Holiday — Lariv",
            chrome,
            hr_menu("holidays"),
            holiday_crumbs(&self.title),
            self.body(),
        )
    }
}

impl RenderAppPane for HolidayDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("holidays"),
            holiday_crumbs(&self.title),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(holiday_crumbs(&self.title), self.body())
    }
}

pub struct HolidayCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub title: String,
    pub description: String,
    pub date: String,
    pub error: String,
}

impl HolidayCreateModalPage {
    pub fn new(form_name: String, refresh_table: String) -> Self {
        Self {
            form_name,
            refresh_table,
            title: String::new(),
            description: String::new(),
            date: String::new(),
            error: String::new(),
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        form: &HolidayForm,
        error: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            title: form.title.clone(),
            description: form.description.clone(),
            date: form.date.clone(),
            error,
        }
    }
}

impl RenderTemplate for HolidayCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<HolidayCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New holiday" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<HolidayCreateModalKey>(&modal_create_post_url(
                        HolidayCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: holiday_form_inputs(&self.title, &self.description, &self.date),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct HolidayEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub title: String,
    pub description: String,
    pub date: String,
    pub error: String,
}

impl RenderTemplate for HolidayEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<HolidayEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit holiday" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<HolidayEditModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: holiday_form_inputs(&self.title, &self.description, &self.date),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct HolidayDeleteModalPage {
    pub id: i64,
    pub form_name: String,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for HolidayDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", HolidayDeleteModalKey::ID);
        let post_url = HolidayDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid: HolidayDeleteModalKey::ID,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
