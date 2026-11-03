//! Compile-time HTMX swap keys for the filesystem plugin.

use lariv_core::components::SwapKey;

pub const V_NODE_TABLE: SwapKey = SwapKey("vnode-table");
pub const V_NODE_SELECT_TABLE: SwapKey = SwapKey("vnode-selection-table");
pub const V_NODE_SELECT_MODAL: SwapKey = SwapKey("vnode-selection-modal");
pub const V_NODE_CREATE_MODAL: SwapKey = SwapKey("vnode-create-modal");
pub const V_NODE_EDIT_MODAL: SwapKey = SwapKey("vnode-edit-modal");
pub const V_NODE_MULTI_UPLOAD_MODAL: SwapKey = SwapKey("vnode-multi-upload-modal");
pub const V_NODE_ZIP_UPLOAD_MODAL: SwapKey = SwapKey("vnode-zip-upload-modal");
pub const V_NODE_DELETE_MODAL: SwapKey = SwapKey("vnode-delete-modal");
pub const V_NODE_BULK_DELETE_MODAL: SwapKey = SwapKey("vnode-bulk-delete-modal");
pub const V_NODE_FK_PARENT: SwapKey = SwapKey("fk-vnode-parent");
pub const V_NODE_FK_DESTINATION: SwapKey = SwapKey("fk-vnode-destination");
pub const V_NODE_PDF_MODAL: SwapKey = SwapKey("vnode-pdf-modal");
