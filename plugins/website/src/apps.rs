//! Website app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: WebsiteTag;
    key: "p_website";
    name: "Website";
    href: crate::routes::WebsiteRoutesListRouteTag.url();
    icon: "globe-alt";
    roles: [lariv_plugin_users::roles::Admin];
}
