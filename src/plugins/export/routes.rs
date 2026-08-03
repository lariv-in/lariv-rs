//! Export HTTP routes — page and download endpoints.
use lariv_rs::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: ExportPluginTag;
    routes: [
        get ExportPageRouteTag, "/export", handlers::page;
        post ExportDownloadRouteTag, "/export/download", bare handlers::download, file;
    ]
}
