crate::define_register_apps! {
    plugin: FinanceCustomerTag;
    key: "p_customer";
    name: "Customers";
    href: "/customers/";
    icon: "building-storefront";
    plugin_type: crate::apps::PluginType::Addon;
    roles: ["superuser"];
}
