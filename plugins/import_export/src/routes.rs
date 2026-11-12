//! Import and export HTTP routes.
use lariv_core::define_plugin_routes;

use super::handlers;

/// Import pages. Empty allowlist: superuser only until another plugin patches it.
pub struct ImportAdmin;

/// Export pages. Empty allowlist: superuser only until another plugin patches it.
pub struct ExportAdmin;

define_plugin_routes! {
    plugin: ImportExportPluginTag;
    prefix: "/dashboard";
    routes: [
        get ImportPageRouteTag, "/import", handlers::import::page, authorize(ImportAdmin, []);
        post ImportPostRouteTag, "/import", handlers::import::import_post, authorize(ImportAdmin, []);
        get ExportPageRouteTag, "/export", handlers::export::page, authorize(ExportAdmin, []);
        get ExportDownloadGetRouteTag, "/export/download", bare handlers::export::download_get, redirect, authorize(ExportAdmin, []);
        post ExportDownloadRouteTag, "/export/download", bare handlers::export::download, file, authorize(ExportAdmin, []);
    ]
}
