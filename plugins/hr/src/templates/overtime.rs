use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonClear, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldDatetime,
    FieldText, FieldTextarea, FormOpts, ObjectList, PaginationPage, ShellChrome, SwapKey,
    TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_clear, button_link_url,
    button_modal_form, button_submit, column_sort_url, container_row, data_table_list_refresh,
    delete_confirmation, detail, detail_header, field_datetime, field_text, field_textarea, form,
    form_hx_get_url, form_hx_post_selector, form_hx_post_url, label, modal, modal_keyed,
    pagination_pages, row_attr_navigate, sort_indicator, table_button_filter, table_pagination,
    with_list_filter_common,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::modal_create_post_url;

use crate::{
    crumbs::{overtime_crumbs, overtime_list_crumbs},
    forms::{
        ApproveOvertimeForm, ApprovedOvertimeFilterForm, ApprovedOvertimeFilterFormField,
        ApprovedOvertimeFilterFormFlag, OvertimeApplicationFilterForm,
        OvertimeApplicationFilterFormField, OvertimeApplicationFilterFormFlag,
        OvertimeApplicationForm, OvertimeApplicationFormField, RejectOvertimeForm,
        RevokeOvertimeApprovalForm, RevokeOvertimeRejectionForm,
    },
    keys::{
        ApprovedOvertimeTableKey, OvertimeApproveModalKey, OvertimeCreateModalKey,
        OvertimeDeleteModalKey, OvertimeEditModalKey, OvertimeRejectModalKey,
        OvertimeRevokeApprovalModalKey, OvertimeRevokeRejectionModalKey, OvertimeTableKey,
    },
    logic::overtime::STATUS_PENDING,
    routes::{
        OvertimeApproveGetRouteTag, OvertimeApprovePostRouteTag, OvertimeApprovedRouteTag,
        OvertimeCreateGetRouteTag, OvertimeCreatePostRouteTag, OvertimeDeleteGetRouteTag,
        OvertimeDeletePostRouteTag, OvertimeEditGetRouteTag, OvertimeEditPostRouteTag,
        OvertimeRejectGetRouteTag, OvertimeRejectPostRouteTag, OvertimeRevokeApprovalGetRouteTag,
        OvertimeRevokeApprovalPostRouteTag, OvertimeRevokeRejectionGetRouteTag,
        OvertimeRevokeRejectionPostRouteTag,
    },
    templates::{app_scaffold, hr_menu, scaffold_main, scaffold_pane},
};

#[derive(Clone)]
pub struct OvertimeRow {
    pub id: i64,
    pub user: String,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
    pub reason: String,
    pub detail_href: String,
}

