//! Dashboard HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::plugin_routes::define_plugin_routes;

use super::{
    DashboardTag, handlers,
    templates::{AppsPage, DashboardAppsPageTag},
};

define_plugin_routes! {
    plugin: DashboardTag;
    proof: DashboardRoutesProof;
    pages: [
        pane AppsIdx, AppsP => DashboardAppsPageTag, AppsPage;
    ];
    routes: [
        get DashboardAppsRouteTag, "/dashboard", handlers::apps;
    ]
}
