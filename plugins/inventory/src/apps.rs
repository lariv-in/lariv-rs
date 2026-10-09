//! Inventory app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: InventoryTag;
    key: "p_inventory";
    name: "Inventory";
    href: crate::routes::StockDefaultRouteTag.url();
    icon: "cube";
    roles: [
        lariv_plugin_users::roles::Unassigned,
        lariv_plugin_users::roles::Admin,
    ];
}
