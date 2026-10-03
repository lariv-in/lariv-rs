//! Filesystem app catalog tile.

use crate::apps::define_register_apps;

pub const FILESYSTEM_APP_KEY: &str = "p_filesystem";

define_register_apps! {
    plugin: FilesystemTag;
    key: FILESYSTEM_APP_KEY;
    name: "Filesystem";
    href: crate::plugins::filesystem::routes::VNodeListRouteTag.url();
    icon: "folder";
    roles: ["superuser", "admin"];
}
