//! ToDo app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: TodoTag;
    key: "p_todo";
    name: "ToDo";
    href: crate::routes::TodoListRouteTag.url();
    icon: "check-circle";
    roles: [
        lariv_plugin_users::roles::Unassigned,
        lariv_plugin_users::roles::Admin,
    ];
}
