use super::{
    handlers,
    keys::{
        TaskDeleteModalKey, TaskLogDeleteModalKey, TaskLogsKey, TaskStatusDeleteModalKey,
        TaskStatusTableKey, TaskStatusTasksTableKey, TaskTableKey,
    },
};

crate::define_plugin_routes! {
    plugin: TasksTag;
    routes: [
        get TaskDefaultRouteTag, "/tasks", handlers::tasks::hub, fragment(TaskTableKey);
        get TaskCreateGetRouteTag, "/tasks/create", handlers::tasks::create_get, modal;
        post TaskCreatePostRouteTag, "/tasks/create", handlers::tasks::create_post;

        get TaskStatusDefaultRouteTag, "/tasks/statuses", handlers::statuses::list, fragment(TaskStatusTableKey);
        get TaskStatusCreateGetRouteTag, "/tasks/statuses/create", handlers::statuses::create_get, modal;
        post TaskStatusCreatePostRouteTag, "/tasks/statuses/create", handlers::statuses::create_post;
        get TaskStatusDetailRouteTag, "/tasks/statuses/{id}", handlers::statuses::detail, fragment(TaskStatusTasksTableKey);
        get TaskStatusEditGetRouteTag, "/tasks/statuses/{id}/edit", handlers::statuses::edit_get, modal;
        post TaskStatusEditPostRouteTag, "/tasks/statuses/{id}/edit", handlers::statuses::edit_post;
        get TaskStatusDeleteGetRouteTag, "/tasks/statuses/{id}/delete", handlers::statuses::delete_get, modal;
        post TaskStatusDeletePostRouteTag, "/tasks/statuses/{id}/delete", bare handlers::statuses::delete_post, fragment(TaskStatusDeleteModalKey);

        post TaskLogAddPostRouteTag, "/tasks/{task_id}/logs", handlers::logs::add_post, param task_id: i64, fragment(TaskLogsKey);
        get TaskLogDetailRouteTag, "/tasks/logs/{id}", handlers::logs::detail;
        get TaskLogEditGetRouteTag, "/tasks/logs/{id}/edit", handlers::logs::edit_get, modal;
        post TaskLogEditPostRouteTag, "/tasks/logs/{id}/edit", handlers::logs::edit_post;
        get TaskLogDeleteGetRouteTag, "/tasks/logs/{id}/delete", handlers::logs::delete_get, modal;
        post TaskLogDeletePostRouteTag, "/tasks/logs/{id}/delete", bare handlers::logs::delete_post, fragment(TaskLogDeleteModalKey);

        get TaskDetailRouteTag, "/tasks/{id}", handlers::tasks::detail, fragment(TaskLogsKey);
        get TaskEditGetRouteTag, "/tasks/{id}/edit", handlers::tasks::edit_get, modal;
        post TaskEditPostRouteTag, "/tasks/{id}/edit", handlers::tasks::edit_post;
        get TaskDeleteGetRouteTag, "/tasks/{id}/delete", handlers::tasks::delete_get, modal;
        post TaskDeletePostRouteTag, "/tasks/{id}/delete", bare handlers::tasks::delete_post, fragment(TaskDeleteModalKey);
    ]
}
