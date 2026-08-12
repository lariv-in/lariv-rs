//! Dashboard HTTP routes — tagged entries on [`crate::http::HttpCapability`]'s route HList.

use crate::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: DashboardTag;
    routes: [
        get DashboardHomeRouteTag, "/", bare handlers::home_redirect, redirect;
        get DashboardAppsRouteTag, "/dashboard", handlers::apps;
    ]
}
