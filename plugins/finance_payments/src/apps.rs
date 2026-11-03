lariv_core::define_register_apps! {
    plugin: FinancePaymentsTag;
    key: "p_finance_payments";
    name: "Finance payments";
    href: crate::routes::PaymentListRouteTag.url();
    icon: "banknotes";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
