//! Tasks app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: TasksTag;
    key: "p_tasks";
    name: "Tasks";
    href: crate::routes::TaskDefaultRouteTag.url();
    icon: "clipboard-document-list";
    roles: [];
}
