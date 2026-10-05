//! Dashboard HTTP routes — tagged entries on [`lariv_core::http::HttpCapability`]'s route HList.

use lariv_core::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: DashboardTag;
    routes: [
        get DashboardHomeRouteTag, "/", bare handlers::home_redirect, redirect;
        get DashboardAppsRouteTag, "/dashboard", handlers::apps;
    ]
}
