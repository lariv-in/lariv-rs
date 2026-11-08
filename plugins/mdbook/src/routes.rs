//! mdBook dashboard routes.

use lariv_core::define_plugin_routes;

use super::handlers;

/// Who may edit mdBook preferences. Empty extra roles: superuser is always allowed.
pub struct MdBookPrefsAdmin;

define_plugin_routes! {
    plugin: MdBookTag;
    prefix: "/dashboard";
    routes: [
        get MdBookPrefsGetRouteTag, "/mdbook/preferences", handlers::preferences::get, authorize(MdBookPrefsAdmin, [lariv_plugin_users::roles::Admin]);
        post MdBookPrefsPostRouteTag, "/mdbook/preferences", handlers::preferences::post, authorize(MdBookPrefsAdmin, [lariv_plugin_users::roles::Admin]);
        post MdBookBuildPostRouteTag, "/mdbook/build", handlers::preferences::build, authorize(MdBookPrefsAdmin, [lariv_plugin_users::roles::Admin]);
    ]
}
