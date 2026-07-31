//! Filesystem app catalog tile (Go `p_filesystem` `PluginTypeApp` registration).

use crate::apps::define_register_apps;


define_register_apps! {
    plugin: FilesystemTag;
    key: "p_filesystem";
    name: "Filesystem";
    href: "/filesystem";
    icon: "folder";
    roles: ["superuser", "admin"];
}
