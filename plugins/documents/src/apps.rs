//! Documents app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: DocumentsTag;
    key: "p_documents";
    name: "Documents";
    href: crate::routes::DocumentDefaultRouteTag.url();
    icon: "identification";
    roles: [];
}
