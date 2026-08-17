//! Export HTTP routes — page and download endpoints.
use lariv_rs::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: ExportPluginTag;
    routes: [
        get ExportPageRouteTag, "/export", handlers::page;
        get ExportDownloadGetRouteTag, "/export/download", bare handlers::download_get, redirect;
        post ExportDownloadRouteTag, "/export/download", bare handlers::download, file;
    ]
}
