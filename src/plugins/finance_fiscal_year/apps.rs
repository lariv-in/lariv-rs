crate::define_register_apps! {
    plugin: FinanceFiscalYearTag;
    key: "p_finance_fiscal_year";
    name: "Finance fiscal years";
    href: "/finance-fiscal-years/";
    icon: "calendar";
    plugin_type: crate::apps::PluginType::Addon;
    roles: ["superuser"];
}
