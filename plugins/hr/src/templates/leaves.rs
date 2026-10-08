use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonClear, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldDate,
    FieldDatetime, FieldText, FieldTextarea, FormOpts, ObjectList, PaginationPage, ShellChrome,
    SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_clear,
    button_modal_form, button_submit, column_sort_url, container_column, container_row,
    data_table_list_refresh, delete_confirmation, detail, detail_header, field_date,
    field_datetime, field_text, field_textarea, form, form_hx_get_route, form_hx_post_selector,
    form_hx_post_url, label, modal, modal_keyed, pagination_pages, row_attr_navigate,
    sort_indicator, table_button_filter, table_pagination, with_list_filter_common,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::modal_create_post_url;

use crate::{
    crumbs::{leave_crumbs, leaves_list_crumbs},
    forms::{
        ApproveLeaveForm, LeaveApplicationFilterForm, LeaveApplicationFilterFormField,
        LeaveApplicationForm, LeaveApplicationFormField, RejectLeaveForm, RejectLeaveFormField,
        RevokeApprovalForm, RevokeRejectionForm,
    },
    keys::{
        LeaveApproveModalKey, LeaveCreateModalKey, LeaveDeleteModalKey, LeaveEditModalKey,
        LeaveRejectModalKey, LeaveRevokeApprovalModalKey, LeaveRevokeRejectionModalKey,
        LeaveTableKey,
    },
    logic::leave::STATUS_PENDING,
    routes::{
        LeaveApproveGetRouteTag, LeaveApprovePostRouteTag, LeaveCreateGetRouteTag,
        LeaveCreatePostRouteTag, LeaveDeleteGetRouteTag, LeaveDeletePostRouteTag,
        LeaveEditGetRouteTag, LeaveEditPostRouteTag, LeaveListRouteTag, LeaveRejectGetRouteTag,
        LeaveRejectPostRouteTag, LeaveRevokeApprovalGetRouteTag, LeaveRevokeApprovalPostRouteTag,
        LeaveRevokeRejectionGetRouteTag, LeaveRevokeRejectionPostRouteTag,
    },
    templates::{app_scaffold, hr_menu, scaffold_main, scaffold_pane},
};

