lariv_core::define_register_apps! {
    plugin: super::FinanceQuotationsTag;
    key: "p_finance_quotations";
    name: "Finance quotations";
    href: crate::routes::QuotationDefaultRouteTag.url();
    icon: "document-text";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
