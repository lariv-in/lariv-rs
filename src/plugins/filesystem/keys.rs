//! Compile-time HTMX swap keys for the filesystem plugin.

use crate::swap_key;

swap_key!(VNodeTableKey, "vnode-table");
swap_key!(VNodeSelectTableKey, "vnode-selection-table");
swap_key!(VNodeSelectModalKey, "vnode-selection-modal");
swap_key!(VNodeCreateModalKey, "vnode-create-modal");
swap_key!(VNodeEditModalKey, "vnode-edit-modal");
swap_key!(VNodeMultiUploadModalKey, "vnode-multi-upload-modal");
swap_key!(VNodeZipUploadModalKey, "vnode-zip-upload-modal");
swap_key!(VNodeDeleteModalKey, "vnode-delete-modal");
swap_key!(VNodeBulkDeleteModalKey, "vnode-bulk-delete-modal");
swap_key!(VNodeFkParentKey, "fk-vnode-parent");
swap_key!(VNodeFkDestinationKey, "fk-vnode-destination");
