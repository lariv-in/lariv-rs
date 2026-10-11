lariv_core::swap_key!(TaskTableKey, "tasks-table");
lariv_core::swap_key!(TaskChildrenTableKey, "tasks-children-table");
lariv_core::swap_key!(TaskSelectTableKey, "tasks-select-table");
lariv_core::swap_key!(TaskSelectModalKey, "tasks-select-modal");
lariv_core::swap_key!(TaskCreateModalKey, "tasks-create-modal");
lariv_core::swap_key!(TaskEditModalKey, "tasks-edit-modal");
lariv_core::swap_key!(TaskDeleteModalKey, "tasks-delete-modal");
lariv_core::swap_key!(TaskLogsKey, "tasks-logs");
/// HTMX `HX-Trigger` / Alpine event after a log is saved from the logs form.
pub const TASK_LOG_SAVED_EVENT: &str = "tasks-log-saved";
lariv_core::swap_key!(TaskLogEditModalKey, "tasks-log-edit-modal");
lariv_core::swap_key!(TaskLogDeleteModalKey, "tasks-log-delete-modal");
lariv_core::swap_key!(TaskStatusTableKey, "tasks-status-table");
lariv_core::swap_key!(TaskStatusCreateModalKey, "tasks-status-create-modal");
lariv_core::swap_key!(TaskStatusEditModalKey, "tasks-status-edit-modal");
lariv_core::swap_key!(TaskStatusDeleteModalKey, "tasks-status-delete-modal");
lariv_core::swap_key!(TaskStatusTasksTableKey, "tasks-status-tasks-table");
