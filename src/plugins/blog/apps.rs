//! Blog app catalog tile.

use crate::apps::define_register_apps;


define_register_apps! {
    plugin: BlogTag;
    key: "p_blog";
    name: "Blog";
    href: "/blog";
    icon: "newspaper";
    roles: ["superuser", "admin"];
}
