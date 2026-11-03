//! Compile-time HTMX swap keys for the users plugin.

use lariv_core::components::SwapKey;

pub const USER_TABLE: SwapKey = SwapKey("user-table");
pub const USER_SELECT_TABLE: SwapKey = SwapKey("user-selection-table");
pub const USER_CREATE_MODAL: SwapKey = SwapKey("user-create-modal");
pub const USER_EDIT_MODAL: SwapKey = SwapKey("user-edit-modal");
pub const USER_DELETE_MODAL: SwapKey = SwapKey("user-delete-modal");
pub const USER_SELECT_MODAL: SwapKey = SwapKey("user-selection-modal");
pub const SELF_EDIT_MODAL: SwapKey = SwapKey("user-self-edit-modal");
