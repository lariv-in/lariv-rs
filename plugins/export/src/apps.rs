//! Dashboard app tile registration for the export plugin.

use lariv_core::define_register_apps;

define_register_apps! {
    plugin: ExportPluginTag;
    key: "p_export";
    name: "Export";
    href: crate::routes::ExportPageRouteTag.url();
    icon: "arrow-down-tray";
    roles: [];
}
