use crate::apps::define_register_apps;

define_register_apps! {
    plugin: MeetsTag;
    key: "p_meets";
    name: "Meets";
    href: "/meets";
    icon: "video-camera";
    roles: ["superuser"];
}
