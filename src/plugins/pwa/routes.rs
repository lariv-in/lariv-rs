//! PWA HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: PwaTag;
    slots: clone;
    pages: [];
    routes: [
        get PwaManifestRouteTag, "/app.webmanifest", bare handlers::manifest, raw;
        get PwaServiceWorkerRouteTag, "/serviceworker.js", bare handlers::service_worker, raw;
        get PwaOfflineRouteTag, "/offline", bare handlers::offline, raw;
        get PwaAssetLinksRouteTag, "/.well-known/assetlinks.json", bare handlers::asset_links, raw;
        get PwaStaticRootRouteTag, "/static/pwa", bare handlers::static_pwa_root, raw;
        get PwaStaticFilesRouteTag, "/static/pwa/{*path}", bare handlers::static_pwa_file, raw;
    ]
}
