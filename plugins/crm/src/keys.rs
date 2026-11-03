use lariv_core::components::SwapKey;

pub const LEAD_HUB_TABLE: SwapKey = SwapKey("crm-lead-hub-table");
pub const LEAD_CREATE_MODAL: SwapKey = SwapKey("crm-lead-create-modal");
pub const LEAD_EDIT_MODAL: SwapKey = SwapKey("crm-lead-edit-modal");
pub const LEAD_DELETE_MODAL: SwapKey = SwapKey("crm-lead-delete-modal");
pub const LEAD_CONVERT_MODAL: SwapKey = SwapKey("crm-lead-convert-modal");
pub const LEAD_FAIL_MODAL: SwapKey = SwapKey("crm-lead-fail-modal");
pub const LEAD_UPDATES: SwapKey = SwapKey("crm-lead-updates");
pub const LEAD_UPDATE_EDIT_MODAL: SwapKey = SwapKey("crm-lead-update-edit-modal");
pub const LEAD_UPDATE_DELETE_MODAL: SwapKey = SwapKey("crm-lead-update-delete-modal");
pub const LEAD_TAG_TABLE: SwapKey = SwapKey("crm-lead-tag-table");
pub const LEAD_TAG_LEADS_TABLE: SwapKey = SwapKey("crm-lead-tag-leads-table");
pub const LEAD_TAG_SELECT_TABLE: SwapKey = SwapKey("crm-lead-tag-select-table");
pub const LEAD_TAG_SELECT_MODAL: SwapKey = SwapKey("crm-lead-tag-select-modal");
pub const LEAD_TAG_CREATE_MODAL: SwapKey = SwapKey("crm-lead-tag-create-modal");
pub const LEAD_TAG_EDIT_MODAL: SwapKey = SwapKey("crm-lead-tag-edit-modal");
pub const LEAD_TAG_DELETE_MODAL: SwapKey = SwapKey("crm-lead-tag-delete-modal");
pub const CUSTOMER_SELECT_TABLE: SwapKey = SwapKey("crm-customer-select-table");
pub const CUSTOMER_SELECT_MODAL: SwapKey = SwapKey("crm-customer-select-modal");
pub const CUSTOMER_CREATE_MODAL: SwapKey = SwapKey("crm-customer-create-modal");

/// HTMX `HX-Trigger` / Alpine event after a lead update is saved from the detail form.
pub const LEAD_UPDATE_SAVED_EVENT: &str = "crm-lead-update-saved";
