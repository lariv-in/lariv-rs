//! Website app catalog tile.

use crate::apps::define_register_apps;


define_register_apps! {
    plugin: WebsiteTag;
    key: "p_website";
    name: "Website";
    href: "/website";
    icon: "globe-alt";
    roles: ["superuser", "admin"];
}
