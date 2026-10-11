//! Typed [`CreateModal`] wiring for tasks swap keys.

use super::keys::{PeriodicTaskCreateModalKey, TaskCreateModalKey, TaskStatusCreateModalKey};
use super::routes::{
    PeriodicTaskCreateGetRouteTag, PeriodicTaskCreatePostRouteTag, TaskCreateGetRouteTag,
    TaskCreatePostRouteTag, TaskStatusCreateGetRouteTag, TaskStatusCreatePostRouteTag,
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
lariv_core::impl_create_modal!(
    PeriodicTaskCreateModalKey,
    PeriodicTaskCreateGetRouteTag,
    PeriodicTaskCreatePostRouteTag,
    "p_tasks.PeriodicTaskCreateForm"
);
