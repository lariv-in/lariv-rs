lariv_core::define_register_apps! {
    plugin: crate::FinanceGstExportTag;
    key: "p_finance_gst_export";
    name: "GST export";
    href: crate::routes::GstPreferencesRouteTag.url();
    icon: "document-arrow-down";
    roles: [];
}
