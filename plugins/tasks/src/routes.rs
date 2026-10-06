use lariv_plugin_users::roles::{Admin, Unassigned};

use super::{
    handlers,
    keys::{
        TaskDeleteModalKey, TaskLogDeleteModalKey, TaskLogsKey, TaskStatusDeleteModalKey,
        TaskStatusTableKey, TaskStatusTasksTableKey, TaskTableKey,
    },
};

pub struct TasksView;

pub struct TasksMutate;

lariv_core::define_plugin_routes! {
    plugin: TasksTag;
    prefix: "/dashboard";
    routes: [
        get TaskDefaultRouteTag, "/tasks", handlers::tasks::hub, fragment(TaskTableKey), authorize(TasksView, [Unassigned, Admin]);
        get TaskCreateGetRouteTag, "/tasks/create", handlers::tasks::create_get, modal, authorize(TasksMutate, []);
        post TaskCreatePostRouteTag, "/tasks/create", handlers::tasks::create_post, authorize(TasksMutate, []);

        get TaskStatusDefaultRouteTag, "/tasks/statuses", handlers::statuses::list, fragment(TaskStatusTableKey), authorize(TasksView, [Unassigned, Admin]);
        get TaskStatusCreateGetRouteTag, "/tasks/statuses/create", handlers::statuses::create_get, modal, authorize(TasksMutate, []);
        post TaskStatusCreatePostRouteTag, "/tasks/statuses/create", handlers::statuses::create_post, authorize(TasksMutate, []);
        get TaskStatusDetailRouteTag, "/tasks/statuses/{id}", handlers::statuses::detail, fragment(TaskStatusTasksTableKey), authorize(TasksView, [Unassigned, Admin]);
        get TaskStatusEditGetRouteTag, "/tasks/statuses/{id}/edit", handlers::statuses::edit_get, modal, authorize(TasksMutate, []);
        post TaskStatusEditPostRouteTag, "/tasks/statuses/{id}/edit", handlers::statuses::edit_post, authorize(TasksMutate, []);
        get TaskStatusDeleteGetRouteTag, "/tasks/statuses/{id}/delete", handlers::statuses::delete_get, modal, authorize(TasksMutate, []);
        post TaskStatusDeletePostRouteTag, "/tasks/statuses/{id}/delete", bare handlers::statuses::delete_post, fragment(TaskStatusDeleteModalKey), authorize(TasksMutate, []);

        get TaskLogsRouteTag, "/tasks/{id}/logs", handlers::logs::list, fragment(TaskLogsKey), authorize(TasksView, [Unassigned, Admin]);
        post TaskLogAddPostRouteTag, "/tasks/{id}/logs", handlers::logs::add_post, fragment(TaskLogsKey), authorize(TasksMutate, []);
        get TaskLogDetailRouteTag, "/tasks/logs/{id}", handlers::logs::detail, authorize(TasksView, [Unassigned, Admin]);
        get TaskLogEditGetRouteTag, "/tasks/logs/{id}/edit", handlers::logs::edit_get, modal, authorize(TasksMutate, []);
        post TaskLogEditPostRouteTag, "/tasks/logs/{id}/edit", handlers::logs::edit_post, authorize(TasksMutate, []);
        get TaskLogDeleteGetRouteTag, "/tasks/logs/{id}/delete", handlers::logs::delete_get, modal, authorize(TasksMutate, []);
        post TaskLogDeletePostRouteTag, "/tasks/logs/{id}/delete", bare handlers::logs::delete_post, fragment(TaskLogDeleteModalKey), authorize(TasksMutate, []);

        post TaskSetStatusRouteTag, "/tasks/{id}/status/{slug}", handlers::tasks::set_status, authorize(TasksView, [Unassigned, Admin]);
        post TaskSetStatusListRouteTag, "/tasks/{id}/status/{slug}/list", handlers::tasks::set_status, fragment(TaskTableKey), authorize(TasksView, [Unassigned, Admin]);

        get TaskDetailRouteTag, "/tasks/{id}", handlers::tasks::detail, authorize(TasksView, [Unassigned, Admin]);
        get TaskEditGetRouteTag, "/tasks/{id}/edit", handlers::tasks::edit_get, modal, authorize(TasksMutate, []);
        post TaskEditPostRouteTag, "/tasks/{id}/edit", handlers::tasks::edit_post, authorize(TasksMutate, []);
        get TaskDeleteGetRouteTag, "/tasks/{id}/delete", handlers::tasks::delete_get, modal, authorize(TasksMutate, []);
        post TaskDeletePostRouteTag, "/tasks/{id}/delete", bare handlers::tasks::delete_post, fragment(TaskDeleteModalKey), authorize(TasksMutate, []);
    ]
}
