lariv_core::define_register_apps! {
    plugin: FinanceCustomerTag;
    key: "p_customer";
    name: "Customers";
    href: lariv_plugin_customer::routes::CustomerDefaultRouteTag.url();
    icon: "building-storefront";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
