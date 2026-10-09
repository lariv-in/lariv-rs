use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonClear, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldDatetime,
    FieldText, FieldTextarea, FormOpts, ObjectList, PaginationPage, ShellChrome,
    SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_clear,
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
        ApprovedOvertimeFilterFormFlag, ApprovedOvertimeForm, ApprovedOvertimeFormField,
        OvertimeApplicationFilterForm, OvertimeApplicationFilterFormField,
        OvertimeApplicationFilterFormFlag, OvertimeApplicationForm, OvertimeApplicationFormField,
        OvertimeApplicationFormFlag, RejectOvertimeForm, RevokeOvertimeApprovalForm,
        RevokeOvertimeRejectionForm,
    },
    keys::{
        ApprovedOvertimeCreateModalKey, ApprovedOvertimeTableKey, OvertimeApproveModalKey,
        OvertimeCreateModalKey, OvertimeDeleteModalKey, OvertimeEditModalKey,
        OvertimeRejectModalKey, OvertimeRevokeApprovalModalKey, OvertimeRevokeRejectionModalKey,
        OvertimeTableKey,
    },
    logic::overtime::{FILTER_APPROVED, FILTER_PENDING, FILTER_REJECTED, STATUS_PENDING},
    routes::{
        OvertimeApproveGetRouteTag, OvertimeApprovePostRouteTag, OvertimeApprovedCreateGetRouteTag,
        OvertimeApprovedCreatePostRouteTag, OvertimeApprovedRouteTag, OvertimeCreateGetRouteTag,
        OvertimeCreatePostRouteTag, OvertimeDeleteGetRouteTag, OvertimeDeletePostRouteTag,
        OvertimeEditGetRouteTag, OvertimeEditPostRouteTag, OvertimeRejectGetRouteTag,
        OvertimeRejectPostRouteTag, OvertimeRevokeApprovalGetRouteTag,
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

fn overtime_form_inputs(
    user_id: &str,
    user_display: &str,
    show_user: bool,
    start_time: &str,
    end_time: &str,
    reason: &str,
) -> Markup {
    OvertimeApplicationForm::render_inputs(
        &FormCtx::form::<OvertimeApplicationForm>(CsrfToken::current())
            .flag(OvertimeApplicationFormFlag::PickUser, show_user)
            .value(OvertimeApplicationFormField::UserId, user_id)
            .display(OvertimeApplicationFormField::UserId, user_display)
            .value(OvertimeApplicationFormField::StartTime, start_time)
            .value(OvertimeApplicationFormField::EndTime, end_time)
            .value(OvertimeApplicationFormField::Reason, reason),
    )
}

fn overtime_section_tabs(section_base: &str, approved_href: &str, active: &str) -> Markup {
    let pending = format!("{section_base}?tab={FILTER_PENDING}");
    let rejected = format!("{section_base}?tab={FILTER_REJECTED}");
    html! {
        div class="tabs tabs-boxed mb-4" {
            (super::tab_nav_link(&pending, active == FILTER_PENDING, "Pending"))
            (super::tab_nav_link(&rejected, active == FILTER_REJECTED, "Rejected"))
            (super::tab_nav_link(approved_href, active == FILTER_APPROVED, "Approved"))
        }
    }
}

#[derive(Generic)]
pub struct OvertimeListPage {
    pub rows: ObjectList<OvertimeRow>,
    pub filter_user_id: String,
    pub filter_user_display: String,
    pub filter_start_time: String,
    pub filter_end_time: String,
    pub filter_reason: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
    pub title: String,
    pub menu_active: String,
    pub show_create: bool,
    pub show_user_filter: bool,
    /// Pending, approved, and rejected. The approval queue stays on pending only.
    pub show_tabs: bool,
    pub tab: String,
    pub filter_path: String,
    pub approved_href: String,
}

impl OvertimeListPage {
    fn filter_url(&self) -> String {
        if self.show_tabs {
            format!("{}?tab={}", self.filter_path, self.tab)
        } else {
            self.filter_path.clone()
        }
    }

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
        let filter_url = self.filter_url();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_url::<OvertimeTableKey>(&filter_url),
                    inputs: with_list_filter_common(
                        OvertimeApplicationFilterForm::render_inputs(
                            &FormCtx::form::<OvertimeApplicationFilterForm>(CsrfToken::current())
                                .flag(
                                    OvertimeApplicationFilterFormFlag::AnyUser,
                                    self.show_user_filter,
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

    fn body(&self) -> Markup {
        html! {
            @if self.show_tabs {
                (overtime_section_tabs(&self.filter_path, &self.approved_href, &self.tab))
            }
            (self.render_table())
        }
    }
}

impl RenderTemplate for OvertimeListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Overtime — Lariv",
            chrome,
            hr_menu(&self.menu_active),
            overtime_list_crumbs(&self.title),
            self.body(),
        )
    }
}

impl RenderAppPane for OvertimeListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu(&self.menu_active),
            overtime_list_crumbs(&self.title),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(overtime_list_crumbs(&self.title), self.body())
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
    pub user_id: String,
    pub user_display: String,
    pub show_user: bool,
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
            user_id: String::new(),
            user_display: String::new(),
            show_user: false,
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
        user_display: String,
        show_user: bool,
        error: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            user_id: if form.user_id > 0 {
                form.user_id.to_string()
            } else {
                String::new()
            },
            user_display,
            show_user,
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
                    inputs: overtime_form_inputs(
                        &self.user_id,
                        &self.user_display,
                        self.show_user,
                        &self.start_time,
                        &self.end_time,
                        &self.reason,
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

pub struct OvertimeEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub user_id: String,
    pub user_display: String,
    pub show_user: bool,
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
                    inputs: overtime_form_inputs(
                        &self.user_id,
                        &self.user_display,
                        self.show_user,
                        &self.start_time,
                        &self.end_time,
                        &self.reason,
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

#[derive(Clone)]
pub struct ApprovedOvertimeRow {
    pub user: String,
    pub start_time: String,
    pub end_time: String,
    pub approved_by: String,
    pub approved_at: String,
    pub detail_href: Option<String>,
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
    pub show_create: bool,
    pub section_base: String,
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
        let mut actions = html! {
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
        if self.show_create {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.ApprovedOvertimeCreateForm",
                    href: &OvertimeApprovedCreateGetRouteTag.url(),
                    form_post_url: &OvertimeApprovedCreateGetRouteTag.path(),
                    modal_uid: ApprovedOvertimeCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
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

    fn body(&self) -> Markup {
        html! {
            (overtime_section_tabs(
                &self.section_base,
                &OvertimeApprovedRouteTag.url(),
                FILTER_APPROVED,
            ))
            (self.render_table())
        }
    }
}

impl RenderTemplate for ApprovedOvertimeListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Approved overtime — Lariv",
            chrome,
            hr_menu("overtime"),
            overtime_list_crumbs("Approved overtime"),
            self.body(),
        )
    }
}

impl RenderAppPane for ApprovedOvertimeListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("overtime"),
            overtime_list_crumbs("Approved overtime"),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(overtime_list_crumbs("Approved overtime"), self.body())
    }
}

