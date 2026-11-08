//! mdBook app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: MdBookTag;
    key: "p_mdbook";
    name: "mdBook";
    href: crate::routes::MdBookPrefsGetRouteTag.url();
    icon: "book-open";
    roles: [lariv_plugin_users::roles::Admin];
}
