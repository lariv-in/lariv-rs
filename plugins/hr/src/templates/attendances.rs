use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonClear, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldDatetime,
    FieldText, FormOpts, ObjectList, PaginationPage, ShellChrome, SwapKey, TableButtonFilter,
    TableColumnHeader, TablePagination, TableRow, button_clear, button_modal_form, button_submit,
    column_sort_url, container_column, container_row, data_table_list_refresh, delete_confirmation,
    detail, detail_header, field_datetime, field_text, form, form_hx_get_route,
    form_hx_post_selector, form_hx_post_url, label, modal, modal_keyed, pagination_pages,
    row_attr_navigate, sort_indicator, table_button_filter, table_pagination,
    with_list_filter_common,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::modal_create_post_url;

use crate::{
    crumbs::{attendance_crumbs, attendances_list_crumbs},
    forms::{AttendanceFilterForm, AttendanceFilterFormField, AttendanceForm, AttendanceFormField},
    keys::{
        AttendanceCreateModalKey, AttendanceDeleteModalKey, AttendanceEditModalKey,
        AttendanceTableKey,
    },
    routes::{
        AttendanceCreateGetRouteTag, AttendanceCreatePostRouteTag, AttendanceDeleteGetRouteTag,
        AttendanceDeletePostRouteTag, AttendanceEditGetRouteTag, AttendanceEditPostRouteTag,
        AttendanceListRouteTag,
    },
    templates::{app_scaffold, hr_menu, scaffold_main, scaffold_pane},
};

