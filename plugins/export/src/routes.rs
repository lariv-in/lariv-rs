//! Export HTTP routes — page and download endpoints.
use lariv_core::define_plugin_routes;

use super::handlers;

/// Export pages. Empty allowlist: superuser only until another plugin patches it.
pub struct ExportAdmin;

define_plugin_routes! {
    plugin: ExportPluginTag;
    prefix: "/dashboard";
    routes: [
        get ExportPageRouteTag, "/export", handlers::page, authorize(ExportAdmin, []);
        get ExportDownloadGetRouteTag, "/export/download", bare handlers::download_get, redirect, authorize(ExportAdmin, []);
        post ExportDownloadRouteTag, "/export/download", bare handlers::download, file, authorize(ExportAdmin, []);
    ]
}
