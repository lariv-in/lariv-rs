crate::define_register_apps! {
    plugin: FinanceCustomerTag;
    key: "p_customer";
    name: "Customers";
    href: crate::plugins::customer::routes::CustomerDefaultRouteTag.url();
    icon: "building-storefront";
    plugin_type: crate::apps::PluginType::Addon;
    roles: [];
}
