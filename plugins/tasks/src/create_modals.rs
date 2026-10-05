//! Typed [`CreateModal`] wiring for tasks swap keys.

use super::keys::{TaskCreateModalKey, TaskStatusCreateModalKey};
use super::routes::{
    TaskCreateGetRouteTag, TaskCreatePostRouteTag, TaskStatusCreateGetRouteTag,
    TaskStatusCreatePostRouteTag,
};

lariv_core::impl_create_modal!(
    TaskCreateModalKey,
    TaskCreateGetRouteTag,
    TaskCreatePostRouteTag,
    "p_tasks.TaskCreateForm"
);
lariv_core::impl_create_modal!(
    TaskStatusCreateModalKey,
    TaskStatusCreateGetRouteTag,
    TaskStatusCreatePostRouteTag,
    "p_tasks.TaskStatusCreateForm"
);
