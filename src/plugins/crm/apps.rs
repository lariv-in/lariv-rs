use crate::apps::define_register_apps;

define_register_apps! {
    plugin: CrmTag;
    key: "p_crm";
    name: "CRM";
    href: "/crm/leads";
    icon: "building-office";
    roles: ["superuser"];
}
