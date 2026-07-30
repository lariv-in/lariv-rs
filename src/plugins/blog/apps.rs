//! Blog app catalog tile (Go `p_blog` `PluginTypeApp` registration).

use crate::apps::define_register_apps;

use super::BlogTag;

define_register_apps! {
    plugin: BlogTag;
    key: "p_blog";
    name: "Blog";
    href: "/blog";
    icon: "newspaper";
    roles: ["superuser", "admin"];
}
