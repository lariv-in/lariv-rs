lariv_core::define_register_apps! {
    plugin: FinanceServicesTag;
    key: "p_finance_services";
    name: "Finance services";
    href: crate::routes::ServiceDefaultRouteTag.url();
    icon: "briefcase";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
