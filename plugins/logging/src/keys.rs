//! Compile-time HTMX swap key for the audit log list.

use lariv_core::components::SwapKey;

pub const LOG_TABLE: SwapKey = SwapKey("logs-table");
