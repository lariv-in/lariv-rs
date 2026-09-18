use crate::apps::define_register_apps;

define_register_apps! {
    plugin: FormsTag;
    key: "p_forms";
    name: "Forms";
    href: "/forms";
    icon: "clipboard-document-list";
    roles: ["superuser"];
}