#[derive(Clone)]
pub struct ApprovedOvertimeRow {
    pub user: String,
    pub start_time: String,
    pub end_time: String,
    pub approved_by: String,
    pub approved_at: String,
    pub detail_href: Option<String>,
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

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

fn overtime_form_inputs(start_time: &str, end_time: &str, reason: &str) -> Markup {
    OvertimeApplicationForm::render_inputs(
        &FormCtx::form::<OvertimeApplicationForm>(CsrfToken::current())
            .value(OvertimeApplicationFormField::StartTime, start_time)
            .value(OvertimeApplicationFormField::EndTime, end_time)
            .value(OvertimeApplicationFormField::Reason, reason),
    )
}

#[derive(Generic)]
pub struct OvertimeListPage {
    pub rows: ObjectList<OvertimeRow>,
    pub filter_user_id: String,
    pub filter_user_display: String,
    pub filter_start_time: String,
    pub filter_end_time: String,
    pub filter_status: String,
    pub filter_reason: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
    pub title: String,
    pub menu_active: String,
    pub show_create: bool,
    pub show_user_filter: bool,
    pub show_status_filter: bool,
    pub filter_path: String,
    pub approved_href: String,
}

impl OvertimeListPage {
    pub fn render_table(&self) -> Markup {
        let user_sort = column_sort_url(&self.path_and_query, "User", &self.sort);
        let start_sort = column_sort_url(&self.path_and_query, "Start", &self.sort);
        let end_sort = column_sort_url(&self.path_and_query, "End", &self.sort);
        let user_label = format!("User{}", sort_indicator(&self.sort, "User"));
        let start_label = format!("Start{}", sort_indicator(&self.sort, "Start"));
        let end_label = format!("End{}", sort_indicator(&self.sort, "End"));
        let headers = [
            TableColumnHeader {
                key: "User",
                label: &user_label,
                sort_url: Some(&user_sort),
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
                key: "Status",
                label: "Status",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Reason",
                label: "Reason",
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
                        value: &row.user,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.start_time,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.end_time,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.status,
                        classes: "",
                    }),
                    field_textarea(FieldTextarea {
                        value: &row.reason,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let statuses = choice_pairs(OvertimeApplicationFilterForm::status_choices());
        let mut actions = html! {
            (button_link_url(&self.approved_href, "Approved overtime", "btn-outline btn-sm"))
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_url::<OvertimeTableKey>(&self.filter_path),
                    inputs: with_list_filter_common(
                        OvertimeApplicationFilterForm::render_inputs(
                            &FormCtx::form::<OvertimeApplicationFilterForm>(CsrfToken::current())
                                .flag(
                                    OvertimeApplicationFilterFormFlag::AnyUser,
                                    self.show_user_filter,
                                )
                                .flag(
                                    OvertimeApplicationFilterFormFlag::AnyStatus,
                                    self.show_status_filter,
                                )
                                .value(
                                    OvertimeApplicationFilterFormField::UserId,
                                    &self.filter_user_id,
                                )
                                .display(
                                    OvertimeApplicationFilterFormField::UserId,
                                    &self.filter_user_display,
                                )
                                .value(
                                    OvertimeApplicationFilterFormField::StartTime,
                                    &self.filter_start_time,
                                )
                                .value(
                                    OvertimeApplicationFilterFormField::EndTime,
                                    &self.filter_end_time,
                                )
                                .value(
                                    OvertimeApplicationFilterFormField::Status,
                                    &self.filter_status,
                                )
                                .choices(OvertimeApplicationFilterFormField::Status, &statuses)
                                .value(
                                    OvertimeApplicationFilterFormField::Reason,
                                    &self.filter_reason,
                                ),
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
        if self.show_create
            && lariv_core::components::role_permitted(
                &lariv_plugin_users::role_authorization::roles_for::<
                    super::super::routes::OvertimeView,
                >(),
            )
        {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.OvertimeCreateForm",
                    href: &OvertimeCreateGetRouteTag.url(),
                    form_post_url: &OvertimeCreateGetRouteTag.path(),
                    modal_uid: OvertimeCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        data_table_list_refresh::<OvertimeTableKey>(
            &self.title,
            actions,
            &headers,
            &table_rows,
            render_pagination::<OvertimeTableKey>(
                &self.path_and_query,
                self.rows.number,
                self.rows.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for OvertimeListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Overtime — Lariv",
            chrome,
            hr_menu(&self.menu_active),
            overtime_list_crumbs(&self.title),
            self.render_table(),
        )
    }
}

impl RenderAppPane for OvertimeListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu(&self.menu_active),
            overtime_list_crumbs(&self.title),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(overtime_list_crumbs(&self.title), self.render_table())
    }
}

#[derive(Generic)]
pub struct ApprovedOvertimeListPage {
    pub rows: ObjectList<ApprovedOvertimeRow>,
    pub filter_user_id: String,
    pub filter_user_display: String,
    pub filter_start_time: String,
    pub filter_end_time: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
    pub show_user_filter: bool,
    pub applications_href: String,
}

impl ApprovedOvertimeListPage {
    pub fn render_table(&self) -> Markup {
        let user_sort = column_sort_url(&self.path_and_query, "User", &self.sort);
        let start_sort = column_sort_url(&self.path_and_query, "Start", &self.sort);
        let end_sort = column_sort_url(&self.path_and_query, "End", &self.sort);
        let approved_sort = column_sort_url(&self.path_and_query, "ApprovedAt", &self.sort);
        let user_label = format!("User{}", sort_indicator(&self.sort, "User"));
        let start_label = format!("Start{}", sort_indicator(&self.sort, "Start"));
        let end_label = format!("End{}", sort_indicator(&self.sort, "End"));
        let approved_label = format!("Approved at{}", sort_indicator(&self.sort, "ApprovedAt"));
        let headers = [
            TableColumnHeader {
                key: "User",
                label: &user_label,
                sort_url: Some(&user_sort),
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
                key: "ApprovedBy",
                label: "Approved by",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "ApprovedAt",
                label: &approved_label,
                sort_url: Some(&approved_sort),
                push_url: true,
            },
        ];
        let table_rows: Vec<TableRow> = self
            .rows
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row
                    .detail_href
                    .as_deref()
                    .map(row_attr_navigate)
                    .unwrap_or_default(),
                cells: vec![
                    field_text(FieldText {
                        value: &row.user,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.start_time,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.end_time,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.approved_by,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.approved_at,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let actions = html! {
            (button_link_url(&self.applications_href, "Overtime", "btn-outline btn-sm"))
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_url::<ApprovedOvertimeTableKey>(&OvertimeApprovedRouteTag.path()),
                    inputs: with_list_filter_common(
                        ApprovedOvertimeFilterForm::render_inputs(
                            &FormCtx::form::<ApprovedOvertimeFilterForm>(CsrfToken::current())
                                .flag(
                                    ApprovedOvertimeFilterFormFlag::AnyUser,
                                    self.show_user_filter,
                                )
                                .value(
                                    ApprovedOvertimeFilterFormField::UserId,
                                    &self.filter_user_id,
                                )
                                .display(
                                    ApprovedOvertimeFilterFormField::UserId,
                                    &self.filter_user_display,
                                )
                                .value(
                                    ApprovedOvertimeFilterFormField::StartTime,
                                    &self.filter_start_time,
                                )
                                .value(
                                    ApprovedOvertimeFilterFormField::EndTime,
                                    &self.filter_end_time,
                                ),
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
        data_table_list_refresh::<ApprovedOvertimeTableKey>(
            "Approved overtime",
            actions,
            &headers,
            &table_rows,
            render_pagination::<ApprovedOvertimeTableKey>(
                &self.path_and_query,
                self.rows.number,
                self.rows.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for ApprovedOvertimeListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Approved overtime — Lariv",
            chrome,
            hr_menu("overtime"),
            overtime_list_crumbs("Approved overtime"),
            self.render_table(),
        )
    }
}

impl RenderAppPane for ApprovedOvertimeListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("overtime"),
            overtime_list_crumbs("Approved overtime"),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            overtime_list_crumbs("Approved overtime"),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct OvertimeDetailPage {
    pub id: i64,
    pub title: String,
    pub user: String,
    pub start_time: String,
    pub end_time: String,
    pub reason: String,
    pub status: String,
    pub approved_by: String,
    pub approved_at: String,
    pub rejected_by: String,
    pub rejected_at: String,
    pub can_approve: bool,
    pub can_edit: bool,
    pub can_revoke_approval: bool,
    pub can_revoke_rejection: bool,
    pub menu_active: String,
    pub parent_label: String,
    pub parent_href: String,
}

impl OvertimeDetailPage {
    fn body(&self) -> Markup {
        let pending = self.status == STATUS_PENDING;
        let actions = html! {
            @if pending && self.can_approve {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.OvertimeApproveForm",
                    href: &OvertimeApproveGetRouteTag::new(self.id).url(),
                    form_post_url: &OvertimeApprovePostRouteTag::new(self.id).path(),
                    modal_uid: OvertimeApproveModalKey::ID,
                    label: "Approve",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.OvertimeRejectForm",
                    href: &OvertimeRejectGetRouteTag::new(self.id).url(),
                    form_post_url: &OvertimeRejectPostRouteTag::new(self.id).path(),
                    modal_uid: OvertimeRejectModalKey::ID,
                    label: "Reject",
                    classes: "btn-outline btn-error btn-sm",
                    ..Default::default()
                }))
            }
            @if self.can_revoke_approval {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.OvertimeRevokeApprovalForm",
                    href: &OvertimeRevokeApprovalGetRouteTag::new(self.id).url(),
                    form_post_url: &OvertimeRevokeApprovalPostRouteTag::new(self.id).path(),
                    modal_uid: OvertimeRevokeApprovalModalKey::ID,
                    label: "Revoke approval",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
            }
            @if self.can_revoke_rejection {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.OvertimeRevokeRejectionForm",
                    href: &OvertimeRevokeRejectionGetRouteTag::new(self.id).url(),
                    form_post_url: &OvertimeRevokeRejectionPostRouteTag::new(self.id).path(),
                    modal_uid: OvertimeRevokeRejectionModalKey::ID,
                    label: "Revoke rejection",
                    classes: "btn-outline btn-error btn-sm",
                    ..Default::default()
                }))
            }
            @if self.can_edit {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.OvertimeEditForm",
                    href: &OvertimeEditGetRouteTag::new(self.id).url(),
                    form_post_url: &OvertimeEditPostRouteTag::new(self.id).path(),
                    modal_uid: OvertimeEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.OvertimeDeleteForm",
                    href: &OvertimeDeleteGetRouteTag::new(self.id).url(),
                    form_post_url: &OvertimeDeleteGetRouteTag::new(self.id).path(),
                    modal_uid: OvertimeDeleteModalKey::ID,
                    label: "Delete",
                    classes: "btn-outline btn-error btn-sm",
                    ..Default::default()
                }))
            }
        };
        html! {
            (detail(html! {
                (detail_header(DetailHeader {
                    title: &self.title,
                    actions,
                }))
                (label("User", field_text(FieldText { value: &self.user, classes: "" })))
                (label("Start", field_datetime(FieldDatetime { value: &self.start_time, classes: "" })))
                (label("End", field_datetime(FieldDatetime { value: &self.end_time, classes: "" })))
                (label("Reason", field_textarea(FieldTextarea { value: &self.reason, classes: "" })))
                (label("Status", field_text(FieldText { value: &self.status, classes: "" })))
                @if !self.approved_by.is_empty() {
                    (label("Approved by", field_text(FieldText { value: &self.approved_by, classes: "" })))
                    (label("Approved at", field_datetime(FieldDatetime { value: &self.approved_at, classes: "" })))
                }
                @if !self.rejected_by.is_empty() {
                    (label("Rejected by", field_text(FieldText { value: &self.rejected_by, classes: "" })))
                    (label("Rejected at", field_datetime(FieldDatetime { value: &self.rejected_at, classes: "" })))
                }
            }))
        }
    }
}

impl RenderTemplate for OvertimeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Overtime — Lariv",
            chrome,
            hr_menu(&self.menu_active),
            overtime_crumbs(&self.parent_label, &self.parent_href, &self.title),
            self.body(),
        )
    }
}

impl RenderAppPane for OvertimeDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu(&self.menu_active),
            overtime_crumbs(&self.parent_label, &self.parent_href, &self.title),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            overtime_crumbs(&self.parent_label, &self.parent_href, &self.title),
            self.body(),
        )
    }
}

