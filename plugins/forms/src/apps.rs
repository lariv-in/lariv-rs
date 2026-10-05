use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: FormsTag;
    key: "p_forms";
    name: "Forms";
    href: crate::routes::FormListRouteTag.url();
    icon: "clipboard-document-list";
    roles: [];
}
