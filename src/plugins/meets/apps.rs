use crate::apps::define_register_apps;

define_register_apps! {
    plugin: MeetsTag;
    key: "p_meets";
    name: "Meets";
    href: crate::plugins::meets::routes::HubRouteTag.url();
    icon: "video-camera";
    roles: [];
}