pub struct OvertimeCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub start_time: String,
    pub end_time: String,
    pub reason: String,
    pub error: String,
}

impl OvertimeCreateModalPage {
    pub fn new(form_name: String, refresh_table: String) -> Self {
        Self {
            form_name,
            refresh_table,
            start_time: String::new(),
            end_time: String::new(),
            reason: String::new(),
            error: String::new(),
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        form: &OvertimeApplicationForm,
        error: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            start_time: form.start_time.clone(),
            end_time: form.end_time.clone(),
            reason: form.reason.clone(),
            error,
        }
    }
}

impl RenderTemplate for OvertimeCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<OvertimeCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New overtime" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<OvertimeCreateModalKey>(&modal_create_post_url(
                        OvertimeCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: overtime_form_inputs(&self.start_time, &self.end_time, &self.reason),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct OvertimeEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub start_time: String,
    pub end_time: String,
    pub reason: String,
    pub error: String,
}

impl RenderTemplate for OvertimeEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<OvertimeEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit overtime" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<OvertimeEditModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: overtime_form_inputs(&self.start_time, &self.end_time, &self.reason),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct OvertimeDeleteModalPage {
    pub id: i64,
    pub form_name: String,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for OvertimeDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", OvertimeDeleteModalKey::ID);
        let post_url = OvertimeDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid: OvertimeDeleteModalKey::ID,
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

