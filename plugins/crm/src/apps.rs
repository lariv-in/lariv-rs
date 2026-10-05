use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: CrmTag;
    key: "p_crm";
    name: "CRM";
    href: crate::routes::LeadDefaultRouteTag.url();
    icon: "building-office";
    roles: [];
}
