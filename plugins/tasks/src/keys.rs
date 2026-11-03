use lariv_core::components::SwapKey;

pub const TASK_TABLE: SwapKey = SwapKey("tasks-table");
pub const TASK_CHILDREN_TABLE: SwapKey = SwapKey("tasks-children-table");
pub const TASK_SELECT_TABLE: SwapKey = SwapKey("tasks-select-table");
pub const TASK_SELECT_MODAL: SwapKey = SwapKey("tasks-select-modal");
pub const TASK_CREATE_MODAL: SwapKey = SwapKey("tasks-create-modal");
pub const TASK_EDIT_MODAL: SwapKey = SwapKey("tasks-edit-modal");
pub const TASK_DELETE_MODAL: SwapKey = SwapKey("tasks-delete-modal");
pub const TASK_LOGS: SwapKey = SwapKey("tasks-logs");
/// HTMX `HX-Trigger` / Alpine event after a log is saved from the logs form.
pub const TASK_LOG_SAVED_EVENT: &str = "tasks-log-saved";
pub const TASK_LOG_EDIT_MODAL: SwapKey = SwapKey("tasks-log-edit-modal");
pub const TASK_LOG_DELETE_MODAL: SwapKey = SwapKey("tasks-log-delete-modal");
pub const TASK_STATUS_TABLE: SwapKey = SwapKey("tasks-status-table");
pub const TASK_STATUS_CREATE_MODAL: SwapKey = SwapKey("tasks-status-create-modal");
pub const TASK_STATUS_EDIT_MODAL: SwapKey = SwapKey("tasks-status-edit-modal");
pub const TASK_STATUS_DELETE_MODAL: SwapKey = SwapKey("tasks-status-delete-modal");
pub const TASK_STATUS_TASKS_TABLE: SwapKey = SwapKey("tasks-status-tasks-table");
pub const PERIODIC_TASK_TABLE: SwapKey = SwapKey("tasks-periodic-table");
pub const PERIODIC_TASK_CREATE_MODAL: SwapKey = SwapKey("tasks-periodic-create-modal");
pub const PERIODIC_TASK_EDIT_MODAL: SwapKey = SwapKey("tasks-periodic-edit-modal");
pub const PERIODIC_TASK_DELETE_MODAL: SwapKey = SwapKey("tasks-periodic-delete-modal");
