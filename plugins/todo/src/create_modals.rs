//! Typed create-modal wiring for a to-do page.

use super::keys::TodoPageCreateModalKey;
use super::routes::{TodoPageCreateGetRouteTag, TodoPageCreatePostRouteTag};

lariv_core::impl_create_modal!(
    TodoPageCreateModalKey,
    TodoPageCreateGetRouteTag,
    TodoPageCreatePostRouteTag,
    "p_todo.TodoPageCreateForm"
);
