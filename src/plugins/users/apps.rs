//! Users app catalog tile.

use crate::apps::define_register_apps;

define_register_apps! {
    plugin: UsersTag;
    key: "p_users";
    name: "Users";
    href: "/users";
    icon: "users";
    roles: [];
}
