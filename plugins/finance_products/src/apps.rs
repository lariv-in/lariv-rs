lariv_core::define_register_apps! {
    plugin: FinanceProductsTag;
    key: "p_finance_products";
    name: "Finance products";
    href: crate::routes::ProductDefaultRouteTag.url();
    icon: "cube";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
