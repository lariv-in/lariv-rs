//! Import HTTP routes — upload page and workbook POST.
use lariv_rs::define_plugin_routes;

use super::handlers;

/// Import pages. Empty allowlist: superuser only until another plugin patches it.
pub struct ImportAdmin;

define_plugin_routes! {
    plugin: ImportPluginTag;
    prefix: "/dashboard";
    routes: [
        get ImportPageRouteTag, "/import", handlers::page, authorize(ImportAdmin, []);
        post ImportPostRouteTag, "/import", handlers::import_post, authorize(ImportAdmin, []);
    ]
}
