use lariv_rs::define_plugin_routes;

use super::{
    ExportPluginTag, handlers,
    templates::{ExportPage, ExportPageTag},
};

define_plugin_routes! {
    plugin: ExportPluginTag;
    proof: ExportRoutesProof;
    pages: [
        pane ExportPageIdx, ExportPageP => ExportPageTag, ExportPage;
    ];
    routes: [
        get ExportPageRouteTag, "/export", handlers::page;
        post ExportDownloadRouteTag, "/export/download", bare handlers::download;
    ]
}
