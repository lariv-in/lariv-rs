lariv_core::define_register_apps! {
    plugin: FinanceTaxesTag;
    key: "p_finance_taxes";
    name: "Finance taxes";
    href: crate::routes::TaxDefaultRouteTag.url();
    icon: "receipt-percent";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
