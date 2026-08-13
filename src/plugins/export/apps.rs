//! Dashboard app tile registration for the export plugin.

use lariv_rs::define_register_apps;

define_register_apps! {
    plugin: ExportPluginTag;
    key: "p_export";
    name: "Export";
    href: "/export";
    icon: "arrow-down-tray";
    roles: [];
}
