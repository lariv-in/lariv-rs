use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: MeetsTag;
    key: "p_meets";
    name: "Meets";
    href: crate::routes::HubRouteTag.url();
    icon: "video-camera";
    roles: [];
}