#[derive(Clone)]
pub struct AttendanceRow {
    pub id: i64,
    pub user: String,
    pub started_at: String,
    pub ended_at: String,
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

fn fk_value(id: i64) -> String {
    if id <= 0 {
        String::new()
    } else {
        id.to_string()
    }
}

fn attendance_form_inputs(
    user_id: i64,
    user_display: &str,
    started_at: &str,
    ended_at: &str,
) -> Markup {
    let user_id = fk_value(user_id);
    AttendanceForm::render_inputs(
        &FormCtx::form::<AttendanceForm>(CsrfToken::current())
            .value(AttendanceFormField::UserId, user_id.as_str())
            .display(AttendanceFormField::UserId, user_display)
            .value(AttendanceFormField::StartedAt, started_at)
            .value(AttendanceFormField::EndedAt, ended_at),
    )
}

#[derive(Generic)]
pub struct AttendanceListPage {
    pub rows: ObjectList<AttendanceRow>,
    pub filter_user_id: String,
    pub filter_user_display: String,
    pub filter_started_at: String,
    pub filter_ended_at: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl AttendanceListPage {
    pub fn render_table(&self) -> Markup {
        let user_sort = column_sort_url(&self.path_and_query, "User", &self.sort);
        let start_sort = column_sort_url(&self.path_and_query, "StartedAt", &self.sort);
        let end_sort = column_sort_url(&self.path_and_query, "EndedAt", &self.sort);
        let user_label = format!("User{}", sort_indicator(&self.sort, "User"));
        let start_label = format!("Start{}", sort_indicator(&self.sort, "StartedAt"));
        let end_label = format!("End{}", sort_indicator(&self.sort, "EndedAt"));
        let headers = [
            TableColumnHeader {
                key: "User",
                label: &user_label,
                sort_url: Some(&user_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "StartedAt",
                label: &start_label,
                sort_url: Some(&start_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "EndedAt",
                label: &end_label,
                sort_url: Some(&end_sort),
                push_url: true,
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
                        value: &row.user,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.started_at,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.ended_at,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<AttendanceTableKey, AttendanceListRouteTag>(
                        AttendanceListRouteTag,
                    ),
                    inputs: with_list_filter_common(
                        AttendanceFilterForm::render_inputs(
                            &FormCtx::form::<AttendanceFilterForm>(CsrfToken::current())
                                .value(AttendanceFilterFormField::UserId, &self.filter_user_id)
                                .display(
                                    AttendanceFilterFormField::UserId,
                                    &self.filter_user_display,
                                )
                                .value(
                                    AttendanceFilterFormField::StartedAt,
                                    &self.filter_started_at,
                                )
                                .value(AttendanceFilterFormField::EndedAt, &self.filter_ended_at),
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
            &lariv_plugin_users::role_authorization::roles_for::<
                super::super::routes::AttendanceMutate,
            >(),
        ) {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.AttendanceCreateForm",
                    href: &AttendanceCreateGetRouteTag.url(),
                    form_post_url: &AttendanceCreateGetRouteTag.path(),
                    modal_uid: AttendanceCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        data_table_list_refresh::<AttendanceTableKey>(
            "Attendance",
            actions,
            &headers,
            &table_rows,
            render_pagination::<AttendanceTableKey>(
                &self.path_and_query,
                self.rows.number,
                self.rows.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for AttendanceListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Attendance — Lariv",
            chrome,
            hr_menu("attendance"),
            attendances_list_crumbs(),
            self.render_table(),
        )
    }
}

impl RenderAppPane for AttendanceListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("attendance"),
            attendances_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(attendances_list_crumbs(), self.render_table())
    }
}

#[derive(Generic)]
pub struct AttendanceDetailPage {
    pub id: i64,
    pub user: String,
    pub started_at: String,
    pub ended_at: String,
}

impl AttendanceDetailPage {
    fn body(&self) -> Markup {
        let actions = if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                super::super::routes::AttendanceMutate,
            >(),
        ) {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.AttendanceEditForm",
                    href: &AttendanceEditGetRouteTag::new(self.id).url(),
                    form_post_url: &AttendanceEditPostRouteTag::new(self.id).path(),
                    modal_uid: AttendanceEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.AttendanceDeleteForm",
                    href: &AttendanceDeleteGetRouteTag::new(self.id).url(),
                    form_post_url: &AttendanceDeleteGetRouteTag::new(self.id).path(),
                    modal_uid: AttendanceDeleteModalKey::ID,
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
                        title: &self.user,
                        actions,
                    }))
                    (label("User", field_text(FieldText { value: &self.user, classes: "" })))
                    (label("Start", field_datetime(FieldDatetime { value: &self.started_at, classes: "" })))
                    (label("End", field_datetime(FieldDatetime { value: &self.ended_at, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderTemplate for AttendanceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Attendance — Lariv",
            chrome,
            hr_menu("attendance"),
            attendance_crumbs(&self.user),
            self.body(),
        )
    }
}

impl RenderAppPane for AttendanceDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("attendance"),
            attendance_crumbs(&self.user),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(attendance_crumbs(&self.user), self.body())
    }
}

pub struct AttendanceCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub user_id: i64,
    pub user_display: String,
    pub started_at: String,
    pub ended_at: String,
    pub error: String,
}

impl AttendanceCreateModalPage {
    pub fn new(form_name: String, refresh_table: String) -> Self {
        Self {
            form_name,
            refresh_table,
            user_id: 0,
            user_display: String::new(),
            started_at: String::new(),
            ended_at: String::new(),
            error: String::new(),
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        form: &AttendanceForm,
        user_display: String,
        error: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            user_id: form.user_id,
            user_display,
            started_at: form.started_at.clone(),
            ended_at: form.ended_at.clone(),
            error,
        }
    }
}

impl RenderTemplate for AttendanceCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<AttendanceCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New attendance" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<AttendanceCreateModalKey>(&modal_create_post_url(
                        AttendanceCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: attendance_form_inputs(
                        self.user_id,
                        &self.user_display,
                        &self.started_at,
                        &self.ended_at,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct AttendanceEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub user_id: i64,
    pub user_display: String,
    pub started_at: String,
    pub ended_at: String,
    pub error: String,
}

impl RenderTemplate for AttendanceEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<AttendanceEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit attendance" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<AttendanceEditModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: attendance_form_inputs(
                        self.user_id,
                        &self.user_display,
                        &self.started_at,
                        &self.ended_at,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct AttendanceDeleteModalPage {
    pub id: i64,
    pub form_name: String,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for AttendanceDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", AttendanceDeleteModalKey::ID);
        let post_url = AttendanceDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid: AttendanceDeleteModalKey::ID,
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
