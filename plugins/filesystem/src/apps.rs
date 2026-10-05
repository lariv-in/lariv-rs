//! Filesystem app catalog tile.

use lariv_core::apps::define_register_apps;

pub const FILESYSTEM_APP_KEY: &str = "p_filesystem";

define_register_apps! {
    plugin: FilesystemTag;
    key: FILESYSTEM_APP_KEY;
    name: "Filesystem";
    href: crate::routes::VNodeListRouteTag.url();
    icon: "folder";
    roles: [lariv_plugin_users::roles::Admin];
}
