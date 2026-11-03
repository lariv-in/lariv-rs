//! Typed [`CreateModal`] wiring for tasks swap keys.

use lariv_core::web::CreateModal;

use super::keys::{PERIODIC_TASK_CREATE_MODAL, TASK_CREATE_MODAL, TASK_STATUS_CREATE_MODAL};
use super::routes::{
    PeriodicTaskCreateGetRouteTag, PeriodicTaskCreatePostRouteTag, TaskCreateGetRouteTag,
    TaskCreatePostRouteTag, TaskStatusCreateGetRouteTag, TaskStatusCreatePostRouteTag,
};

pub const TASK_CREATE: CreateModal<TaskCreateGetRouteTag, TaskCreatePostRouteTag> = CreateModal {
    key: TASK_CREATE_MODAL,
    form_name: "p_tasks.TaskCreateForm",
    get: TaskCreateGetRouteTag,
    post: TaskCreatePostRouteTag,
};

pub const TASK_STATUS_CREATE: CreateModal<
    TaskStatusCreateGetRouteTag,
    TaskStatusCreatePostRouteTag,
> = CreateModal {
    key: TASK_STATUS_CREATE_MODAL,
    form_name: "p_tasks.TaskStatusCreateForm",
    get: TaskStatusCreateGetRouteTag,
    post: TaskStatusCreatePostRouteTag,
};

pub const PERIODIC_TASK_CREATE: CreateModal<
    PeriodicTaskCreateGetRouteTag,
    PeriodicTaskCreatePostRouteTag,
> = CreateModal {
    key: PERIODIC_TASK_CREATE_MODAL,
    form_name: "p_tasks.PeriodicTaskCreateForm",
    get: PeriodicTaskCreateGetRouteTag,
    post: PeriodicTaskCreatePostRouteTag,
};