pub struct ApprovedOvertimeCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub user_id: String,
    pub user_display: String,
    pub start_time: String,
    pub end_time: String,
    pub approved_by_id: String,
    pub approved_by_display: String,
    pub approved_at: String,
    pub error: String,
}

impl ApprovedOvertimeCreateModalPage {
    pub fn new(form_name: String, refresh_table: String) -> Self {
        Self {
            form_name,
            refresh_table,
            user_id: String::new(),
            user_display: String::new(),
            start_time: String::new(),
            end_time: String::new(),
            approved_by_id: String::new(),
            approved_by_display: String::new(),
            approved_at: String::new(),
            error: String::new(),
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        form: &ApprovedOvertimeForm,
        user_display: String,
        approved_by_display: String,
        error: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            user_id: positive_id(form.user_id),
            user_display,
            start_time: form.start_time.clone(),
            end_time: form.end_time.clone(),
            approved_by_id: positive_id(form.approved_by_id),
            approved_by_display,
            approved_at: form.approved_at.clone(),
            error,
        }
    }
}

fn positive_id(id: i64) -> String {
    if id > 0 {
        id.to_string()
    } else {
        String::new()
    }
}

impl RenderTemplate for ApprovedOvertimeCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<ApprovedOvertimeCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New approved overtime" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<ApprovedOvertimeCreateModalKey>(&modal_create_post_url(
                        OvertimeApprovedCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ApprovedOvertimeForm::render_inputs(
                        &FormCtx::form::<ApprovedOvertimeForm>(CsrfToken::current())
                            .value(ApprovedOvertimeFormField::UserId, &self.user_id)
                            .display(ApprovedOvertimeFormField::UserId, &self.user_display)
                            .value(ApprovedOvertimeFormField::StartTime, &self.start_time)
                            .value(ApprovedOvertimeFormField::EndTime, &self.end_time)
                            .value(ApprovedOvertimeFormField::ApprovedById, &self.approved_by_id)
                            .display(
                                ApprovedOvertimeFormField::ApprovedById,
                                &self.approved_by_display,
                            )
                            .value(ApprovedOvertimeFormField::ApprovedAt, &self.approved_at),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_page(tab: &str) -> OvertimeListPage {
        OvertimeListPage {
            rows: ObjectList::from_page(Vec::new(), 1, 20, 0),
            filter_user_id: String::new(),
            filter_user_display: String::new(),
            filter_start_time: String::new(),
            filter_end_time: String::new(),
            filter_reason: String::new(),
            sort: String::new(),
            path_and_query: format!("/hr/overtime?tab={tab}"),
            page_size: 20,
            title: "Overtime".into(),
            menu_active: "overtime".into(),
            show_create: false,
            show_user_filter: false,
            show_tabs: true,
            tab: tab.into(),
            filter_path: "/hr/overtime".into(),
            approved_href: "/hr/overtime/approved".into(),
        }
    }

    #[test]
    fn list_uses_status_tabs() {
        let pending = sample_page(FILTER_PENDING).body().into_string();
        assert!(pending.contains("tabs tabs-boxed"));
        assert!(pending.contains("Pending"));
        assert!(pending.contains("Rejected"));
        assert!(pending.contains("/hr/overtime/approved"));
        assert!(pending.contains("tab=pending"));
        assert!(pending.contains("tab=rejected"));
        assert!(!pending.contains("tab=approved"));

        let rejected = sample_page(FILTER_REJECTED);
        let html = rejected.body().into_string();
        assert!(html.contains("tab-active"));
        assert!(!html.contains("OvertimeCreateForm"));
        assert!(!html.contains("ApprovedOvertimeCreateForm"));
    }

    #[test]
    fn approved_list_create_is_superuser_only() {
        let page = ApprovedOvertimeListPage {
            rows: ObjectList::from_page(Vec::new(), 1, 20, 0),
            filter_user_id: String::new(),
            filter_user_display: String::new(),
            filter_start_time: String::new(),
            filter_end_time: String::new(),
            sort: String::new(),
            path_and_query: "/hr/overtime/approved".into(),
            page_size: 20,
            show_user_filter: false,
            show_create: false,
            section_base: "/hr/overtime/applications".into(),
        };
        let html = page.body().into_string();
        assert!(!html.contains("ApprovedOvertimeCreateForm"));
        assert!(html.contains("Approved overtime"));
    }
}
