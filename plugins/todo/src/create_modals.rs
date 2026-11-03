//! Typed create-modal wiring for a to-do page.

use lariv_core::web::CreateModal;

use super::keys::TODO_PAGE_CREATE_MODAL;
use super::routes::{TodoPageCreateGetRouteTag, TodoPageCreatePostRouteTag};

pub const TODO_PAGE_CREATE: CreateModal<TodoPageCreateGetRouteTag, TodoPageCreatePostRouteTag> =
    CreateModal {
        key: TODO_PAGE_CREATE_MODAL,
        form_name: "p_todo.TodoPageCreateForm",
        get: TodoPageCreateGetRouteTag,
        post: TodoPageCreatePostRouteTag,
    };
