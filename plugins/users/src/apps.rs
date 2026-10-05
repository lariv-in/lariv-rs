//! Users app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: UsersTag;
    key: "p_users";
    name: "Users";
    href: crate::routes::UsersListRouteTag.url();
    icon: "users";
    roles: [];
}
