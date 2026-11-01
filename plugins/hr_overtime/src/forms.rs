use lariv_core::html_form::{
    html_form,
    widgets::{Datetime, Textarea},
};

use lariv_plugin_users::routes::UsersSelectRouteTag;

#[html_form]
pub struct OvertimeApplicationForm {
    /// Superuser picks who the overtime is for. Hidden for every other role.
    #[form(
        label = "User",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-overtime-user",
        display = "user",
        placeholder = "Select user…",
        when = "pick_user"
    )]
    pub user_id: i64,

    #[form(label = "Start", required, widget = Datetime)]
    pub start_time: String,

    #[form(label = "End", required, widget = Datetime)]
    pub end_time: String,

    #[form(label = "Reason", widget = Textarea, rows = 4)]
    pub reason: String,
}

/// Superuser records approved overtime directly. Every column is entered here.
#[html_form]
pub struct ApprovedOvertimeForm {
    #[form(
        label = "User",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-approved-overtime-user",
        display = "user",
        placeholder = "Select user…",
    )]
    pub user_id: i64,

    #[form(label = "Start", required, widget = Datetime)]
    pub start_time: String,

    #[form(label = "End", required, widget = Datetime)]
    pub end_time: String,

    #[form(
        label = "Approved by",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-approved-overtime-approved-by",
        display = "approved_by",
        placeholder = "Select user…",
    )]
    pub approved_by_id: i64,

    #[form(label = "Approved at", required, widget = Datetime)]
    pub approved_at: String,
}

/// Confirm-only. The approver is the signed-in user and the time is now.
#[html_form]
pub struct ApproveOvertimeForm {}

/// Confirm-only. The rejector is the signed-in user and the time is now.
#[html_form]
pub struct RejectOvertimeForm {}

/// Confirm-only. Removes the approval and returns the application to pending.
#[html_form]
pub struct RevokeOvertimeApprovalForm {}

/// Confirm-only. Removes the rejection and returns the application to pending.
#[html_form]
pub struct RevokeOvertimeRejectionForm {}
