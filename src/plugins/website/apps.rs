//! Website app catalog tile.

use crate::apps::define_register_apps;

define_register_apps! {
    plugin: WebsiteTag;
    key: "p_website";
    name: "Website";
    href: crate::plugins::website::routes::WebsiteRoutesListRouteTag.url();
    icon: "globe-alt";
    roles: [crate::plugins::users::roles::Admin];
}
