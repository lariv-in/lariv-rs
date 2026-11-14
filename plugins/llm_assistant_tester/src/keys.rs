//! Compile-time HTMX swap keys for the assistant tester plugin.

use lariv_core::components::SwapKey;

pub const TEST_CASE_TABLE: SwapKey = SwapKey("test-cases-table");
pub const TEST_CASE_CREATE_MODAL: SwapKey = SwapKey("test-case-create-modal");
pub const TEST_CASE_EDIT_MODAL: SwapKey = SwapKey("test-case-edit-modal");
pub const TEST_CASE_DELETE_MODAL: SwapKey = SwapKey("test-case-delete-modal");
