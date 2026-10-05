use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: ContactsTag;
    key: "p_contacts";
    name: "Contacts";
    href: crate::routes::ContactDefaultRouteTag.url();
    icon: "identification";
    roles: [];
}
