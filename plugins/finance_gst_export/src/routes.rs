use super::handlers;

/// GST component preferences. Empty allowlist: superuser only.
pub struct GstExportView;

/// Saving GST component preferences. Empty allowlist: superuser only.
pub struct GstExportMutate;

/// GST Excel page and workbook download. Empty allowlist until a role plugin patches it.
pub struct GstExcelView;

lariv_core::define_plugin_routes! {
    plugin: crate::FinanceGstExportTag;
    prefix: "/dashboard";
    routes: [
        get GstPreferencesRouteTag, "/finance-gst-export", handlers::preferences::page, authorize(GstExportView, []);
        post GstPreferencesPostRouteTag, "/finance-gst-export", handlers::preferences::save, authorize(GstExportMutate, []);
        get GstExcelRouteTag, "/finance-gst-export/excel", handlers::excel::page, authorize(GstExcelView, []);
        get GstWorkbookRouteTag, "/finance-gst-export/invoices.xlsx", bare handlers::workbook::download, file, authorize(GstExcelView, []);
    ]
}
