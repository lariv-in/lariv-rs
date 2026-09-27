crate::define_register_apps! {
    plugin: FinanceProductsTag;
    key: "p_finance_products";
    name: "Finance products";
    href: crate::plugins::finance_products::routes::ProductDefaultRouteTag.url();
    icon: "cube";
    plugin_type: crate::apps::PluginType::Addon;
    roles: ["superuser"];
}
