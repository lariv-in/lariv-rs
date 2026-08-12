crate::define_register_apps! {
    plugin: FinanceTaxesTag;
    key: "p_finance_taxes";
    name: "Finance taxes";
    href: "/finance-taxes/";
    icon: "receipt-percent";
    plugin_type: crate::apps::PluginType::Addon;
    roles: ["superuser"];
}