pub struct OvertimeApproveModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for OvertimeApproveModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<OvertimeApproveModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg" { "Approve overtime" }
                p class="text-sm opacity-80 mb-4" { "This records the approval in your name at the current time." }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<OvertimeApproveModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ApproveOvertimeForm::render_inputs(
                        &FormCtx::form::<ApproveOvertimeForm>(CsrfToken::current()),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Approve", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct OvertimeRejectModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for OvertimeRejectModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<OvertimeRejectModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg" { "Reject overtime" }
                p class="text-sm opacity-80 mb-4" { "This records the rejection in your name at the current time." }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<OvertimeRejectModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: RejectOvertimeForm::render_inputs(
                        &FormCtx::form::<RejectOvertimeForm>(CsrfToken::current()),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Reject", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct OvertimeRevokeApprovalModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for OvertimeRevokeApprovalModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<OvertimeRevokeApprovalModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg" { "Revoke approval" }
                p class="text-sm opacity-80 mb-4" { "This removes the approval and returns the overtime to pending." }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<OvertimeRevokeApprovalModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: RevokeOvertimeApprovalForm::render_inputs(
                        &FormCtx::form::<RevokeOvertimeApprovalForm>(CsrfToken::current()),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Revoke approval", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct OvertimeRevokeRejectionModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for OvertimeRevokeRejectionModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<OvertimeRevokeRejectionModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg" { "Revoke rejection" }
                p class="text-sm opacity-80 mb-4" { "This removes the rejection and returns the overtime to pending." }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<OvertimeRevokeRejectionModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: RevokeOvertimeRejectionForm::render_inputs(
                        &FormCtx::form::<RevokeOvertimeRejectionForm>(CsrfToken::current()),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Revoke rejection", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
