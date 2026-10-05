//! Blog app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: BlogTag;
    key: "p_blog";
    name: "Blog";
    href: crate::routes::BlogListRouteTag.url();
    icon: "newspaper";
    roles: [lariv_plugin_users::roles::Admin];
}
