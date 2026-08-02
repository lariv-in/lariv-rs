//! Dashboard HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: DashboardTag;
    routes: [
        get DashboardAppsRouteTag, "/dashboard", handlers::apps;
    ]
}
