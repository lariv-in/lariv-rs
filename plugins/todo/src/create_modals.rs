//! Typed [`CreateModal`] wiring for the todo create swap key.

use super::keys::TodoCreateModalKey;
use super::routes::{TodoCreateGetRouteTag, TodoCreatePostRouteTag};

lariv_core::impl_create_modal!(
    TodoCreateModalKey,
    TodoCreateGetRouteTag,
    TodoCreatePostRouteTag,
    "p_todo.TodoCreateForm"
);
