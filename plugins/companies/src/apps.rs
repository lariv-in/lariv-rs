use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: CompaniesTag;
    key: "p_companies";
    name: "Companies";
    href: crate::routes::CompanyDefaultRouteTag.url();
    icon: "building-office-2";
    roles: [
        lariv_plugin_users::roles::Unassigned,
        lariv_plugin_users::roles::Admin,
    ];
}
