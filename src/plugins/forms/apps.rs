use crate::apps::define_register_apps;

define_register_apps! {
    plugin: FormsTag;
    key: "p_forms";
    name: "Forms";
    href: crate::plugins::forms::routes::FormListRouteTag.url();
    icon: "clipboard-document-list";
    roles: [];
}
