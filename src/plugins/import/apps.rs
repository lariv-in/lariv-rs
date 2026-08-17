//! Dashboard app tile registration for the import plugin.

use lariv_rs::define_register_apps;

define_register_apps! {
    plugin: ImportPluginTag;
    key: "p_import";
    name: "Import";
    href: "/import";
    icon: "arrow-up-tray";
    roles: [];
}
