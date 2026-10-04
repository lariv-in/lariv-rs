use crate::apps::define_register_apps;

define_register_apps! {
    plugin: ContactsTag;
    key: "p_contacts";
    name: "Contacts";
    href: crate::plugins::contacts::routes::ContactDefaultRouteTag.url();
    icon: "identification";
    roles: [];
}
