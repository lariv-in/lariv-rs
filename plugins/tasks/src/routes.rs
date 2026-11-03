use lariv_plugin_users::roles::{Admin, Unassigned};

use super::{
    handlers,
    keys::{
        PERIODIC_TASK_DELETE_MODAL, PERIODIC_TASK_TABLE, TASK_CHILDREN_TABLE, TASK_DELETE_MODAL,
        TASK_LOG_DELETE_MODAL, TASK_LOGS, TASK_SELECT_MODAL, TASK_SELECT_TABLE,
        TASK_STATUS_DELETE_MODAL, TASK_STATUS_TABLE, TASK_STATUS_TASKS_TABLE, TASK_TABLE,
    },
};

pub struct TasksView;

pub struct TasksMutate;

lariv_core::define_plugin_routes! {
    plugin: TasksTag;
    prefix: "/dashboard";
    routes: [
        get TaskDefaultRouteTag, "/tasks", handlers::tasks::hub, fragment(TASK_TABLE), authorize(TasksView, [Unassigned, Admin]);
        get TaskCreateGetRouteTag, "/tasks/create", handlers::tasks::create_get, modal, authorize(TasksMutate, []);
        post TaskCreatePostRouteTag, "/tasks/create", handlers::tasks::create_post, authorize(TasksMutate, []);
        get TaskSelectRouteTag, "/tasks/select", handlers::tasks::select, fk_select(TASK_SELECT_TABLE, TASK_SELECT_MODAL), authorize(TasksView, [Unassigned, Admin]);

        get TaskStatusDefaultRouteTag, "/tasks/statuses", handlers::statuses::list, fragment(TASK_STATUS_TABLE), authorize(TasksView, [Unassigned, Admin]);
        get TaskStatusCreateGetRouteTag, "/tasks/statuses/create", handlers::statuses::create_get, modal, authorize(TasksMutate, []);
        post TaskStatusCreatePostRouteTag, "/tasks/statuses/create", handlers::statuses::create_post, authorize(TasksMutate, []);
        get TaskStatusDetailRouteTag, "/tasks/statuses/{id}", handlers::statuses::detail, fragment(TASK_STATUS_TASKS_TABLE), authorize(TasksView, [Unassigned, Admin]);
        get TaskStatusEditGetRouteTag, "/tasks/statuses/{id}/edit", handlers::statuses::edit_get, modal, authorize(TasksMutate, []);
        post TaskStatusEditPostRouteTag, "/tasks/statuses/{id}/edit", handlers::statuses::edit_post, authorize(TasksMutate, []);
        get TaskStatusDeleteGetRouteTag, "/tasks/statuses/{id}/delete", handlers::statuses::delete_get, modal, authorize(TasksMutate, []);
        post TaskStatusDeletePostRouteTag, "/tasks/statuses/{id}/delete", bare handlers::statuses::delete_post, fragment(TASK_STATUS_DELETE_MODAL), authorize(TasksMutate, []);

        get PeriodicTaskDefaultRouteTag, "/tasks/periodic", handlers::periodic::list, fragment(PERIODIC_TASK_TABLE), authorize(TasksView, [Unassigned, Admin]);
        get PeriodicTaskCreateGetRouteTag, "/tasks/periodic/create", handlers::periodic::create_get, modal, authorize(TasksMutate, []);
        post PeriodicTaskCreatePostRouteTag, "/tasks/periodic/create", handlers::periodic::create_post, authorize(TasksMutate, []);
        get PeriodicTaskDetailRouteTag, "/tasks/periodic/{id}", handlers::periodic::detail, authorize(TasksView, [Unassigned, Admin]);
        get PeriodicTaskEditGetRouteTag, "/tasks/periodic/{id}/edit", handlers::periodic::edit_get, modal, authorize(TasksMutate, []);
        post PeriodicTaskEditPostRouteTag, "/tasks/periodic/{id}/edit", handlers::periodic::edit_post, authorize(TasksMutate, []);
        get PeriodicTaskDeleteGetRouteTag, "/tasks/periodic/{id}/delete", handlers::periodic::delete_get, modal, authorize(TasksMutate, []);
        post PeriodicTaskDeletePostRouteTag, "/tasks/periodic/{id}/delete", bare handlers::periodic::delete_post, fragment(PERIODIC_TASK_DELETE_MODAL), authorize(TasksMutate, []);

        get TaskLogsRouteTag, "/tasks/{id}/logs", handlers::logs::list, fragment(TASK_LOGS), authorize(TasksView, [Unassigned, Admin]);
        post TaskLogAddPostRouteTag, "/tasks/{id}/logs", handlers::logs::add_post, fragment(TASK_LOGS), authorize(TasksMutate, []);
        get TaskLogDetailRouteTag, "/tasks/logs/{id}", handlers::logs::detail, authorize(TasksView, [Unassigned, Admin]);
        get TaskLogEditGetRouteTag, "/tasks/logs/{id}/edit", handlers::logs::edit_get, modal, authorize(TasksMutate, []);
        post TaskLogEditPostRouteTag, "/tasks/logs/{id}/edit", handlers::logs::edit_post, authorize(TasksMutate, []);
        get TaskLogDeleteGetRouteTag, "/tasks/logs/{id}/delete", handlers::logs::delete_get, modal, authorize(TasksMutate, []);
        post TaskLogDeletePostRouteTag, "/tasks/logs/{id}/delete", bare handlers::logs::delete_post, fragment(TASK_LOG_DELETE_MODAL), authorize(TasksMutate, []);

        post TaskSetStatusRouteTag, "/tasks/{id}/status/{slug}", handlers::tasks::set_status, authorize(TasksView, [Unassigned, Admin]);
        post TaskSetPriorityListRouteTag, "/tasks/{id}/priority/{direction}/list", handlers::tasks::set_priority, fragment(TASK_TABLE), authorize(TasksMutate, []);
        post TaskSetPriorityChildrenRouteTag, "/tasks/{id}/priority/{direction}/children", handlers::tasks::set_priority, fragment(TASK_CHILDREN_TABLE), authorize(TasksMutate, []);

        get TaskDetailRouteTag, "/tasks/{id}", handlers::tasks::detail, fragment(TASK_CHILDREN_TABLE), authorize(TasksView, [Unassigned, Admin]);
        get TaskEditGetRouteTag, "/tasks/{id}/edit", handlers::tasks::edit_get, modal, authorize(TasksMutate, []);
        post TaskEditPostRouteTag, "/tasks/{id}/edit", handlers::tasks::edit_post, authorize(TasksMutate, []);
        get TaskDeleteGetRouteTag, "/tasks/{id}/delete", handlers::tasks::delete_get, modal, authorize(TasksMutate, []);
        post TaskDeletePostRouteTag, "/tasks/{id}/delete", bare handlers::tasks::delete_post, fragment(TASK_DELETE_MODAL), authorize(TasksMutate, []);

        get PushVapidRouteTag, "/tasks/push/vapid", bare handlers::push::vapid_public_key, raw, authorize(TasksView, [Unassigned, Admin]);
        post PushSubscribeRouteTag, "/tasks/push/subscribe", bare handlers::push::subscribe, raw, authorize(TasksView, [Unassigned, Admin]);
        post PushUnsubscribeRouteTag, "/tasks/push/unsubscribe", bare handlers::push::unsubscribe, raw, authorize(TasksView, [Unassigned, Admin]);
        get TasksPushServiceWorkerRouteTag, "/sw/tasks.js", root bare lariv_plugin_pwa::handlers::service_worker_module, raw;
    ]
}
