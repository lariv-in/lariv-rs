//! Dashboard app tile registration for the import plugin.

use lariv_core::define_register_apps;

define_register_apps! {
    plugin: ImportPluginTag;
    key: "p_import";
    name: "Import";
    href: crate::routes::ImportPageRouteTag.url();
    icon: "arrow-up-tray";
    roles: [];
}
