use super::handlers;

pub struct GstExportView;

pub struct GstExportMutate;

lariv_core::define_plugin_routes! {
    plugin: crate::FinanceGstExportTag;
    prefix: "/dashboard";
    routes: [
        get GstPreferencesRouteTag, "/finance-gst-export", handlers::preferences::page, authorize(GstExportView, []);
        post GstPreferencesPostRouteTag, "/finance-gst-export", handlers::preferences::save, authorize(GstExportMutate, []);
        get GstWorkbookRouteTag, "/finance-gst-export/invoices.xlsx", bare handlers::workbook::download, file, authorize(GstExportView, []);
    ]
}
