//! Documents app catalog tile.

use crate::apps::define_register_apps;

define_register_apps! {
    plugin: DocumentsTag;
    key: "p_documents";
    name: "Documents";
    href: crate::plugins::documents::routes::DocumentDefaultRouteTag.url();
    icon: "identification";
    roles: [];
}
