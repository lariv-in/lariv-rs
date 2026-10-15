//! Compile-time HTMX swap keys for the todo plugin.

use lariv_core::swap_key;

swap_key!(TodoTableKey, "todos-table");
swap_key!(TodoCreateModalKey, "todo-create-modal");
swap_key!(TodoEditModalKey, "todo-edit-modal");
swap_key!(TodoDeleteModalKey, "todo-delete-modal");
