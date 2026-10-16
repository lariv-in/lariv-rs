//! Compile-time HTMX swap keys for the to-do plugin.

use lariv_core::swap_key;

swap_key!(TodoPageTableKey, "todo-pages-table");
swap_key!(TodoPageCreateModalKey, "todo-page-create-modal");
swap_key!(TodoPageEditModalKey, "todo-page-edit-modal");
swap_key!(TodoPageDeleteModalKey, "todo-page-delete-modal");
