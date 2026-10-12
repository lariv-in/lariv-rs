use crate::routes::TaskSelectRouteTag;
use lariv_core::html_form::{
    html_form,
    widgets::{Color, Datetime, Duration, Number, Select, Text, Textarea},
};
use lariv_plugin_users::routes::UsersSelectRouteTag;

#[html_form]
pub struct TaskForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Description", widget = Textarea)]
    pub description: String,

    #[form(
        label = "Assigned To",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "tasks-assigned-to",
        display = "assigned_to",
        placeholder = "Select user…"
    )]
    pub assigned_to_id: i64,

    #[form(
        label = "Parent task",
        widget = ForeignKey,
        route = TaskSelectRouteTag,
        swap_key = "tasks-parent",
        display = "parent",
        placeholder = "Optional parent…"
    )]
    pub parent_id: String,

    #[form(label = "Priority", required, widget = Number)]
    pub priority: String,

    #[form(label = "Due date & time", required, widget = Datetime)]
    pub due_datetime: String,
}

#[html_form]
pub struct TaskStatusForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Color", required, widget = Color)]
    pub color: String,
}

#[html_form]
pub struct TaskLogForm {
    #[form(label = "Date & time", required, widget = Datetime)]
    pub datetime: String,

    #[form(label = "Description", required, widget = Textarea)]
    pub description: String,
}

#[html_form]
pub struct PeriodicTaskForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Description", widget = Textarea)]
    pub description: String,

    #[form(
        label = "Assigned To",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "tasks-periodic-assigned-to",
        display = "assigned_to",
        placeholder = "Select user…"
    )]
    pub assigned_to_id: i64,

    #[form(label = "Status", required, widget = Select, choices = "status")]
    pub status_id: String,

    #[form(
        label = "Parent task",
        widget = ForeignKey,
        route = TaskSelectRouteTag,
        swap_key = "tasks-periodic-parent",
        display = "parent",
        placeholder = "Optional parent…"
    )]
    pub parent_id: String,

    #[form(label = "Priority", required, widget = Number)]
    pub priority: String,

    #[form(label = "Due date & time", required, widget = Datetime)]
    pub due_datetime: String,

    #[form(label = "Frequency", required, widget = Duration)]
    pub frequency: String,

    #[form(label = "Last added at", required, widget = Datetime)]
    pub last_added_at: String,
}

#[html_form]
pub struct TaskLogQuickForm {
    #[form(label = "Date & time", required, widget = Datetime, model = "datetime")]
    pub datetime: String,

    #[form(label = "Description", required, widget = Textarea, model = "description")]
    pub description: String,
}
