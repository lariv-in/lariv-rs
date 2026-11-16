//! Audit log app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: LoggingTag;
    key: "p_logging";
    name: "Logs";
    href: crate::routes::LogListRouteTag.url();
    icon: "lucide:scroll-text";
    roles: [lariv_plugin_users::roles::Admin];
}
