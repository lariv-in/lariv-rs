//! Tasks app catalog tile.

use crate::apps::define_register_apps;

define_register_apps! {
    plugin: TasksTag;
    key: "p_tasks";
    name: "Tasks";
    href: "/tasks";
    icon: "clipboard-document-list";
    roles: ["superuser"];
}
