use lariv_core::html_form::{html_form, widgets::Datetime};
use lariv_plugin_users::routes::UsersSelectRouteTag;

#[html_form]
pub struct AttendanceForm {
    #[form(
        label = "User",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-attendance-user",
        display = "user",
        placeholder = "Select user…"
    )]
    pub user_id: i64,

    #[form(label = "Start", required, widget = Datetime)]
    pub started_at: String,

    #[form(label = "End", widget = Datetime)]
    pub ended_at: String,
}
