//! Dashboard app tile for import and export.

use lariv_core::define_register_apps;

define_register_apps! {
    plugin: ImportExportPluginTag;
    key: "p_import_export";
    name: "Import / Export";
    href: crate::routes::ImportPageRouteTag.url();
    icon: "arrows-right-left";
    roles: [];
}
