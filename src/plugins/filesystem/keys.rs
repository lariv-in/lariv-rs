//! Compile-time HTMX swap keys for the filesystem plugin.

use crate::swap_key;

swap_key!(VNodeTableKey, "vnode-table");
swap_key!(VNodeSelectTableKey, "vnode-selection-table");
swap_key!(VNodeSelectModalKey, "vnode-selection-modal");
swap_key!(VNodeDeleteModalKey, "vnode-delete-modal");
swap_key!(VNodeFkParentKey, "fk-vnode-parent");
swap_key!(VNodeFkDestinationKey, "fk-vnode-destination");
