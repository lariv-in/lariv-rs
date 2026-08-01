//! Dashboard HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::define_plugin_routes;

use super::{
    handlers,
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
