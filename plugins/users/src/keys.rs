//! Compile-time HTMX swap keys for the users plugin.

use lariv_core::swap_key;

swap_key!(UserTableKey, "user-table");
swap_key!(UserSelectTableKey, "user-selection-table");
swap_key!(UserCreateModalKey, "user-create-modal");
swap_key!(UserEditModalKey, "user-edit-modal");
swap_key!(UserDeleteModalKey, "user-delete-modal");
swap_key!(UserSelectModalKey, "user-selection-modal");
swap_key!(SelfEditModalKey, "user-self-edit-modal");
