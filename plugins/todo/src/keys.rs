//! Compile-time HTMX swap keys for the to-do plugin.

use lariv_core::components::SwapKey;

pub const TODO_PAGE_TABLE: SwapKey = SwapKey("todo-pages-table");
pub const TODO_PAGE_CREATE_MODAL: SwapKey = SwapKey("todo-page-create-modal");
pub const TODO_PAGE_EDIT_MODAL: SwapKey = SwapKey("todo-page-edit-modal");
pub const TODO_PAGE_DELETE_MODAL: SwapKey = SwapKey("todo-page-delete-modal");
