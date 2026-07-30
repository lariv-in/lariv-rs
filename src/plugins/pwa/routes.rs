//! PWA HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::plugin_routes::define_plugin_routes;

use super::{PwaTag, handlers};

define_plugin_routes! {
    plugin: PwaTag;
    slots: clone;
    pages: [];
    routes: [
        get PwaManifestRouteTag, "/app.webmanifest", bare handlers::manifest;
        get PwaServiceWorkerRouteTag, "/serviceworker.js", bare handlers::service_worker;
        get PwaOfflineRouteTag, "/offline", bare handlers::offline;
        get PwaAssetLinksRouteTag, "/.well-known/assetlinks.json", bare handlers::asset_links;
        get PwaStaticRootRouteTag, "/static/pwa", bare handlers::static_pwa_root;
        get PwaStaticFilesRouteTag, "/static/pwa/{*path}", bare handlers::static_pwa_file;
    ]
}