#[derive(Clone)]
pub struct LeaveRow {
    pub id: i64,
    pub applied_by: String,
    pub date: String,
    pub leave_type: String,
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

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

fn leave_form_inputs(date: &str, leave_type: &str, reason: &str) -> Markup {
    let leave_types = choice_pairs(LeaveApplicationForm::leave_type_choices());
    LeaveApplicationForm::render_inputs(
        &FormCtx::form::<LeaveApplicationForm>(CsrfToken::current())
            .value(LeaveApplicationFormField::Date, date)
            .value(LeaveApplicationFormField::LeaveType, leave_type)
            .choices(LeaveApplicationFormField::LeaveType, &leave_types)
            .value(LeaveApplicationFormField::Reason, reason),
    )
}

fn approve_form_inputs() -> Markup {
    ApproveLeaveForm::render_inputs(&FormCtx::form::<ApproveLeaveForm>(CsrfToken::current()))
}

fn reject_form_inputs(reason: &str) -> Markup {
    RejectLeaveForm::render_inputs(
        &FormCtx::form::<RejectLeaveForm>(CsrfToken::current())
            .value(RejectLeaveFormField::Reason, reason),
    )
}

#[derive(Generic)]
pub struct LeaveListPage {
    pub rows: ObjectList<LeaveRow>,
    pub filter_applied_by_id: String,
    pub filter_applied_by_display: String,
    pub filter_date: String,
    pub filter_leave_type: String,
    pub filter_status: String,
    pub filter_reason: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl LeaveListPage {
    pub fn render_table(&self) -> Markup {
        let applied_by_sort = column_sort_url(&self.path_and_query, "AppliedBy", &self.sort);
        let date_sort = column_sort_url(&self.path_and_query, "Date", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "LeaveType", &self.sort);
        let applied_by_label = format!("Applied by{}", sort_indicator(&self.sort, "AppliedBy"));
        let date_label = format!("Date{}", sort_indicator(&self.sort, "Date"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "LeaveType"));
        let headers = [
            TableColumnHeader {
                key: "AppliedBy",
                label: &applied_by_label,
                sort_url: Some(&applied_by_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Date",
                label: &date_label,
                sort_url: Some(&date_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "LeaveType",
                label: &type_label,
                sort_url: Some(&type_sort),
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
                        value: &row.applied_by,
                        classes: "",
                    }),
                    field_date(FieldDate {
                        value: &row.date,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.leave_type,
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
        let leave_types = choice_pairs(LeaveApplicationFilterForm::leave_type_choices());
        let statuses = choice_pairs(LeaveApplicationFilterForm::status_choices());
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<LeaveTableKey, LeaveListRouteTag>(
                        LeaveListRouteTag,
                    ),
                    inputs: with_list_filter_common(
                        LeaveApplicationFilterForm::render_inputs(
                            &FormCtx::form::<LeaveApplicationFilterForm>(CsrfToken::current())
                                .value(
                                    LeaveApplicationFilterFormField::AppliedById,
                                    &self.filter_applied_by_id,
                                )
                                .display(
                                    LeaveApplicationFilterFormField::AppliedById,
                                    &self.filter_applied_by_display,
                                )
                                .value(
                                    LeaveApplicationFilterFormField::Date,
                                    &self.filter_date,
                                )
                                .value(
                                    LeaveApplicationFilterFormField::LeaveType,
                                    &self.filter_leave_type,
                                )
                                .choices(
                                    LeaveApplicationFilterFormField::LeaveType,
                                    &leave_types,
                                )
                                .value(
                                    LeaveApplicationFilterFormField::Status,
                                    &self.filter_status,
                                )
                                .choices(LeaveApplicationFilterFormField::Status, &statuses)
                                .value(
                                    LeaveApplicationFilterFormField::Reason,
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
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<super::super::routes::LeaveMutate>(
            ),
        ) {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.LeaveCreateForm",
                    href: &LeaveCreateGetRouteTag.url(),
                    form_post_url: &LeaveCreateGetRouteTag.path(),
                    modal_uid: LeaveCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        data_table_list_refresh::<LeaveTableKey>(
            "Leaves",
            actions,
            &headers,
            &table_rows,
            render_pagination::<LeaveTableKey>(
                &self.path_and_query,
                self.rows.number,
                self.rows.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for LeaveListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Leaves — Lariv",
            chrome,
            hr_menu("leaves"),
            leaves_list_crumbs(),
            self.render_table(),
        )
    }
}

impl RenderAppPane for LeaveListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(hr_menu("leaves"), leaves_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(leaves_list_crumbs(), self.render_table())
    }
}

#[derive(Generic)]
pub struct LeaveDetailPage {
    pub id: i64,
    pub title: String,
    pub applied_by: String,
    pub date: String,
    pub leave_type: String,
    pub reason: String,
    pub status: String,
    pub approved_by: String,
    pub approved_at: String,
    pub rejected_by: String,
    pub rejected_at: String,
    pub rejection_reason: String,
    pub can_approve: bool,
    pub can_edit: bool,
    pub can_revoke_approval: bool,
    pub can_revoke_rejection: bool,
}

impl LeaveDetailPage {
    fn body(&self) -> Markup {
        let can_mutate = lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<super::super::routes::LeaveMutate>(
            ),
        );
        let pending = self.status == STATUS_PENDING;
        let actions = html! {
            @if pending && self.can_approve {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.LeaveApproveForm",
                    href: &LeaveApproveGetRouteTag::new(self.id).url(),
                    form_post_url: &LeaveApprovePostRouteTag::new(self.id).path(),
                    modal_uid: LeaveApproveModalKey::ID,
                    label: "Approve",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
            }
            @if can_mutate {
                @if pending {
                    (button_modal_form(ButtonModalForm {
                        name: "p_hr.LeaveRejectForm",
                        href: &LeaveRejectGetRouteTag::new(self.id).url(),
                        form_post_url: &LeaveRejectPostRouteTag::new(self.id).path(),
                        modal_uid: LeaveRejectModalKey::ID,
                        label: "Reject",
                        classes: "btn-outline btn-error btn-sm",
                        ..Default::default()
                    }))
                }
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.LeaveDeleteForm",
                    href: &LeaveDeleteGetRouteTag::new(self.id).url(),
                    form_post_url: &LeaveDeleteGetRouteTag::new(self.id).path(),
                    modal_uid: LeaveDeleteModalKey::ID,
                    label: "Delete",
                    classes: "btn-outline btn-error btn-sm",
                    ..Default::default()
                }))
            }
            @if self.can_revoke_approval {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.LeaveRevokeApprovalForm",
                    href: &LeaveRevokeApprovalGetRouteTag::new(self.id).url(),
                    form_post_url: &LeaveRevokeApprovalPostRouteTag::new(self.id).path(),
                    modal_uid: LeaveRevokeApprovalModalKey::ID,
                    label: "Revoke approval",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
            }
            @if self.can_revoke_rejection {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.LeaveRevokeRejectionForm",
                    href: &LeaveRevokeRejectionGetRouteTag::new(self.id).url(),
                    form_post_url: &LeaveRevokeRejectionPostRouteTag::new(self.id).path(),
                    modal_uid: LeaveRevokeRejectionModalKey::ID,
                    label: "Revoke rejection",
                    classes: "btn-outline btn-error btn-sm",
                    ..Default::default()
                }))
            }
            @if self.can_edit {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.LeaveEditForm",
                    href: &LeaveEditGetRouteTag::new(self.id).url(),
                    form_post_url: &LeaveEditPostRouteTag::new(self.id).path(),
                    modal_uid: LeaveEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.title,
                        actions,
                    }))
                    (label("Applied by", field_text(FieldText { value: &self.applied_by, classes: "" })))
                    (label("Date", field_date(FieldDate { value: &self.date, classes: "" })))
                    (label("Type", field_text(FieldText { value: &self.leave_type, classes: "" })))
                    (label("Reason", field_textarea(FieldTextarea { value: &self.reason, classes: "" })))
                    (label("Status", field_text(FieldText { value: &self.status, classes: "" })))
                    @if !self.approved_by.is_empty() {
                        (label("Approved by", field_text(FieldText { value: &self.approved_by, classes: "" })))
                        (label("Approved at", field_datetime(FieldDatetime { value: &self.approved_at, classes: "" })))
                    }
                    @if !self.rejected_by.is_empty() {
                        (label("Rejected by", field_text(FieldText { value: &self.rejected_by, classes: "" })))
                        (label("Rejected at", field_datetime(FieldDatetime { value: &self.rejected_at, classes: "" })))
                        @if !self.rejection_reason.is_empty() {
                            (label("Rejection reason", field_textarea(FieldTextarea { value: &self.rejection_reason, classes: "" })))
                        }
                    }
                }))
            }))
        }
    }
}

