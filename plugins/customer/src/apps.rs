//! Customers app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: CustomerTag;
    key: "p_customer";
    name: "Customers";
    href: crate::routes::CustomerDefaultRouteTag.url();
    icon: "user-circle";
    roles: [
        lariv_plugin_users::roles::Unassigned,
        lariv_plugin_users::roles::Admin,
    ];
}

#[cfg(test)]
mod tests {
    use lariv_core::apps::{AppsCapability, AppsRegistrar};
    use lariv_core::role_registry::SUPERUSER_ROLE_NAME;

    use super::Hook;

    #[test]
    fn customers_tile_shows_on_the_dashboard() {
        let apps = Hook.register_apps(AppsCapability::new());
        let visible = apps.visible_apps(SUPERUSER_ROLE_NAME);
        let tile = visible
            .iter()
            .find(|tile| tile.key == "p_customer")
            .expect("customers tile");
        assert_eq!(tile.verbose_name, "Customers");
        assert!(tile.href.contains("/dashboard/customers"), "{}", tile.href);
        let unassigned = apps.visible_apps("unassigned");
        assert!(
            unassigned.iter().any(|tile| tile.key == "p_customer"),
            "{unassigned:?}"
        );
    }
}
