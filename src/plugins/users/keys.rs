//! Compile-time HTMX swap keys for the users plugin.

use crate::swap_key;

swap_key!(UserTableKey, "user-table");
swap_key!(RoleTableKey, "role-table");
swap_key!(UserSelectTableKey, "user-selection-table");
swap_key!(RoleSelectTableKey, "role-selection-table");
swap_key!(UserCreateModalKey, "user-create-modal");
swap_key!(UserEditModalKey, "user-edit-modal");
swap_key!(UserDeleteModalKey, "user-delete-modal");
swap_key!(RoleCreateModalKey, "role-create-modal");
swap_key!(RoleEditModalKey, "role-edit-modal");
swap_key!(RoleDeleteModalKey, "role-delete-modal");
swap_key!(UserSelectModalKey, "user-selection-modal");
swap_key!(RoleSelectModalKey, "role-selection-modal");
swap_key!(UserFkRoleKey, "fk-user-role");
swap_key!(SelfEditModalKey, "user-self-edit-modal");