impl RenderTemplate for LeaveDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Leave — Lariv",
            chrome,
            hr_menu("leaves"),
            leave_crumbs(&self.title),
            self.body(),
        )
    }
}

impl RenderAppPane for LeaveDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(hr_menu("leaves"), leave_crumbs(&self.title), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(leave_crumbs(&self.title), self.body())
    }
}

pub struct LeaveCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub date: String,
    pub leave_type: String,
    pub reason: String,
    pub error: String,
}

impl LeaveCreateModalPage {
    pub fn new(form_name: String, refresh_table: String) -> Self {
        Self {
            form_name,
            refresh_table,
            date: String::new(),
            leave_type: String::new(),
            reason: String::new(),
            error: String::new(),
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        form: &LeaveApplicationForm,
        error: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            date: form.date.clone(),
            leave_type: form.leave_type.clone(),
            reason: form.reason.clone(),
            error,
        }
    }
}

impl RenderTemplate for LeaveCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<LeaveCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New leave" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<LeaveCreateModalKey>(&modal_create_post_url(
                        LeaveCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: leave_form_inputs(&self.date, &self.leave_type, &self.reason),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct LeaveEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub date: String,
    pub leave_type: String,
    pub reason: String,
    pub error: String,
}

impl RenderTemplate for LeaveEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<LeaveEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit leave" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<LeaveEditModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: leave_form_inputs(&self.date, &self.leave_type, &self.reason),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct LeaveDeleteModalPage {
    pub id: i64,
    pub form_name: String,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for LeaveDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", LeaveDeleteModalKey::ID);
        let post_url = LeaveDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid: LeaveDeleteModalKey::ID,
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

pub struct LeaveApproveModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for LeaveApproveModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<LeaveApproveModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg" { "Approve leave" }
                p class="text-sm opacity-80 mb-4" { "This records the approval in your name at the current time." }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<LeaveApproveModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: approve_form_inputs(),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Approve", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct LeaveRejectModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub reason: String,
    pub error: String,
}

