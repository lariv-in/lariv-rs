//! Import HTTP routes — upload page and workbook POST.
use lariv_rs::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: ImportPluginTag;
    routes: [
        get ImportPageRouteTag, "/import", handlers::page;
        post ImportPostRouteTag, "/import", handlers::import_post;
    ]
}
