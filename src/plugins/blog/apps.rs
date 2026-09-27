//! Blog app catalog tile.

use crate::apps::define_register_apps;

define_register_apps! {
    plugin: BlogTag;
    key: "p_blog";
    name: "Blog";
    href: crate::plugins::blog::routes::BlogListRouteTag.url();
    icon: "newspaper";
    roles: ["superuser", "admin"];
}