impl RenderTemplate for LeaveRejectModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<LeaveRejectModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg" { "Reject leave" }
                p class="text-sm opacity-80 mb-4" { "This records the rejection in your name at the current time. You can add a reason." }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<LeaveRejectModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: reject_form_inputs(&self.reason),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Reject", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct LeaveRevokeApprovalModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for LeaveRevokeApprovalModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<LeaveRevokeApprovalModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg" { "Revoke approval" }
                p class="text-sm opacity-80 mb-4" { "This removes the approval and returns the leave to pending." }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<LeaveRevokeApprovalModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: RevokeApprovalForm::render_inputs(
                        &FormCtx::form::<RevokeApprovalForm>(CsrfToken::current()),
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

pub struct LeaveRevokeRejectionModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for LeaveRevokeRejectionModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<LeaveRevokeRejectionModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg" { "Revoke rejection" }
                p class="text-sm opacity-80 mb-4" { "This removes the rejection and returns the leave to pending." }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<LeaveRevokeRejectionModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: RevokeRejectionForm::render_inputs(
                        &FormCtx::form::<RevokeRejectionForm>(CsrfToken::current()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_table_shows_leave_fields() {
        let page = LeaveListPage {
            rows: ObjectList::from_page(
                vec![LeaveRow {
                    id: 1,
                    applied_by: "Ada".into(),
                    date: "04/10/2026".into(),
                    leave_type: "Privilege Leave".into(),
                    status: "Pending".into(),
                    reason: "Family function".into(),
                    detail_href: "/dashboard/hr/leaves/1".into(),
                }],
                1,
                20,
                1,
            ),
            filter_applied_by_id: String::new(),
            filter_applied_by_display: String::new(),
            filter_date: String::new(),
            filter_leave_type: String::new(),
            filter_status: String::new(),
            filter_reason: String::new(),
            sort: String::new(),
            path_and_query: "/dashboard/hr/leaves".into(),
            page_size: 20,
        };
        let html = page.render_table().into_string();
        assert!(html.contains("Leaves"));
        assert!(html.contains("Ada"));
        assert!(html.contains("Privilege Leave"));
        assert!(html.contains("Pending"));
        assert!(html.contains("Family function"));
        assert!(html.contains("Casual"));
        assert!(html.contains("Sick"));
    }

    #[test]
    fn detail_shows_the_decision() {
        let page = LeaveDetailPage {
            id: 1,
            title: "Ada".into(),
            applied_by: "Ada".into(),
            date: "04/10/2026".into(),
            leave_type: "Privilege Leave".into(),
            reason: "Family function".into(),
            status: "Approved".into(),
            approved_by: "Grace".into(),
            approved_at: "04/10/2026 09:00".into(),
            rejected_by: String::new(),
            rejected_at: String::new(),
            rejection_reason: String::new(),
            can_approve: false,
            can_edit: false,
            can_revoke_approval: false,
            can_revoke_rejection: false,
        };
        let html = page
            .render(&lariv_core::components::ShellChrome::default())
            .into_string();
        assert!(html.contains("Privilege Leave"));
        assert!(html.contains("Family function"));
        assert!(html.contains("Approved"));
        assert!(html.contains("Grace"));
        assert!(html.contains("Approved at"));
        assert!(!html.contains("Rejection reason"));
    }

    #[test]
    fn approve_button_follows_the_approver_flag() {
        let mut page = LeaveDetailPage {
            id: 2,
            title: "Ada".into(),
            applied_by: "Ada".into(),
            date: "04/10/2026".into(),
            leave_type: "Casual Leave".into(),
            reason: "Errand".into(),
            status: STATUS_PENDING.to_string(),
            approved_by: String::new(),
            approved_at: String::new(),
            rejected_by: String::new(),
            rejected_at: String::new(),
            rejection_reason: String::new(),
            can_approve: false,
            can_edit: false,
            can_revoke_approval: false,
            can_revoke_rejection: false,
        };
        let hidden = page
            .render(&lariv_core::components::ShellChrome::default())
            .into_string();
        assert!(!hidden.contains("Approve"));

        page.can_approve = true;
        let shown = page
            .render(&lariv_core::components::ShellChrome::default())
            .into_string();
        assert!(shown.contains("Approve"));
    }

    #[test]
    fn edit_button_follows_the_editor_flag() {
        let mut page = LeaveDetailPage {
            id: 2,
            title: "Ada".into(),
            applied_by: "Ada".into(),
            date: "04/10/2026".into(),
            leave_type: "Casual Leave".into(),
            reason: "Errand".into(),
            status: STATUS_PENDING.to_string(),
            approved_by: String::new(),
            approved_at: String::new(),
            rejected_by: String::new(),
            rejected_at: String::new(),
            rejection_reason: String::new(),
            can_approve: false,
            can_edit: false,
            can_revoke_approval: false,
            can_revoke_rejection: false,
        };
        let hidden = page
            .render(&lariv_core::components::ShellChrome::default())
            .into_string();
        assert!(!hidden.contains("p_hr.LeaveEditForm"));

        page.can_edit = true;
        let shown = page
            .render(&lariv_core::components::ShellChrome::default())
            .into_string();
        assert!(shown.contains("p_hr.LeaveEditForm"));
    }

    #[test]
    fn revoke_buttons_follow_the_decision_flags() {
        let mut page = LeaveDetailPage {
            id: 3,
            title: "Ada".into(),
            applied_by: "Ada".into(),
            date: "04/10/2026".into(),
            leave_type: "Casual Leave".into(),
            reason: "Errand".into(),
            status: "Approved".into(),
            approved_by: "Grace".into(),
            approved_at: "04/10/2026 09:00".into(),
            rejected_by: String::new(),
            rejected_at: String::new(),
            rejection_reason: String::new(),
            can_approve: true,
            can_edit: false,
            can_revoke_approval: false,
            can_revoke_rejection: false,
        };
        let hidden = page
            .render(&lariv_core::components::ShellChrome::default())
            .into_string();
        assert!(!hidden.contains("p_hr.LeaveRevokeApprovalForm"));
        assert!(!hidden.contains("p_hr.LeaveRevokeRejectionForm"));

        page.can_revoke_approval = true;
        let approved = page
            .render(&lariv_core::components::ShellChrome::default())
            .into_string();
        assert!(approved.contains("p_hr.LeaveRevokeApprovalForm"));
        assert!(approved.contains("Revoke approval"));
        assert!(!approved.contains("p_hr.LeaveRevokeRejectionForm"));

        page.can_revoke_approval = false;
        page.can_revoke_rejection = true;
        page.status = "Rejected".into();
        let rejected = page
            .render(&lariv_core::components::ShellChrome::default())
            .into_string();
        assert!(rejected.contains("p_hr.LeaveRevokeRejectionForm"));
        assert!(rejected.contains("Revoke rejection"));
        assert!(!rejected.contains("p_hr.LeaveRevokeApprovalForm"));
    }

    #[test]
    fn decision_forms_omit_actor_and_time() {
        let chrome = lariv_core::components::ShellChrome::default();
        let approve = LeaveApproveModalPage {
            id: 1,
            form_name: "approve".into(),
            post_url: "/dashboard/hr/leaves/1/approve".into(),
            error: String::new(),
        }
        .render(&chrome)
        .into_string();
        assert!(approve.contains("Approve leave"));
        assert!(approve.contains("This records the approval in your name at the current time."));
        assert!(!approve.contains("Approved by"));
        assert!(!approve.contains("Approved at"));

        let reject = LeaveRejectModalPage {
            id: 1,
            form_name: "reject".into(),
            post_url: "/dashboard/hr/leaves/1/reject".into(),
            reason: "Dates overlap".into(),
            error: String::new(),
        }
        .render(&chrome)
        .into_string();
        assert!(reject.contains("Reject leave"));
        assert!(reject.contains(
            "This records the rejection in your name at the current time. You can add a reason."
        ));
        assert!(reject.contains("Dates overlap"));
        assert!(reject.contains("Reason"));
        assert!(!reject.contains("Rejected by"));
        assert!(!reject.contains("Rejected at"));
    }

    #[test]
    fn application_forms_omit_applied_by() {
        let chrome = lariv_core::components::ShellChrome::default();
        let create = LeaveCreateModalPage::new("create".into(), "#hr-leaves".into())
            .render(&chrome)
            .into_string();
        assert!(create.contains("New leave"));
        assert!(create.contains("Date"));
        assert!(!create.contains("Applied by"));
        assert!(!create.contains("Select user"));

        let edit = LeaveEditModalPage {
            id: 1,
            form_name: "edit".into(),
            post_url: "/dashboard/hr/leaves/1/edit".into(),
            date: "04/10/2026".into(),
            leave_type: "casual".into(),
            reason: "Errand".into(),
            error: String::new(),
        }
        .render(&chrome)
        .into_string();
        assert!(edit.contains("Edit leave"));
        assert!(edit.contains("Errand"));
        assert!(!edit.contains("Applied by"));
        assert!(!edit.contains("Select user"));
    }
}
